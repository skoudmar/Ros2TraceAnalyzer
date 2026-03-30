use std::fmt::Display;
use std::sync::LazyLock;

use colorgrad::{Gradient, GradientBuilder, LinearGradient};

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
        let value = Self::normalize_for_range(value, min, max);
        self.color(value)
    }

    fn normalize_for_range(value: i64, min: i64, max: i64) -> f32 {
        const DEFAULT_RATIO: f32 = 0.5;

        if max > min {
            ((value - min) as f32 / (max - min) as f32).clamp(0.0, 1.0)
        } else {
            DEFAULT_RATIO
        }
    }
}

impl Default for ColorGradient {
    fn default() -> Self {
        Self::new()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_for_range_handles_zero_width_range() {
        let normalized = ColorGradient::normalize_for_range(42, 10, 10);
        assert_eq!(normalized, 0.5);
    }

    #[test]
    fn normalize_for_range_clamps_out_of_bounds_values() {
        assert_eq!(ColorGradient::normalize_for_range(0, 10, 20), 0.0);
        assert_eq!(ColorGradient::normalize_for_range(30, 10, 20), 1.0);
    }

    #[test]
    fn normalize_for_range_maps_middle_value() {
        assert_eq!(ColorGradient::normalize_for_range(15, 10, 20), 0.5);
    }
}
