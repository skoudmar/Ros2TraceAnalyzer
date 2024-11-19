use std::fmt::Display;
use std::sync::LazyLock;

use colorgrad::{Gradient, GradientBuilder, LinearGradient};

use crate::args::ANALYSIS_CLI_ARGS;

pub mod graphviz_export;

pub static COLOR_GRADIENT: LazyLock<ColorGradient> = LazyLock::new(ColorGradient::new);

pub struct ColorGradient {
    gradient: LinearGradient,
}

impl ColorGradient {
    pub fn new() -> Self {
        Self {
            gradient: GradientBuilder::new()
                .html_colors(&["seagreen", "gold", "red"])
                .build()
                .expect("Failed to build gradient"),
        }
    }

    /// Returns a color for the given value in the range [0, 1].
    pub fn color(&self, value: f32) -> Color {
        Color(self.gradient.at(value).to_rgba8())
    }

    /// Returns a color for the given value in the range `[min, max]`.
    pub fn color_for_range(&self, value: i64, min: i64, max: i64) -> Color {
        let value = (value - min) as f32 / (max - min) as f32;
        self.color(value)
    }

    /// Returns a color for the given value in the range `[min, MAX(max, min * min_multiplier)]`.
    ///
    /// The [`min_multiplier`](ANALYSIS_CLI_ARGS.min_multiplier) is taken from the [`ANALYSIS_CLI_ARGS`] variable.
    pub fn color_for_range_with_min_multiplier(&self, value: i64, min: i64, max: i64) -> Color {
        let max = max.max(
            min * ANALYSIS_CLI_ARGS
                .get()
                .expect("CLI arguments should be set")
                .min_multiplier,
        );
        self.color_for_range(value, min, max)
    }
}

pub struct Color([u8; 4]);

impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "#{:02X}{:02X}{:02X}{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3]
        )
    }
}
