use derive_more::Display;
use itertools::Itertools;
use strum::{EnumIter, IntoEnumIterator};

use crate::argsv2::plot_args::{PlotVariants, PlottedValue};

/// # Axis Descriptors
///
/// Structure describing `x` and `y` axis formatting for a plot
#[derive(Debug)]
pub struct AxisDescriptors {
    /// The formatting for the `x` axis
    pub x: AxisDescriptor,
    /// The formatting for the `y` axis
    pub y: AxisDescriptor,
}

/// # Axis Descriptor
///
/// The formatting to use when displaying axis label and ticks
#[derive(Copy, Clone, Debug)]
pub struct AxisDescriptor {
    /// The main name of the axis
    pub label: &'static str,
    /// The quantity of the axis
    ///
    /// This value also contains the magnitude the values are expected to be in
    pub quantity: AxisQuantity,
}

impl AxisDescriptor {
    // Returns the original axis descriptor (self) together with a reasonable scaling factor
    pub fn scaled_axis_unit(&self, fit_to_value: i64) -> ScaledAxisDescriptor {
        ScaledAxisDescriptor {
            default_axis: *self,
            target: match self.quantity {
                AxisQuantity::Duration { base } => AxisQuantity::new_duration(
                    DurationUnit::iter()
                        .rev()
                        .map(|unit| (unit, unit.express_value(fit_to_value, base)))
                        .find_or_last(|&(_, value)| (0.0..1000.0).contains(&value))
                        .unwrap()
                        .0,
                ),
                AxisQuantity::SimpleSi {
                    base,
                    show_exponent,
                } => AxisQuantity::new_si(
                    SiPrefix::iter()
                        .rev()
                        .map(|unit| (unit, unit.express_value(fit_to_value, base)))
                        .find_or_last(|&(_, value)| (0.0..1000.0).contains(&value))
                        .unwrap()
                        .0,
                    show_exponent,
                ),
            },
        }
    }
}

#[derive(Debug)]
pub struct ScaledAxisDescriptor {
    pub default_axis: AxisDescriptor,
    pub target: AxisQuantity,
}

impl ScaledAxisDescriptor {
    pub fn name(&self) -> String {
        match self.target {
            AxisQuantity::Duration { base } => format!("{} [{}]", self.default_axis.label, base),
            AxisQuantity::SimpleSi {
                base,
                show_exponent,
            } => {
                if base == SiPrefix::Base {
                    self.default_axis.label.to_string()
                } else {
                    if show_exponent {
                        format!("{} ×{}", self.default_axis.label, base.exponent())
                    } else {
                        format!("{} [{}]", self.default_axis.label, base.to_string())
                    }
                }
            }
        }
    }

    pub fn convert(&self, value: i64) -> f64 {
        match (self.default_axis.quantity, self.target) {
            (AxisQuantity::Duration { base: source }, AxisQuantity::Duration { base: target }) => {
                target.express_value(value, source)
            }
            (
                AxisQuantity::SimpleSi { base: source, .. },
                AxisQuantity::SimpleSi { base: target, .. },
            ) => target.express_value(value, source),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumIter)]
pub enum SiPrefix {
    #[display("M")]
    Mega,
    #[display("k")]
    Kilo,
    #[display("")]
    Base,
    #[display("m")]
    Milli,
    #[display("μ")]
    Micro,
    #[display("n")]
    Nano,
}

impl SiPrefix {
    const fn ratio(self) -> f64 {
        match self {
            Self::Mega => 1e-6,
            Self::Kilo => 1e-3,
            Self::Base => 1e-0,
            Self::Milli => 1e+3,
            Self::Micro => 1e6,
            Self::Nano => 1e+9,
        }
    }

    fn express_value(self, value: i64, base: SiPrefix) -> f64 {
        value as f64 * (self.ratio() / base.ratio())
    }

    fn exponent(self) -> &'static str {
        match self {
            SiPrefix::Mega => "10⁶",
            SiPrefix::Kilo => "10³",
            SiPrefix::Base => "",
            SiPrefix::Milli => "10⁻³",
            SiPrefix::Micro => "10⁻⁶",
            SiPrefix::Nano => "10⁻⁹",
        }
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumIter)]
pub enum DurationUnit {
    #[display("h")]
    Hour,
    #[display("m")]
    Minute,
    #[display("s")]
    Second,
    #[display("ms")]
    Millisecond,
    #[display("μs")]
    Microsecond,
    #[display("ns")]
    Nanosecond,
}

impl DurationUnit {
    const fn ratio(self) -> f64 {
        match self {
            Self::Hour => 1. / 3600.,
            Self::Minute => 1. / 60.,
            Self::Second => 1e-0,
            Self::Millisecond => 1e+3,
            Self::Microsecond => 1e6,
            Self::Nanosecond => 1e+9,
        }
    }

