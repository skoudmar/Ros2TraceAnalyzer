use std::convert::identity;
use std::io::Write;

use plotters::chart::{ChartBuilder, ChartContext, LabelAreaPosition};
use plotters::coord::ranged1d::ValueFormatter;
use plotters::prelude::{BitMapBackend, Cartesian2d, DrawingBackend, IntoDrawingArea, Ranged};
use plotters_svg::SVGBackend;

use crate::argsv2::plot_args::{PlotOutputFormat, PlotRequest, PlotVariants};
use crate::extract::PlottableData;
use crate::plotting::axis_descriptor::{ScaledAxisDescriptor, resolve_axis_descriptors};
use crate::plotting::error::{PlotConstructionCommonError, PlotConstructionError};
use crate::plotting::plots::PlotData;
use crate::plotting::plots::histogram::HistogramPlot;
use crate::plotting::plots::scatter::ScatterPlot;

mod axis_descriptor;
mod error;
mod plots;

pub fn render_plot(
    output: &mut std::io::BufWriter<dyn std::io::Write>,
    plotting_data: PlottableData,
    plot_request: &PlotRequest,
    output_format: PlotOutputFormat,
) -> Result<(), PlotConstructionCommonError> {
    let (width, height) = plot_request.size;

    match output_format {
        PlotOutputFormat::Svg => {
            let mut out = String::new();
            draw_into_canvas(
                SVGBackend::with_string(&mut out, (width, height)),
                plotting_data,
                plot_request,
            )?;
            output.write_all(out.as_bytes())?;
        }
        PlotOutputFormat::Png => {
            let mut buffer = vec![0u8; (width * height * 3) as usize];
            draw_into_canvas(
                BitMapBackend::with_buffer(&mut buffer, (width, height)),
                plotting_data,
                plot_request,
            )?;

            use image::ImageEncoder;
            use image::codecs::png::PngEncoder;

            let img_encoder = PngEncoder::new(&mut *output);
            img_encoder.write_image(&buffer, width, height, image::ColorType::Rgb8)?;
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

    pub y_label_size: i32,
    pub x_label_size: i32,

    /// Font size of the axis description
    pub desc_size: i32,

    /// Font size of the title
    title_size: i32,
}

impl PlotSpacing {
    fn apply_to<B: DrawingBackend>(&self, builder: &mut ChartBuilder<B>) {
        builder
            .margin_left(self.margin[0])
            .margin_top(self.margin[1])
            .margin_right(self.margin[2])
            .margin_bottom(self.margin[3])
            .set_label_area_size(LabelAreaPosition::Left, self.label_margin[0])
            .set_label_area_size(LabelAreaPosition::Top, self.label_margin[1])
            .set_label_area_size(LabelAreaPosition::Right, self.label_margin[2])
            .set_label_area_size(LabelAreaPosition::Bottom, self.label_margin[3]);
    }
}

impl From<(u32, u32)> for PlotSpacing {
    fn from(value: (u32, u32)) -> Self {
        match value {
            (..400, _) | (_, ..400) => PlotSpacing {
                margin: [16; 4],
                label_margin: [32, 0, 0, 32],
                y_label_size: 12,
                x_label_size: 12,
                desc_size: 14,
                title_size: 24,
            },
            (400..800, 400..800) => PlotSpacing {
                margin: [16; 4],
                label_margin: [48, 0, 0, 48],
                y_label_size: 12,
                x_label_size: 12,
                desc_size: 20,
                title_size: 24,
            },
            (800.., _) | (_, 800..) => PlotSpacing {
                margin: [32; 4],
                label_margin: [82, 0, 0, 64],
                y_label_size: 20,
                x_label_size: 20,
                desc_size: 32,
                title_size: 36,
            },
        }
    }
}

fn format_tick(desc: &ScaledAxisDescriptor, v: i64) -> String {
    format!("{:.2}", desc.convert(v))
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn label_axis<B: DrawingBackend, T: Copy>(
    mut plot: ChartContext<
        '_,
        B,
        Cartesian2d<
            impl Ranged<ValueType = T> + ValueFormatter<T>,
            impl Ranged<ValueType = i64> + ValueFormatter<i64>,
        >,
    >,
    scaled_axis_descriptor: &[ScaledAxisDescriptor; 2],
    sizes: &PlotSpacing,
    fmap: impl Fn(T) -> i64,
) -> Result<(), PlotConstructionError<B::ErrorType>> {
    plot.configure_mesh()
        .max_light_lines(1)
        .x_desc(scaled_axis_descriptor[0].name())
        .y_desc(scaled_axis_descriptor[1].name())
        .x_label_formatter(&|v| format_tick(&scaled_axis_descriptor[0], fmap(*v)))
        .y_label_formatter(&|v| format_tick(&scaled_axis_descriptor[1], *v))
        .axis_desc_style(("sans-serif", sizes.desc_size))
        .y_label_style(("sans-serif", sizes.y_label_size))
        .x_label_style(("sans-serif", sizes.x_label_size))
        .draw()
        .map_err(PlotConstructionError::InvalidCoordinateSystem)
}

fn draw_into_canvas<B: DrawingBackend>(
    canvas: B,
    data: PlottableData,
    plot_request: &PlotRequest,
) -> Result<(), PlotConstructionError<B::ErrorType>> {
    let spacing = PlotSpacing::from(plot_request.size);
    let axis_description = resolve_axis_descriptors(plot_request.property, &plot_request.plot);

    let area = canvas.into_drawing_area();
    area.fill(&plotters::style::WHITE)
        .map_err(PlotConstructionError::DrawingError)?;

    let mut plot = ChartBuilder::on(&area);
    spacing.apply_to(&mut plot);

    if plot_request.include_title {
        plot.caption(&data.title, ("sans-serif", spacing.title_size));
    }

    match &plot_request.plot {
        PlotVariants::Histogram(histogram_data) => {
            let histogram = HistogramPlot::new(histogram_data, &data.data, &axis_description);
            label_axis(
                histogram.draw_into(&mut plot)?,
                histogram.scale_axis(),
                &spacing,
                |v| v as i64,
            )?;
        }
        PlotVariants::Scatter => {
            let scatter = ScatterPlot::new(&data.data, &axis_description);
            label_axis(
                scatter.draw_into(&mut plot)?,
                scatter.scale_axis(),
                &spacing,
                identity,
            )?;
        }
    }

    area.present().map_err(PlotConstructionError::DrawingError)
}
