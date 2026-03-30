use std::path::PathBuf;

use plotters::chart::{ChartBuilder, ChartContext, LabelAreaPosition};
use plotters::coord::ranged1d::ValueFormatter;
use plotters::prelude::{BitMapBackend, Cartesian2d, DrawingBackend, IntoDrawingArea, Ranged};
use plotters_svg::SVGBackend;

use crate::argsv2::chart_args::{ChartOutputFormat, ChartRequest, ChartVariants};
use crate::charting::axis_descriptor::{
    AxisDescriptors, ScaledAxisDescriptor, resolve_axis_descriptors,
};
use crate::charting::charts::ChartData;
use crate::charting::charts::histogram::HistogramChart;
use crate::charting::charts::scatter::ScatterChart;
use crate::charting::error::{ChartConstructionCommonError, ChartConstructionError};
use crate::extract::ChartableData;

mod axis_descriptor;
mod charts;
mod error;

pub fn render_chart(
    file_name: &PathBuf,
    charting_data: ChartableData,
    chart_request: &ChartRequest,
    output_format: ChartOutputFormat,
) -> Result<(), ChartConstructionCommonError> {
    let spacing = ChartSpacing::try_from((chart_request.size.0, chart_request.size.1))?;

    let axis_description = resolve_axis_descriptors(chart_request.quantity, &chart_request.plot);

    match output_format {
        crate::argsv2::chart_args::ChartOutputFormat::Svg => draw_into_canvas(
            SVGBackend::new(&file_name, (chart_request.size.0, chart_request.size.1)),
            charting_data,
            &chart_request.plot,
            &spacing,
            &axis_description,
        )?,
        crate::argsv2::chart_args::ChartOutputFormat::Png => draw_into_canvas(
            BitMapBackend::new(&file_name, (chart_request.size.0, chart_request.size.1)),
            charting_data,
            &chart_request.plot,
            &spacing,
            &axis_description,
        )?,
    };

    Ok(())
}

struct ChartSpacing {
    /// Margins of the actual plot
    ///
    /// [left, top, right, bottom]
    pub margin: [i32; 4],

    /// Margins of the labels
    ///
    /// [left, top, right, bottom]
    pub label_margin: [i32; 4],

    /// Font size of the tick labels
    ///
    /// [left, bottom]
    pub label_size: [i32; 2],

    /// Font size of the axis description
    pub desc_size: i32,
}

impl TryFrom<(u32, u32)> for ChartSpacing {
    type Error = ChartConstructionCommonError;

    fn try_from(value: (u32, u32)) -> Result<Self, Self::Error> {
        let aspect_ratio = value.0 as f32 / value.1 as f32;
        if !(0.5..2.0).contains(&aspect_ratio) {
            return Err(ChartConstructionCommonError::ChartSizeRatio(aspect_ratio));
        }

        Ok(match value {
            (400..800, 400..800) => ChartSpacing {
                margin: [16; 4],
                label_margin: [48, 0, 0, 48],
                label_size: [12; 2],
                desc_size: 20,
            },
            (800.., _) | (_, 800..) => ChartSpacing {
                margin: [32; 4],
                label_margin: [82, 0, 0, 64],
                label_size: [20; 2],
                desc_size: 32,
            },
            _ => {
                return Err(ChartConstructionCommonError::ChartSizeTooSmall(
                    value.0, value.1,
                ));
            }
        })
    }
}

fn label_axis<B: DrawingBackend>(
    mut chart: ChartContext<
        '_,
        B,
        Cartesian2d<
            impl Ranged<ValueType = i64> + ValueFormatter<i64>,
            impl Ranged<ValueType = i64> + ValueFormatter<i64>,
        >,
    >,
    scaled_axis_descriptor: &[ScaledAxisDescriptor; 2],
    sizes: &ChartSpacing,
) -> Result<(), ChartConstructionError<B::ErrorType>> {
    chart
        .configure_mesh()
        .max_light_lines(1)
        .x_desc(scaled_axis_descriptor[0].name())
        .y_desc(scaled_axis_descriptor[1].name())
        .x_label_formatter(&|v| {
            format!("{:.2}", scaled_axis_descriptor[0].convert(*v))
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        })
        .y_label_formatter(&|v| {
            format!("{:.2}", scaled_axis_descriptor[1].convert(*v))
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        })
        .axis_desc_style(("sans-serif", sizes.desc_size))
        .y_label_style(("sans-serif", sizes.label_size[0]))
        .x_label_style(("sans-serif", sizes.label_size[1]))
        .draw()
        .map_err(ChartConstructionError::InvalidCoordinateSystem)
}

fn draw_into_canvas<B: DrawingBackend>(
    canvas: B,
    data: ChartableData,
    variant: &ChartVariants,
    spacing: &ChartSpacing,
    axis_description: &AxisDescriptors,
) -> Result<(), ChartConstructionError<B::ErrorType>> {
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
            let histogram = HistogramChart::new(histogram_data, data, axis_description);
            label_axis(
                histogram.draw_into(&mut chart)?,
                histogram.scale_axis(),
                spacing,
            )?;
        }
        ChartVariants::Scatter => {
            let scatter = ScatterChart::new(data, axis_description);
            label_axis(
                scatter.draw_into(&mut chart)?,
                scatter.scale_axis(),
                spacing,
            )?;
        }
    }

    area.present().map_err(ChartConstructionError::DrawingError)
}
