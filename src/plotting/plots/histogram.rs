use itertools::Itertools;
use plotters::chart::{ChartBuilder, ChartContext};
use plotters::coord::types::RangedCoordf64;
use plotters::prelude::{Cartesian2d, DrawingBackend, IntoLogRange, LogCoord, Rectangle};
use plotters::style::Color;

use crate::argsv2::plot_args::HistogramData;
use crate::plotting::axis_descriptor::{AxisDescriptors, ScaledAxisDescriptor};
use crate::plotting::error::PlotConstructionError;
use crate::plotting::plots::PlotData;

pub struct HistogramPlot {
    _bin_count: usize,
    bin_width: u64,
    x_range: (i64, i64),
    y_range: (i64, i64),
    data: Vec<i64>,
    scaled_axis: [ScaledAxisDescriptor; 2],
}

impl HistogramPlot {
    pub fn new(
        histogram_data: &HistogramData,
        data: &[i64],
        axis_descriptor: &AxisDescriptors,
    ) -> Self {
        let min_bin_count = if let Some(bins) = histogram_data.scale
            && bins != 0
        {
            bins
        } else {
            // Sturges's formula
            1 + data
                .len()
                .checked_next_power_of_two() // ceil function for ilog2
                .expect("Data size should be smaller than usize::MAX / 2 + 1")
                .ilog2() as usize
        };

        let (min, max) = match data.iter().minmax() {
            itertools::MinMaxResult::NoElements => (0, 0),
            itertools::MinMaxResult::OneElement(e) => (*e, *e),
            itertools::MinMaxResult::MinMax(l, h) => (*l, *h),
        };

        let x_axis = histogram_x_axis_alignment(min, max, min_bin_count);
        let bin_count = usize::try_from((x_axis.end - x_axis.start) / x_axis.bin_width)
            .expect("bin_count must fit into usize");

        let mut binned_data = vec![0; bin_count];
        let last_idx = bin_count
            .checked_sub(1)
            .expect("bin_count must be at least 1");

        for d in data {
            let bin = usize::try_from((d - x_axis.start) / x_axis.bin_width)
                .unwrap()
                .min(last_idx);

            binned_data[bin] += 1;
        }

        let y_max = *binned_data
            .iter()
            .max()
            .expect("bin_count must be at least 1");

        let scaled_axis = [
            axis_descriptor
                .x
                .scaled_axis_unit(x_axis.start + (x_axis.end - x_axis.start) / 2),
            // This has logarithmic scale so there is no reasonable unit to cover
            // the entire range. If this becomes a problem we can allow for formatting
            // individual ticks and display just the exponents
            axis_descriptor.y.scaled_axis_unit(1),
        ];

        HistogramPlot {
            _bin_count: bin_count,
            bin_width: x_axis.bin_width as u64,
            x_range: (x_axis.start, x_axis.end),
            y_range: (0, y_max),
            data: binned_data,
            scaled_axis,
        }
    }
}

type Coords = Cartesian2d<RangedCoordf64, LogCoord<i64>>;
impl PlotData<Coords> for HistogramPlot {
    fn draw_into<'a, B: DrawingBackend>(
        &self,
        canvas: &mut ChartBuilder<B>,
    ) -> Result<ChartContext<'a, B, Coords>, PlotConstructionError<B::ErrorType>> {
        let mut context = canvas
            .build_cartesian_2d(
                (self.x_range.0 as f64)..(self.x_range.1 as f64),
                (self.y_range.0..self.y_range.1).log_scale(),
            )
            .map_err(PlotConstructionError::InvalidCoordinateSystem)?;

        let margin = self.bin_width as f64 * 0.05;

        context
            .draw_series(self.data.iter().enumerate().map(|(b, size)| {
                let x0 = (self.x_range.0 + (b as u64 * self.bin_width) as i64) as f64;
                let x1 = x0 + self.bin_width as f64;

                Rectangle::new(
                    [(x0 + margin, *size), (x1 - margin, 0)],
                    plotters::style::BLUE.filled(),
                )
            }))
            .map_err(PlotConstructionError::PlotSeriesError)?;

        Ok(context)
    }

    fn scale_axis(&self) -> &[ScaledAxisDescriptor; 2] {
        &self.scaled_axis
    }
}

