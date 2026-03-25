use plotters::chart::{ChartBuilder, ChartContext};
use plotters::coord::types::RangedCoordi64;
use plotters::prelude::{Cartesian2d, Circle, DrawingBackend};
use plotters::style::Color;

use crate::charting::axis_descriptor::{AxisDescriptors, ScaledAxisDescriptor};
use crate::charting::charts::{ChartData, resolve_axis_range};
use crate::charting::error::ChartConstructionError;
use crate::extract::ChartableData;

pub struct ScatterChart {
    x_range: (i64, i64),
    y_range: (i64, i64),
    data: Vec<(i64, i64)>,
    scaled_axis: [ScaledAxisDescriptor; 2],
}

impl ScatterChart {
    pub fn new(data: ChartableData, axis_descriptors: &AxisDescriptors) -> Self {
        let ChartableData::I64(data) = data;

        let x_range = (0, data.len() as i64);
        let y_range = resolve_axis_range(&data);

        let scaled_axis = [
            axis_descriptors.x.scaled_axis_unit(x_range.1),
            axis_descriptors.y.scaled_axis_unit(y_range.1),
        ];

        ScatterChart {
            x_range,
            y_range,
            data: data
                .iter()
                .enumerate()
                .map(|(i, e)| (i as i64, *e))
                .collect(),
            scaled_axis,
        }
    }
}

type Coords = Cartesian2d<RangedCoordi64, RangedCoordi64>;
impl ChartData<Coords> for ScatterChart {
    fn draw_into<'a, B: DrawingBackend>(
        &self,
        canvas: &mut ChartBuilder<B>,
    ) -> Result<ChartContext<'a, B, Coords>, ChartConstructionError<B::ErrorType>> {
        let mut context = canvas
            .build_cartesian_2d(
                self.x_range.0..self.x_range.1,
                self.y_range.0..self.y_range.1,
            )
            .map_err(ChartConstructionError::InvalidCoordinateSystem)?;

        context
            .draw_series(
                self.data
                    .iter()
                    .map(|&(x, y)| Circle::new((x, y), 2, plotters::style::BLUE.filled())),
            )
            .map_err(ChartConstructionError::ChartSeriesError)?;

        Ok(context)
    }

    fn scale_axis(&self) -> &[ScaledAxisDescriptor; 2] {
        &self.scaled_axis
    }
}
