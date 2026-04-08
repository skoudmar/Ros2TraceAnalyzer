use itertools::Itertools;
use plotters::chart::{ChartBuilder, ChartContext};
use plotters::coord::types::RangedCoordi64;
use plotters::prelude::{Cartesian2d, DrawingBackend, IntoLogRange, LogCoord, Rectangle};
use plotters::style::Color;

use crate::argsv2::plot_args::HistogramData;
use crate::extract::PlottableData;
use crate::plotting::axis_descriptor::{AxisDescriptors, ScaledAxisDescriptor};
use crate::plotting::error::PlotConstructionError;
use crate::plotting::plots::{PlotData, resolve_axis_range};

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
        data: PlottableData,
        axis_descriptor: &AxisDescriptors,
    ) -> Self {
        let PlottableData::I64(data) = data;

        // How many bins the data should be split into (this is how many bins will actually render)
        let bin_count = if let Some(bins) = histogram_data.bins
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

        let (bin_width, x_range) = histogram_x_axis_alignment(min, max, bin_count);

        let mut binned_data = vec![0; bin_count];
        let last_idx = bin_count
            .checked_sub(1)
            .expect("bin_count must be at least 1");

        for d in data {
            let bin = usize::try_from((d - min) / bin_width)
                .unwrap()
                .min(last_idx);

            binned_data[bin] += 1;
        }

        let y_range = resolve_axis_range(&binned_data);

        let scaled_axis = [
            axis_descriptor
                .x
                .scaled_axis_unit((x_range.1 - x_range.0) / 2),
            // This has logarithmic scale so there is no reasonable unit to cover
            // the entire range. If this becomes a problem we can allow for formatting
            // individual ticks and display just the exponents
            axis_descriptor.y.scaled_axis_unit(1),
        ];

        HistogramPlot {
            _bin_count: bin_count,
            bin_width: bin_width as u64,
            x_range,
            y_range: (0, y_range.1),
            data: binned_data,
            scaled_axis,
        }
    }
}

type Coords = Cartesian2d<RangedCoordi64, LogCoord<i64>>;
impl PlotData<Coords> for HistogramPlot {
    fn draw_into<'a, B: DrawingBackend>(
        &self,
        canvas: &mut ChartBuilder<B>,
    ) -> Result<ChartContext<'a, B, Coords>, PlotConstructionError<B::ErrorType>> {
        let mut context = canvas
            .build_cartesian_2d(
                self.x_range.0..self.x_range.1,
                (self.y_range.0..self.y_range.1).log_scale(),
            )
            .map_err(PlotConstructionError::InvalidCoordinateSystem)?;

        context
            .draw_series(self.data.iter().enumerate().map(|(b, size)| {
                let x0 = self.x_range.0 + (b as u64 * self.bin_width) as i64;
                let x1 = x0 + self.bin_width as i64;

                Rectangle::new([(x0, *size), (x1, 0)], plotters::style::BLUE.filled())
            }))
            .map_err(PlotConstructionError::PlotSeriesError)?;

        Ok(context)
    }

    fn scale_axis(&self) -> &[ScaledAxisDescriptor; 2] {
        &self.scaled_axis
    }
}

// This method selects a x axis range so that all ticks are placed
// on "nice" round numbers
fn histogram_x_axis_alignment(min: i64, max: i64, data_bins: usize) -> (i64, (i64, i64)) {
    fn round_bin_width(value: f64) -> i64 {
        let exponent = value.log10().floor();
        let magnitude = 10f64.powf(exponent);
        let normalized = value / magnitude;

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
        if bw == 0 { 1 } else { bw }
    }

    let raw_range = (max - min) as u64;
    if raw_range == 0 {
        (1, ((min - 4).max(0), max + 4))
    } else {
        let normalized = raw_range as f64 / data_bins as f64;
        let bin_width = round_bin_width(normalized);

        let mut x_start = min.div_euclid(bin_width) * bin_width;
        let mut x_end = if max % bin_width == 0 {
            max
        } else {
            (max.div_euclid(bin_width) + 1) * bin_width
        };

        let t_range = x_end - x_start;
        let required = 4 * bin_width;

        let r = t_range % required;

        if r != 0 {
            let exp = required - r;

            let l_e = exp / 2;
            let r_e = exp - l_e;

            if x_start - l_e < 0 {
                let c = x_start;
                x_start = 0;

                x_end += exp - c;
            } else {
                x_start -= l_e;
                x_end += r_e;
            }
        }

        (bin_width, (x_start, x_end))
    }
}