    fn express_value(self, value: i64, base: DurationUnit) -> f64 {
        value as f64 * (self.ratio() / base.ratio())
    }
}

#[derive(Clone, Copy, Debug)]
pub enum AxisQuantity {
    Duration { base: DurationUnit },
    SimpleSi { base: SiPrefix, show_exponent: bool },
}

impl AxisQuantity {
    pub const fn new_duration(unit: DurationUnit) -> Self {
        Self::Duration { base: unit }
    }

    pub const fn new_si(unit: SiPrefix, show_exponent: bool) -> Self {
        Self::SimpleSi {
            base: unit,
            show_exponent,
        }
    }
}

pub const fn resolve_axis_descriptors(
    plotted_value: PlottedValue,
    plot_variant: &PlotVariants,
) -> AxisDescriptors {
    match plot_variant {
        PlotVariants::Histogram(_) => match plotted_value {
            PlottedValue::CallbackDuration => AxisDescriptors {
                x: AxisDescriptor {
                    label: "Duration",
                    quantity: AxisQuantity::new_duration(DurationUnit::Nanosecond),
                },
                y: AxisDescriptor {
                    label: "Callbacks",
                    quantity: AxisQuantity::new_si(SiPrefix::Base, true),
                },
            },
            PlottedValue::ActivationDelay => AxisDescriptors {
                x: AxisDescriptor {
                    label: "Delay",
                    quantity: AxisQuantity::new_duration(DurationUnit::Nanosecond),
                },
                y: AxisDescriptor {
                    label: "Activations",
                    quantity: AxisQuantity::new_si(SiPrefix::Base, true),
                },
            },
            PlottedValue::PublicationDelay => AxisDescriptors {
                x: AxisDescriptor {
                    label: "Delay",
                    quantity: AxisQuantity::new_duration(DurationUnit::Nanosecond),
                },
                y: AxisDescriptor {
                    label: "Publications",
                    quantity: AxisQuantity::new_si(SiPrefix::Base, true),
                },
            },
            PlottedValue::MessageDelay => AxisDescriptors {
                x: AxisDescriptor {
                    label: "Delay",
                    quantity: AxisQuantity::new_duration(DurationUnit::Nanosecond),
                },
                y: AxisDescriptor {
                    label: "Messages",
                    quantity: AxisQuantity::new_si(SiPrefix::Base, true),
                },
            },
            PlottedValue::MessageLatency => AxisDescriptors {
                x: AxisDescriptor {
                    label: "Latency",
                    quantity: AxisQuantity::new_duration(DurationUnit::Nanosecond),
                },
                y: AxisDescriptor {
                    label: "Message",
                    quantity: AxisQuantity::new_si(SiPrefix::Base, true),
                },
            },
        },
        PlotVariants::Scatter => match plotted_value {
            PlottedValue::CallbackDuration => AxisDescriptors {
                x: AxisDescriptor {
                    label: "Callback #",
                    quantity: AxisQuantity::new_si(SiPrefix::Base, true),
                },
                y: AxisDescriptor {
                    label: "Duration",
                    quantity: AxisQuantity::new_duration(DurationUnit::Nanosecond),
                },
            },
            PlottedValue::ActivationDelay => AxisDescriptors {
                x: AxisDescriptor {
                    label: "Activation #",
                    quantity: AxisQuantity::new_si(SiPrefix::Base, true),
                },
                y: AxisDescriptor {
                    label: "Delay",
                    quantity: AxisQuantity::new_duration(DurationUnit::Nanosecond),
                },
            },
            PlottedValue::PublicationDelay => AxisDescriptors {
                x: AxisDescriptor {
                    label: "Publication #",
                    quantity: AxisQuantity::new_si(SiPrefix::Base, true),
                },
                y: AxisDescriptor {
                    label: "Delay",
                    quantity: AxisQuantity::new_duration(DurationUnit::Nanosecond),
                },
            },
            PlottedValue::MessageDelay => AxisDescriptors {
                x: AxisDescriptor {
                    label: "Message #",
                    quantity: AxisQuantity::new_si(SiPrefix::Base, true),
                },
                y: AxisDescriptor {
                    label: "Delay",
                    quantity: AxisQuantity::new_duration(DurationUnit::Nanosecond),
                },
            },
            PlottedValue::MessageLatency => AxisDescriptors {
                x: AxisDescriptor {
                    label: "Message #",
                    quantity: AxisQuantity::new_si(SiPrefix::Base, true),
                },
                y: AxisDescriptor {
                    label: "Latency",
                    quantity: AxisQuantity::new_duration(DurationUnit::Nanosecond),
                },
            },
        },
    }
}
