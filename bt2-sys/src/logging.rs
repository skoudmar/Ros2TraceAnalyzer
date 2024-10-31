use crate::raw_bindings::bt_logging_level;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogLevel {
    /// Low-level debugging context information.
    Trace,

    /// Debugging information, with a higher level of details than the TRACE level.
    Debug,

    /// Informational messages that highlight progress or important states of the application, plugins, or library.
    Info,

    /// Unexpected situations which still allow the execution to continue.
    Warning,

    /// Errors that might still allow the execution to continue.
    Error,

    /// Severe errors that lead the execution to abort immediately.
    Fatal,

    /// Logging is disabled.
    None,
}

impl From<bt_logging_level> for LogLevel {
    fn from(value: bt_logging_level) -> Self {
        match value {
            bt_logging_level::BT_LOGGING_LEVEL_TRACE => Self::Trace,
            bt_logging_level::BT_LOGGING_LEVEL_DEBUG => Self::Debug,
            bt_logging_level::BT_LOGGING_LEVEL_INFO => Self::Info,
            bt_logging_level::BT_LOGGING_LEVEL_WARNING => Self::Warning,
            bt_logging_level::BT_LOGGING_LEVEL_ERROR => Self::Error,
            bt_logging_level::BT_LOGGING_LEVEL_FATAL => Self::Fatal,
            bt_logging_level::BT_LOGGING_LEVEL_NONE => Self::None,
            _ => unreachable!("Bug: unknown bt_logging_level = {}", value.0),
        }
    }
}

impl From<LogLevel> for bt_logging_level {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Trace => bt_logging_level::BT_LOGGING_LEVEL_TRACE,
            LogLevel::Debug => bt_logging_level::BT_LOGGING_LEVEL_DEBUG,
            LogLevel::Info => bt_logging_level::BT_LOGGING_LEVEL_INFO,
            LogLevel::Warning => bt_logging_level::BT_LOGGING_LEVEL_WARNING,
            LogLevel::Error => bt_logging_level::BT_LOGGING_LEVEL_ERROR,
            LogLevel::Fatal => bt_logging_level::BT_LOGGING_LEVEL_FATAL,
            LogLevel::None => bt_logging_level::BT_LOGGING_LEVEL_NONE,
        }
    }
}
