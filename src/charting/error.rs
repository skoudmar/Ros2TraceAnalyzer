/// # Chart Construction Error
///
/// This is an exhaustive list of all errors and exceptions that can be produces during the process
/// of creating a chart of a variable for a specific node in the dependency graph
#[derive(thiserror::Error, Debug)]
pub enum ChartConstructionCommonError {
    /// Chart size too small error
    ///
    /// The requested chart size is too small
    #[error("The requested chart size '{0}x{1}' is too small")]
    ChartSizeTooSmall(u32, u32),

    /// Chart size aspect ratio error
    #[error("The requested chart size ratio '{0}' is too small or too large")]
    ChartSizeRatio(f32),

    /// Error reported by plotters during the construction of the chart
    #[error("Chart construction error")]
    ConstructionError(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl<E: std::error::Error + Send + Sync + 'static> From<ChartConstructionError<E>>
    for ChartConstructionCommonError
{
    fn from(value: ChartConstructionError<E>) -> Self {
        ChartConstructionCommonError::ConstructionError(
            Box::new(value) as Box<dyn std::error::Error + Send + Sync>
        )
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ChartConstructionError<BE: std::error::Error + Send + Sync> {
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

    /// # Chart Series Error
    ///
    /// This error occurs during the embedment of a series into a chart
    #[error("The series cannot be inserted into the chart.")]
    ChartSeriesError(#[source] plotters::drawing::DrawingAreaErrorKind<BE>),
}
