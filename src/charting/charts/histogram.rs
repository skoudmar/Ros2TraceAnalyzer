use std::collections::HashMap;

use itertools::Itertools;
use plotters::chart::{ChartBuilder, ChartContext};
use plotters::coord::types::RangedCoordi64;
use plotters::prelude::{Cartesian2d, DrawingBackend, IntoLogRange, LogCoord, Rectangle};
use plotters::style::Color;

use crate::argsv2::chart_args::HistogramData;
use crate::charting::axis_descriptor::{self, AxisBestFit, AxisDescriptors};
use crate::charting::charts::{ChartData, resolve_axis_range};
use crate::charting::error::ChartConstructionError;
use crate::extract::ChartableData;

pub struct HistogramChart {
    _bins: u64,
    bin_width: u64,
    x_range: (i64, i64),
    y_range: (i64, i64),
    data: HashMap<u64, i64>,
    axis_fits: [AxisBestFit; 2],
}

impl HistogramChart {
    pub fn new(
        histogram_data: &HistogramData,
        data: &ChartableData,
        axis_descriptor: &AxisDescriptors,
    ) -> Self {
        let data = match data {
            ChartableData::I64(items) => items.clone(),
        };

        // How many bins the data should be split into (this is how many bins will actuall render)
        let data_bins = if let Some(bins) = histogram_data.bins {
            bins as u64
        } else {
            50
            // TODO
        };

        let (min, max) = match data.iter().minmax() {
            itertools::MinMaxResult::NoElements => (0, 0),
            itertools::MinMaxResult::OneElement(e) => (*e, *e),
            itertools::MinMaxResult::MinMax(l, h) => (*l, *h),
        };

        let (bin_width, x_range) = histogram_x_axis_alignment(min, max, data_bins);

        let data: std::collections::HashMap<u64, i64> = data
            .iter()
            .into_group_map_by(|&v| ((v - min) / bin_width).min(data_bins as i64 - 1))
            .iter()
            .map(|(&k, v)| (k as u64, v.len() as i64))
            .collect();

        let y_range = resolve_axis_range(&data.iter().map(|v| *v.1).collect_vec());

        let axis_fits = [
            axis_descriptor.x.best_fit(x_range.1 / 2),
            axis_descriptor.y.quantity.to_best_fit(),
        ];

        HistogramChart {
            _bins: data_bins,
            bin_width: bin_width as u64,
            x_range,
            y_range: (0, y_range.1),
            data,
            axis_fits,
        }
    }
}

type Coords = Cartesian2d<RangedCoordi64, LogCoord<i64>>;
impl ChartData<Coords> for HistogramChart {
    fn draw_into<'a, B: DrawingBackend>(
        &self,
        canvas: &mut ChartBuilder<B>,
    ) -> Result<ChartContext<'a, B, Coords>, ChartConstructionError> {
        let mut context = canvas
            .build_cartesian_2d(
                self.x_range.0..self.x_range.1,
                (self.y_range.0..self.y_range.1).log_scale(),
            )
            .map_err(|e| ChartConstructionError::InvalidCoordinateSystem(e.to_string()))?;

        context
            .draw_series(self.data.iter().map(|(b, size)| {
                let x0 = self.x_range.0 + (b * self.bin_width) as i64;
                let x1 = x0 + self.bin_width as i64;

                Rectangle::new([(x0, *size), (x1, 0)], plotters::style::BLUE.filled())
            }))
            .map_err(|e| ChartConstructionError::ChartSeriesError(e.to_string()))?;

        Ok(context)
    }

    fn axis_fits(&self) -> &[axis_descriptor::AxisBestFit; 2] {
        &self.axis_fits
    }
}

// This method selects a x axis range so that all ticks are placed
// on "nice" round numbers
fn histogram_x_axis_alignment(min: i64, max: i64, data_bins: u64) -> (i64, (i64, i64)) {
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
