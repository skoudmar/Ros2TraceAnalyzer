use plotters::chart::{ChartBuilder, ChartContext};
use plotters::coord::CoordTranslate;
use plotters::prelude::DrawingBackend;

use crate::charting::axis_descriptor::ScaledAxisDescriptor;
use crate::charting::error::ChartConstructionError;

pub mod histogram;
pub mod scatter;

pub trait ChartData<C: CoordTranslate> {
    fn scale_axis(&self) -> &[ScaledAxisDescriptor; 2];
    fn draw_into<'a, B: DrawingBackend>(
        &self,
        canvas: &mut ChartBuilder<B>,
    ) -> Result<ChartContext<'a, B, C>, ChartConstructionError<B::ErrorType>>;
}

pub fn resolve_axis_range(data: &[i64]) -> (i64, i64) {
    (*data.iter().min().unwrap(), *data.iter().max().unwrap())
}