struct XAxis {
    bin_width: i64,
    start: i64,
    end: i64,
}

/// # Histogram axis alignment
///
/// Expands a histogram x-axis so the axis bounds and tick spacing land on
/// human-friendly values.
///
/// ## Alignment strategy
///
/// 1. Estimate a raw bin width from `range / data_bins`
/// 2. Round that width to a "nice round" value (`1`, `2`, `5`, or `10` × power of ten)
/// 3. Snap the axis start down to the nearest bin boundary
/// 4. Snap the axis end up to the nearest bin boundary
/// 5. Expand the axis further if needed so the full range is divisible by
///    `4 * bin_width`
///
/// Expansion is applied symmetrically where possible. If extending left would
/// move the axis below `0`, the start is clamped at `0` and the remaining
/// expansion is applied to the right side.
///
/// ## "Nice" round numbers
///
/// A "nice" number is a value that is easy for humans to read and reason about,
/// typically with only a few significant digits.
///
/// For example:
/// - `4125152` → `4130000` (ns to 4.13 ms)
/// - `1.3` → `2` (ns to 2 ns)
/// - `552342` → `550000` (ns to 550 us)
/// - `8656757` → `8700000` (ns to 8.7 ms)
fn histogram_x_axis_alignment(min: i64, max: i64, data_bins: usize) -> XAxis {
    /// Round an estimated bin width into a human readable value.
    /// Examples:
    ///
    /// - `1.3`   → `2`
    /// - `3.1`   → `5`
    /// - `812.0` → `1000`
    fn round_bin_width(value: f64) -> i64 {
        // Determine the order of magnitude of the value.
        let exponent = value.log10().floor();

        // Convert the value into scientific notation:
        // value = normalized * magnitude
        let magnitude = 10f64.powf(exponent);
        let normalized = value / magnitude;

        // Snap the normalized component to a "nice" base value.
        let nice_base = if normalized <= 1.0 {
            1.0
        } else if normalized <= 2.0 {
            2.0
        } else if normalized <= 5.0 {
            5.0
        } else {
            10.0
        };

        let bw = (nice_base * magnitude).round() as i64;

        // Prevent zero-width bins for extremely small values.
        if bw == 0 { 1 } else { bw }
    }

    let raw_range = (max - min) as u64;

    // Degenerate case:
    // if all values are identical, create a small default range around them.
    if raw_range == 0 {
        XAxis {
            bin_width: 1,
            start: (min - 4).max(0),
            end: max + 4,
        }
    } else {
        // Estimate the ideal bin width from the requested bin count.
        let normalized = raw_range as f64 / data_bins as f64;

        // Convert the estimated width into a readable value.
        let bin_width = round_bin_width(normalized);

        // Snap the start down to the nearest aligned bin boundary.
        let mut x_start = min.div_euclid(bin_width) * bin_width;

        // Snap the end up to the next aligned bin boundary.
        let mut x_end = if max % bin_width == 0 {
            max
        } else {
            (max.div_euclid(bin_width) + 1) * bin_width
        };

        // Current aligned range.
        let t_range = x_end - x_start;

        // Require divisibility by 4 bins so quarter ticks align nicely.
        let required = 4 * bin_width;

        // Amount by which the current range misses the requirement.
        let r = t_range % required;

        if r != 0 {
            // Additional expansion needed.
            let exp = required - r;

            // Try to distribute expansion evenly across both sides.
            let l_e = exp / 2;
            let r_e = exp - l_e;

            // Prevent the left edge from becoming negative.
            if x_start - l_e < 0 {
                let c = x_start;

                x_start = 0;

                // Apply the leftover expansion to the right side.
                x_end += exp - c;
            } else {
                x_start -= l_e;
                x_end += r_e;
            }
        }

        XAxis {
            bin_width,
            start: x_start,
            end: x_end,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{XAxis, histogram_x_axis_alignment};

    fn assert_axis_invariants(min: i64, max: i64, data_bins: usize) -> (i64, i64, i64) {
        let XAxis {
            bin_width: bw,
            start,
            end,
        } = histogram_x_axis_alignment(min, max, data_bins);

        assert!(start <= end, "axis start must not exceed axis end");
        assert!(start >= 0, "axis start must saturate at zero");

        assert!(start <= min, "axis start clipped minimum value");
        assert!(end >= max, "axis end clipped maximum value");

        let total_range = end - start;
        assert_eq!(
            total_range % (4 * bw),
            0,
            "range must be divisible by 4 * bin_width"
        );

        let q1 = start + total_range / 4;
        let q2 = start + total_range / 2;
        let q3 = start + (3 * total_range) / 4;

        assert_eq!((q1 - start) % bw, 0, "quarter tick misalignment");
        assert_eq!((q2 - start) % bw, 0, "half tick misalignment");
        assert_eq!((q3 - start) % bw, 0, "three-quarter tick misalignment");

        (bw, start, end)
    }

    #[test]
    fn zero_range_creates_default_padding() {
        let XAxis {
            bin_width: bw,
            start,
            end,
        } = histogram_x_axis_alignment(10, 10, 8);

        assert_eq!(bw, 1);

        // The implementation pads by 4 units in both directions.
        assert_eq!(start, 6);
        assert_eq!(end, 14);

        // Original value must remain visible.
        assert!(start <= 10);
        assert!(end >= 10);
    }

    #[test]
    fn already_aligned_ranges_remain_stable() {
        let (bw, start, end) = assert_axis_invariants(0, 40, 4);

        assert_eq!(bw, 10);

        // The range is already perfectly aligned:
        //
        // - starts at 0
        // - ends at 40
        // - divisible by 4 * 10
        //
        // Therefore no extra expansion should occur.
        assert_eq!(start, 0);
        assert_eq!(end, 40);
    }

    #[test]
    fn unaligned_range_expands_symmetrically() {
        let (bw, start, end) = assert_axis_invariants(13, 87, 10);

        // 74 / 10 = 7.4
        // Rounded "nice" width should become 10.
        assert_eq!(bw, 10);

        // Initial snapped range:
        // 10..90 (range = 80)
        //
        // Required divisibility:
        // 4 * 10 = 40
        //
        // 80 is already divisible by 40,
        // therefore no additional expansion should occur.
        assert_eq!(start, 10);
        assert_eq!(end, 90);
    }

    #[test]
    fn non_divisible_range_requires_extra_expansion() {
        let (bw, start, end) = assert_axis_invariants(10, 95, 10);

        assert_eq!(bw, 10);

        // Initial snapped range:
        // 10..100 => 90 units
        //
        // Required multiple:
        // 4 * 10 = 40
        //
        // 90 % 40 = 10
        // Need +30 expansion.
        //
        // Expansion is split:
        // left  = 15
        // right = 15
        //
        // Left expansion would cross zero,
        // therefore remaining space shifts right.
        assert_eq!(start, 0);
        assert_eq!(end, 120);

        assert_eq!(end - start, 120);
        assert_eq!((end - start) % (4 * bw), 0);
    }

    #[test]
    fn large_values_round_to_higher_magnitudes() {
        let (bw, start, end) = assert_axis_invariants(4_125_152, 4_991_221, 20);

        // Range ≈ 866k
        // Bin estimate ≈ 43k
        //
        // Should round to a readable value like 50k.
        assert_eq!(bw, 50_000);

        assert_eq!((end - start) % (4 * bw), 0);
    }

    #[test]
    fn narrow_ranges_with_large_bin_counts_work() {
        let (bw, start, end) = assert_axis_invariants(99, 101, 1_000);

        // Extremely small normalized width.
        assert!(bw >= 1);

        // Axis still properly contains the data.
        assert!(start <= 99);
        assert!(end >= 101);
    }

    #[test]
    fn fuzz_invariant_validation() {
        for min in (0..1000).step_by(37) {
            for max in ((min + 1)..(min + 500)).step_by(53) {
                for bins in [1usize, 2, 5, 10, 20, 50] {
                    assert_axis_invariants(min, max, bins);
                }
            }
        }
    }
}
