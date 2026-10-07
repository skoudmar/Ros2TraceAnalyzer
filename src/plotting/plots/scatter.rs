use plotters::chart::{ChartBuilder, ChartContext};
use plotters::coord::types::RangedCoordi64;
use plotters::prelude::{Cartesian2d, Circle, DrawingBackend};
use plotters::style::Color;

use crate::extract::PlottableData;
use crate::plotting::axis_descriptor::{AxisDescriptors, ScaledAxisDescriptor};
use crate::plotting::error::PlotConstructionError;
use crate::plotting::plots::{PlotData, resolve_axis_range};

pub struct ScatterPlot {
    x_range: (i64, i64),
    y_range: (i64, i64),
    data: Vec<i64>,
    scaled_axis: [ScaledAxisDescriptor; 2],
}

impl ScatterPlot {
    pub fn new(data: &[i64], axis_descriptors: &AxisDescriptors) -> Self {
        let x_range = (0, data.len() as i64);
        let y_range = resolve_axis_range(&data);

        let scaled_axis = [
            axis_descriptors
                .x
                .scaled_axis_unit((x_range.1 - x_range.0) / 2),
            axis_descriptors
                .y
                .scaled_axis_unit((y_range.1 - y_range.0) / 2),
        ];

        ScatterPlot {
            x_range,
            y_range,
            data: data.to_vec(),
            scaled_axis,
        }
    }
}

type Coords = Cartesian2d<RangedCoordi64, RangedCoordi64>;
impl PlotData<Coords> for ScatterPlot {
    fn draw_into<'a, B: DrawingBackend>(
        &self,
        canvas: &mut ChartBuilder<B>,
    ) -> Result<ChartContext<'a, B, Coords>, PlotConstructionError<B::ErrorType>> {
        let mut context = canvas
            .build_cartesian_2d(
                self.x_range.0..self.x_range.1,
                self.y_range.0..self.y_range.1,
            )
            .map_err(PlotConstructionError::InvalidCoordinateSystem)?;

        context
            .draw_series(
                self.data
                    .iter()
                    .enumerate()
                    .map(|(i, &y)| Circle::new((i as i64, y), 2, plotters::style::BLUE.filled())),
            )
            .map_err(PlotConstructionError::PlotSeriesError)?;

        Ok(context)
    }

    fn scale_axis(&self) -> &[ScaledAxisDescriptor; 2] {
        &self.scaled_axis
    }
}
