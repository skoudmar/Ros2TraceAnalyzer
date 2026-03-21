use std::path::PathBuf;

use plotters::chart::{ChartBuilder, ChartContext, LabelAreaPosition};
use plotters::coord::ranged1d::ValueFormatter;
use plotters::prelude::{BitMapBackend, Cartesian2d, DrawingBackend, IntoDrawingArea, Ranged};
use plotters_svg::SVGBackend;

use crate::argsv2::chart_args::{ChartRequest, ChartVariants};
use crate::charting::axis_descriptor::{AxisBestFit, AxisDescriptors, resolve_axis_descriptors};
use crate::charting::charts::ChartData;
use crate::charting::charts::histogram::HistogramChart;
use crate::charting::charts::scatter::ScatterChart;
use crate::charting::error::ChartConstructionError;
use crate::extract::ChartableData;

mod axis_descriptor;
mod charts;
mod error;

pub fn render_chart(
    file_name: &PathBuf,
    charting_data: &ChartableData,
    chart_request: &ChartRequest,
) -> Result<(), ChartConstructionError> {
    let spacing = ChartSpacing::try_from(chart_request.size)?;

    let axis_description = resolve_axis_descriptors(&chart_request.value, &chart_request.plot);

    match chart_request.output_format {
        crate::argsv2::chart_args::ChartOutputFormat::Svg => draw_into_canvas(
            SVGBackend::new(&file_name, (chart_request.size, chart_request.size)),
            charting_data,
            &chart_request.plot,
            &spacing,
            &axis_description,
        ),
        crate::argsv2::chart_args::ChartOutputFormat::Png => draw_into_canvas(
            BitMapBackend::new(&file_name, (chart_request.size, chart_request.size)),
            charting_data,
            &chart_request.plot,
            &spacing,
            &axis_description,
        ),
    }?;

    Ok(())
}

struct ChartSpacing {
    pub margin: [i32; 4],
    pub label_margin: [i32; 4],
    pub label_size: i32,
    pub desc_size: i32,
}

impl TryFrom<u32> for ChartSpacing {
    type Error = ChartConstructionError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(match value {
            400..700 => ChartSpacing {
                margin: [32; 4],
                label_margin: [48, 0, 0, 48],
                label_size: 12,
                desc_size: 20,
            },
            700.. => ChartSpacing {
                margin: [96; 4],
                label_margin: [value as i32 / 15, 0, 0, value as i32 / 15],
                label_size: value as i32 / 50,
                desc_size: value as i32 / 30,
            },
            _ => return Err(ChartConstructionError::ChartSizeTooSmall(value)),
        })
    }
}

fn label_axis<'a>(
    mut chart: ChartContext<
        'a,
        impl DrawingBackend,
        Cartesian2d<
            impl Ranged<ValueType = i64> + ValueFormatter<i64>,
            impl Ranged<ValueType = i64> + ValueFormatter<i64>,
        >,
    >,
    axis_best_fits: &[AxisBestFit; 2],
    axis_description: &AxisDescriptors,
    sizes: &ChartSpacing,
) -> Result<(), ChartConstructionError> {
    chart
        .configure_mesh()
        .max_light_lines(1)
        .x_desc(axis_best_fits[0].name(&axis_description.x))
        .y_desc(axis_best_fits[1].name(&axis_description.y))
        .x_label_formatter(&|v| {
            format!("{:.2}", axis_best_fits[0].convert(*v))
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        })
        .y_label_formatter(&|v| {
            format!("{:.2}", axis_best_fits[1].convert(*v))
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        })
        .axis_desc_style(("sans-serif", sizes.desc_size))
        .label_style(("sans-serif", sizes.label_size))
        .draw()
        .map_err(|e| ChartConstructionError::InvalidCoordinateSystem(e.to_string()))?;

    Ok(())
}

fn draw_into_canvas(
    canvas: impl DrawingBackend,
    data: &ChartableData,
    variant: &ChartVariants,
    spacing: &ChartSpacing,
    axis_description: &AxisDescriptors,
) -> Result<(), ChartConstructionError> {
    let area = canvas.into_drawing_area();
    area.fill(&plotters::style::WHITE).unwrap();

    let mut chart = ChartBuilder::on(&area);

    chart
        .margin_left(spacing.margin[0])
        .margin_top(spacing.margin[1])
        .margin_right(spacing.margin[2])
        .margin_bottom(spacing.margin[3])
        .set_label_area_size(LabelAreaPosition::Left, spacing.label_margin[0])
        .set_label_area_size(LabelAreaPosition::Top, spacing.label_margin[1])
        .set_label_area_size(LabelAreaPosition::Right, spacing.label_margin[2])
        .set_label_area_size(LabelAreaPosition::Bottom, spacing.label_margin[3]);

    match &variant {
        ChartVariants::Histogram(histogram_data) => {
            let d = HistogramChart::new(histogram_data, data, axis_description);
            label_axis(
                d.draw_into(&mut chart)?,
                d.axis_fits(),
                axis_description,
                spacing,
            )?;
        }
        ChartVariants::Scatter => {
            let d = ScatterChart::new(data, axis_description);
            label_axis(
                d.draw_into(&mut chart)?,
                d.axis_fits(),
                axis_description,
                spacing,
            )?;
        }
    }

    area.present()
        .map_err(|e| ChartConstructionError::DrawingError(e.to_string()))?;

    Ok(())
}
