/// # Chart Construction Error
///
/// This is an exhaustive list of all errors and exceptions that can be produces during the process
/// of creating a chart of a variable for a specific node in the dependency graph
#[derive(thiserror::Error, Debug)]
pub enum ChartConstructionError {
    /// # Drawing Error
    ///
    /// This error is produced during the rendering phase. It is a wrapper around a generic
    /// ```plotters::drawing::DrawingAreaErrorKind<_>``` error
    #[error("An error occurred while trying to render the image\n{0}")]
    DrawingError(String),

    /// # Invalid Coordinate System
    ///
    /// This error occurs when an invalid coordinates system is requested
    #[error("Invalid coordinate system\n{0}")]
    InvalidCoordinateSystem(String),

    /// # Chart Series Error
    ///
    /// This error occurs during the embedment of a series into a chart
    #[error("The series cannot be inserted into the chart\n{0}")]
    ChartSeriesError(String),

    /// # Chart size too small error
    ///
    /// The requested chart size is too small
    #[error("The requested chart size '{0}x{1}' is too small")]
    ChartSizeTooSmall(i32, i32),

    /// # Chart size aspect ratio error
    ///
    /// The requested chart size ratio is too small or too large
    #[error("The requested chart size ratio '{0}' is too small or too large")]
    ChartSizeRatio(f32),
}
