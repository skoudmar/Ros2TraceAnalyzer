/// # Plot Construction Error
///
/// This is an exhaustive list of all errors and exceptions that can be produces during the process
/// of creating a plot of a variable for a specific node in the dependency graph
#[derive(thiserror::Error, Debug)]
pub enum PlotConstructionCommonError {
    /// Plot size too small error
    ///
    /// The requested plot size is too small
    #[error("The requested plot size '{0}x{1}' is too small")]
    PlotSizeTooSmall(u32, u32),

    /// Plot size aspect ratio error
    #[error("The requested plot size ratio '{0}' is too small or too large")]
    PlotSizeRatio(f32),

    /// Error reported by plotters during the construction of the plot
    #[error("Plot construction error")]
    ConstructionError(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("Image encoding error")]
    ImageEncodingError(#[from] image::ImageError),
}

impl<E: std::error::Error + Send + Sync + 'static> From<PlotConstructionError<E>>
    for PlotConstructionCommonError
{
    fn from(value: PlotConstructionError<E>) -> Self {
        PlotConstructionCommonError::ConstructionError(
            Box::new(value) as Box<dyn std::error::Error + Send + Sync>
        )
    }
}

#[derive(thiserror::Error, Debug)]
pub enum PlotConstructionError<BE: std::error::Error + Send + Sync> {
    /// # Drawing Error
    ///
    /// This error is produced during the rendering phase.
    #[error("An error occurred while trying to render the image.")]
    DrawingError(#[source] plotters::drawing::DrawingAreaErrorKind<BE>),

    /// # Invalid Coordinate System
    ///
    /// This error occurs when an invalid coordinates system is requested
    #[error("Invalid coordinate system.")]
    InvalidCoordinateSystem(#[source] plotters::drawing::DrawingAreaErrorKind<BE>),

    /// # Plot Series Error
    ///
    /// This error occurs during the embedment of a series into a plot
    #[error("The series cannot be inserted into the plot.")]
    PlotSeriesError(#[source] plotters::drawing::DrawingAreaErrorKind<BE>),
}
