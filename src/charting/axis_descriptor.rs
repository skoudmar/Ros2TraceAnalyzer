use derive_more::Display;
use itertools::Itertools;
use strum::{EnumIter, IntoEnumIterator};

use crate::argsv2::chart_args::{ChartVariants, ChartedValue};

/// # Axis Descriptors
///
/// Structure describing `x` and `y` axis formatting for a chart
pub struct AxisDescriptors {
    /// The formatting for the `x` axis
    pub x: AxisDescriptor,
    /// The formatting for the `y` axis
    pub y: AxisDescriptor,
}

/// # Axis Descriptor
///
/// The formatting to use when displaying axis label and ticks
#[derive(Copy, Clone)]
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
                AxisQuantity::SimpleSi { base } => AxisQuantity::new_si(
                    SiPrefix::iter()
                        .rev()
                        .map(|unit| (unit, unit.express_value(fit_to_value, base)))
                        .find_or_last(|&(_, value)| (0.0..1000.0).contains(&value))
                        .unwrap()
                        .0,
                ),
            },
        }
    }
}

pub struct ScaledAxisDescriptor {
    pub default_axis: AxisDescriptor,
    pub target: AxisQuantity,
}

impl ScaledAxisDescriptor {
    pub fn name(&self) -> String {
        match self.target {
            AxisQuantity::Duration { base } => format!("{} [{}]", self.default_axis.label, base),
            AxisQuantity::SimpleSi { base } => {
                if base == SiPrefix::Base {
                    self.default_axis.label.to_string()
                } else {
                    format!("{} [{}]", self.default_axis.label, base)
                }
            }
        }
    }

    pub fn convert(&self, value: i64) -> f64 {
        match (self.default_axis.quantity, self.target) {
            (AxisQuantity::Duration { base: source }, AxisQuantity::Duration { base: target }) => {
                target.express_value(value, source)
            }
            (AxisQuantity::SimpleSi { base: source }, AxisQuantity::SimpleSi { base: target }) => {
                target.express_value(value, source)
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Copy, Display, Clone, PartialEq, PartialOrd, EnumIter)]
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
            SiPrefix::Mega => 1e-6,
            SiPrefix::Kilo => 1e-3,
            SiPrefix::Base => 1e-0,
            SiPrefix::Milli => 1e+3,
            SiPrefix::Micro => 1e6,
            SiPrefix::Nano => 1e+9,
        }
    }

    fn express_value(&self, value: i64, base: SiPrefix) -> f64 {
        value as f64 * (self.ratio() / base.ratio())
    }
}

#[derive(Debug, Copy, Display, Clone, PartialEq, PartialOrd, EnumIter)]
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
    const fn ratio(&self) -> f64 {
        match self {
            DurationUnit::Hour => 1. / 3600.,
            DurationUnit::Minute => 1. / 60.,
            DurationUnit::Second => 1e-0,
            DurationUnit::Millisecond => 1e+3,
            DurationUnit::Microsecond => 1e6,
            DurationUnit::Nanosecond => 1e+9,
        }
    }

    fn express_value(&self, value: i64, base: DurationUnit) -> f64 {
        value as f64 * (self.ratio() / base.ratio())
    }
}

#[derive(Clone, Copy)]
pub enum AxisQuantity {
    Duration { base: DurationUnit },
    SimpleSi { base: SiPrefix },
}

impl AxisQuantity {
    pub const fn new_duration(unit: DurationUnit) -> Self {
        AxisQuantity::Duration { base: unit }
    }

    pub const fn new_si(unit: SiPrefix) -> Self {
        AxisQuantity::SimpleSi { base: unit }
    }
}

pub const fn resolve_axis_descriptors(
    charted_value: &ChartedValue,
    chart_variant: &ChartVariants,
) -> AxisDescriptors {
    match chart_variant {
        ChartVariants::Histogram(_) => match charted_value {
            ChartedValue::CallbackDuration => AxisDescriptors {
                x: AxisDescriptor {
                    label: "Duration",
                    quantity: AxisQuantity::new_duration(DurationUnit::Nanosecond),
                },
                y: AxisDescriptor {
                    label: "Samples",
                    quantity: AxisQuantity::new_si(SiPrefix::Base),
                },
            },
            ChartedValue::ActivationsDelay => AxisDescriptors {
                x: AxisDescriptor {
                    label: "Delay",
                    quantity: AxisQuantity::new_duration(DurationUnit::Nanosecond),
                },
                y: AxisDescriptor {
                    label: "Activations",
                    quantity: AxisQuantity::new_si(SiPrefix::Base),
                },
            },
            ChartedValue::PublicationsDelay => AxisDescriptors {
                x: AxisDescriptor {
                    label: "Delay",
                    quantity: AxisQuantity::new_duration(DurationUnit::Nanosecond),
                },
                y: AxisDescriptor {
                    label: "Publications",
                    quantity: AxisQuantity::new_si(SiPrefix::Base),
                },
            },
            ChartedValue::MessagesDelay => AxisDescriptors {
                x: AxisDescriptor {
                    label: "Delay",
                    quantity: AxisQuantity::new_duration(DurationUnit::Nanosecond),
                },
                y: AxisDescriptor {
                    label: "Messages",
                    quantity: AxisQuantity::new_si(SiPrefix::Base),
                },
            },
            ChartedValue::MessagesLatency => AxisDescriptors {
                x: AxisDescriptor {
                    label: "Latency",
                    quantity: AxisQuantity::new_duration(DurationUnit::Nanosecond),
                },
                y: AxisDescriptor {
                    label: "Samples",
                    quantity: AxisQuantity::new_si(SiPrefix::Base),
                },
            },
        },
        ChartVariants::Scatter => match charted_value {
            ChartedValue::CallbackDuration => AxisDescriptors {
                x: AxisDescriptor {
                    label: "Nth Sample",
                    quantity: AxisQuantity::new_si(SiPrefix::Base),
                },
                y: AxisDescriptor {
                    label: "Duration",
                    quantity: AxisQuantity::new_duration(DurationUnit::Nanosecond),
                },
            },
            ChartedValue::ActivationsDelay => AxisDescriptors {
                x: AxisDescriptor {
                    label: "Nth Activation",
                    quantity: AxisQuantity::new_si(SiPrefix::Base),
                },
                y: AxisDescriptor {
                    label: "Delay",
                    quantity: AxisQuantity::new_duration(DurationUnit::Nanosecond),
                },
            },
            ChartedValue::PublicationsDelay => AxisDescriptors {
                x: AxisDescriptor {
                    label: "Nth Publication",
                    quantity: AxisQuantity::new_si(SiPrefix::Base),
                },
                y: AxisDescriptor {
                    label: "Delay",
                    quantity: AxisQuantity::new_duration(DurationUnit::Nanosecond),
                },
            },
            ChartedValue::MessagesDelay => AxisDescriptors {
                x: AxisDescriptor {
                    label: "Nth Message",
                    quantity: AxisQuantity::new_si(SiPrefix::Base),
                },
                y: AxisDescriptor {
                    label: "Delay",
                    quantity: AxisQuantity::new_duration(DurationUnit::Nanosecond),
                },
            },
            ChartedValue::MessagesLatency => AxisDescriptors {
                x: AxisDescriptor {
                    label: "Nth sample",
                    quantity: AxisQuantity::new_si(SiPrefix::Base),
                },
                y: AxisDescriptor {
                    label: "Latency",
                    quantity: AxisQuantity::new_duration(DurationUnit::Nanosecond),
                },
            },
        },
    }
}
