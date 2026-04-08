use plotters::chart::{ChartBuilder, ChartContext, LabelAreaPosition};
use plotters::coord::ranged1d::ValueFormatter;
use plotters::prelude::{BitMapBackend, Cartesian2d, DrawingBackend, IntoDrawingArea, Ranged};
use plotters_svg::SVGBackend;

use crate::argsv2::plot_args::{PlotOutputFormat, PlotRequest, PlotVariants};
use crate::extract::PlottableData;
use crate::plotting::axis_descriptor::{
    AxisDescriptors, ScaledAxisDescriptor, resolve_axis_descriptors,
};
use crate::plotting::error::{PlotConstructionCommonError, PlotConstructionError};
use crate::plotting::plots::PlotData;
use crate::plotting::plots::histogram::HistogramPlot;
use crate::plotting::plots::scatter::ScatterPlot;

mod axis_descriptor;
mod error;
mod plots;

pub fn render_plot(
    output: &mut Box<dyn std::io::Write>,
    plotting_data: PlottableData,
    plot_request: &PlotRequest,
    output_format: PlotOutputFormat,
) -> Result<(), PlotConstructionCommonError> {
    let spacing = PlotSpacing::try_from((plot_request.size.0, plot_request.size.1))?;

    let axis_description = resolve_axis_descriptors(plot_request.quantity, &plot_request.plot);

    match output_format {
        crate::argsv2::plot_args::PlotOutputFormat::Svg => {
            let mut out = String::new();
            draw_into_canvas(
                SVGBackend::with_string(&mut out, (plot_request.size.0, plot_request.size.1)),
                plotting_data,
                &plot_request.plot,
                &spacing,
                &axis_description,
            )?;
            output.write_all(out.as_bytes()).unwrap();
        }
        crate::argsv2::plot_args::PlotOutputFormat::Png => {
            let mut buffer = vec![0u8; (plot_request.size.0 * plot_request.size.1 * 3) as usize];
            draw_into_canvas(
                BitMapBackend::with_buffer(&mut buffer, (plot_request.size.0, plot_request.size.1)),
                plotting_data,
                &plot_request.plot,
                &spacing,
                &axis_description,
            )?;

            use image::ImageEncoder;
            use image::codecs::png::PngEncoder;

            let img_encoder = PngEncoder::new(&mut *output);
            img_encoder.write_image(
                &buffer,
                plot_request.size.0,
                plot_request.size.1,
                image::ColorType::Rgb8,
            )?;
        }
    };

    Ok(())
}

struct PlotSpacing {
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

impl TryFrom<(u32, u32)> for PlotSpacing {
    type Error = PlotConstructionCommonError;

    fn try_from(value: (u32, u32)) -> Result<Self, Self::Error> {
        let aspect_ratio = value.0 as f32 / value.1 as f32;
        if !(0.5..2.0).contains(&aspect_ratio) {
            return Err(PlotConstructionCommonError::PlotSizeRatio(aspect_ratio));
        }

        Ok(match value {
            (400..800, 400..800) => PlotSpacing {
                margin: [16; 4],
                label_margin: [48, 0, 0, 48],
                label_size: [12; 2],
                desc_size: 20,
            },
            (800.., _) | (_, 800..) => PlotSpacing {
                margin: [32; 4],
                label_margin: [82, 0, 0, 64],
                label_size: [20; 2],
                desc_size: 32,
            },
            _ => {
                return Err(PlotConstructionCommonError::PlotSizeTooSmall(
                    value.0, value.1,
                ));
            }
        })
    }
}

fn label_axis<B: DrawingBackend>(
    mut plot: ChartContext<
        '_,
        B,
        Cartesian2d<
            impl Ranged<ValueType = i64> + ValueFormatter<i64>,
            impl Ranged<ValueType = i64> + ValueFormatter<i64>,
        >,
    >,
    scaled_axis_descriptor: &[ScaledAxisDescriptor; 2],
    sizes: &PlotSpacing,
) -> Result<(), PlotConstructionError<B::ErrorType>> {
    plot.configure_mesh()
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
        .map_err(PlotConstructionError::InvalidCoordinateSystem)
}

fn draw_into_canvas<B: DrawingBackend>(
    canvas: B,
    data: PlottableData,
    variant: &PlotVariants,
    spacing: &PlotSpacing,
    axis_description: &AxisDescriptors,
) -> Result<(), PlotConstructionError<B::ErrorType>> {
    let area = canvas.into_drawing_area();
    area.fill(&plotters::style::WHITE).unwrap();

    let mut plot = ChartBuilder::on(&area);

    plot.margin_left(spacing.margin[0])
        .margin_top(spacing.margin[1])
        .margin_right(spacing.margin[2])
        .margin_bottom(spacing.margin[3])
        .set_label_area_size(LabelAreaPosition::Left, spacing.label_margin[0])
        .set_label_area_size(LabelAreaPosition::Top, spacing.label_margin[1])
        .set_label_area_size(LabelAreaPosition::Right, spacing.label_margin[2])
        .set_label_area_size(LabelAreaPosition::Bottom, spacing.label_margin[3]);

    match &variant {
        PlotVariants::Histogram(histogram_data) => {
            let histogram = HistogramPlot::new(histogram_data, data, axis_description);
            label_axis(
                histogram.draw_into(&mut plot)?,
                histogram.scale_axis(),
                spacing,
            )?;
        }
        PlotVariants::Scatter => {
            let scatter = ScatterPlot::new(data, axis_description);
            label_axis(scatter.draw_into(&mut plot)?, scatter.scale_axis(), spacing)?;
        }
    }

    area.present().map_err(PlotConstructionError::DrawingError)
}
