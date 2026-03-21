use derive_more::Display;
use itertools::Itertools;

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
/// The formatting to use when isplay an axis and labels on it
pub struct AxisDescriptor {
    /// The main name of the axis
    pub label: &'static str,
    /// The quantity of the axis
    ///
    /// This value also contains the magnitude the values are exprected to be in
    pub quantity: AxisQuantity,
}

impl AxisDescriptor {
    pub fn best_fit(&self, value: i64) -> AxisBestFit {
        match &self.quantity {
            AxisQuantity::Duration { base } => AxisBestFit::Duration {
                base: *base,
                target: *DurationUnit::all()
                    .iter()
                    .map(|d| (d, d.express_value(value, *base)))
                    .find_or_last(|&(_, d)| d > 0.1 && d <= 10.)
                    .unwrap()
                    .0,
            },
            AxisQuantity::SimpleSi { base, .. } => AxisBestFit::SimpleSi {
                base: *base,
                target: *SiPrefix::all()
                    .iter()
                    .map(|d| (d, d.express_value(value, *base)))
                    .inspect(|v| println!("{}: {}", v.0, v.1))
                    .find_or_last(|&(_, d)| (0.1..10.).contains(&d))
                    .unwrap()
                    .0,
            },
        }
    }
}

#[derive(Debug, Copy, Display, Clone, PartialEq, PartialOrd)]
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
    const fn ratio(&self) -> f64 {
        match self {
            SiPrefix::Mega => 1e-6,
            SiPrefix::Kilo => 1e-3,
            SiPrefix::Base => 1e-0,
            SiPrefix::Milli => 1e+3,
            SiPrefix::Micro => 1e6,
            SiPrefix::Nano => 1e+9,
        }
    }

    const fn all() -> [SiPrefix; 6] {
        [
            SiPrefix::Mega,
            SiPrefix::Kilo,
            SiPrefix::Base,
            SiPrefix::Milli,
            SiPrefix::Micro,
            SiPrefix::Nano,
        ]
    }

    fn express_value(&self, value: i64, base: SiPrefix) -> f64 {
        value as f64 * (self.ratio() / base.ratio())
    }
}

#[derive(Debug, Copy, Display, Clone, PartialEq, PartialOrd)]
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

    const fn all() -> [DurationUnit; 6] {
        [
            DurationUnit::Hour,
            DurationUnit::Minute,
            DurationUnit::Second,
            DurationUnit::Millisecond,
            DurationUnit::Microsecond,
            DurationUnit::Nanosecond,
        ]
    }

    fn express_value(&self, value: i64, base: DurationUnit) -> f64 {
        value as f64 * (self.ratio() / base.ratio())
    }
}

pub enum AxisQuantity {
    Duration { base: DurationUnit },
    SimpleSi { base: SiPrefix },
}

impl AxisQuantity {
    pub fn to_best_fit(&self) -> AxisBestFit {
        match &self {
            AxisQuantity::Duration { base } => AxisBestFit::Duration {
                base: *base,
                target: *base,
            },
            AxisQuantity::SimpleSi { base, .. } => AxisBestFit::SimpleSi {
                base: *base,
                target: *base,
            },
        }
    }
}

pub enum AxisBestFit {
    Duration {
        base: DurationUnit,
        target: DurationUnit,
    },
    SimpleSi {
        base: SiPrefix,
        target: SiPrefix,
    },
}

impl AxisBestFit {
    pub fn name(&self, descriptor: &AxisDescriptor) -> String {
        match self {
            AxisBestFit::Duration { target, .. } => format!("{} [{}]", descriptor.label, target),
            AxisBestFit::SimpleSi { target, .. } => match target {
                SiPrefix::Base => descriptor.label.to_owned(),
                _ => format!("{} [{}]", descriptor.label, target),
            },
        }
    }

    pub fn convert(&self, value: i64) -> f64 {
        match self {
            AxisBestFit::Duration { base, target } => target.express_value(value, *base),
            AxisBestFit::SimpleSi { base, target } => target.express_value(value, *base),
        }
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
                    quantity: AxisQuantity::Duration {
                        base: DurationUnit::Nanosecond,
                    },
                },
                y: AxisDescriptor {
                    label: "Samples",
                    quantity: AxisQuantity::SimpleSi {
                        base: SiPrefix::Base,
                    },
                },
            },
            ChartedValue::ActivationsDelay => AxisDescriptors {
                x: AxisDescriptor {
                    label: "Delay",
                    quantity: AxisQuantity::Duration {
                        base: DurationUnit::Nanosecond,
                    },
                },
                y: AxisDescriptor {
                    label: "Activations",
                    quantity: AxisQuantity::SimpleSi {
                        base: SiPrefix::Base,
                    },
                },
            },
            ChartedValue::PublicationsDelay => AxisDescriptors {
                x: AxisDescriptor {
                    label: "Delay",
                    quantity: AxisQuantity::Duration {
                        base: DurationUnit::Nanosecond,
                    },
                },
                y: AxisDescriptor {
                    label: "Publications",
                    quantity: AxisQuantity::SimpleSi {
                        base: SiPrefix::Base,
                    },
                },
            },
            ChartedValue::MessagesDelay => AxisDescriptors {
                x: AxisDescriptor {
                    label: "Delay",
                    quantity: AxisQuantity::Duration {
                        base: DurationUnit::Nanosecond,
                    },
                },
                y: AxisDescriptor {
                    label: "Messages",
                    quantity: AxisQuantity::SimpleSi {
                        base: SiPrefix::Base,
                    },
                },
            },
            ChartedValue::MessagesLatency => todo!(),
        },
        ChartVariants::Scatter => match charted_value {
            ChartedValue::CallbackDuration => AxisDescriptors {
                x: AxisDescriptor {
                    label: "Nth Sample",
                    quantity: AxisQuantity::SimpleSi {
                        base: SiPrefix::Base,
                    },
                },
                y: AxisDescriptor {
                    label: "Duration",
                    quantity: AxisQuantity::Duration {
                        base: DurationUnit::Nanosecond,
                    },
                },
            },
            ChartedValue::ActivationsDelay => AxisDescriptors {
                x: AxisDescriptor {
                    label: "Nth Activation",
                    quantity: AxisQuantity::SimpleSi {
                        base: SiPrefix::Base,
                    },
                },
                y: AxisDescriptor {
                    label: "Delay",
                    quantity: AxisQuantity::Duration {
                        base: DurationUnit::Nanosecond,
                    },
                },
            },
            ChartedValue::PublicationsDelay => AxisDescriptors {
                x: AxisDescriptor {
                    label: "Nth Publication",
                    quantity: AxisQuantity::SimpleSi {
                        base: SiPrefix::Base,
                    },
                },
                y: AxisDescriptor {
                    label: "Delay",
                    quantity: AxisQuantity::Duration {
                        base: DurationUnit::Nanosecond,
                    },
                },
            },
            ChartedValue::MessagesDelay => AxisDescriptors {
                x: AxisDescriptor {
                    label: "Nth Message",
                    quantity: AxisQuantity::SimpleSi {
                        base: SiPrefix::Base,
                    },
                },
                y: AxisDescriptor {
                    label: "Delay",
                    quantity: AxisQuantity::Duration {
                        base: DurationUnit::Nanosecond,
                    },
                },
            },
            ChartedValue::MessagesLatency => todo!(),
        },
    }
}
