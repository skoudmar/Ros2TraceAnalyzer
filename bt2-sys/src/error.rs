use std::ffi::{CStr, CString};
use std::fmt::Display;

use thiserror::Error;

use crate::iterator::MessageIteratorState;
use crate::raw_bindings::{
    bt_current_thread_error_append_cause_from_unknown, bt_current_thread_move_error,
    bt_current_thread_take_error, bt_error, bt_error_borrow_cause_by_index,
    bt_error_cause_get_message, bt_error_get_cause_count, bt_graph_run_once_status,
    bt_message_iterator_next_status,
};
use crate::utils::ConstNonNull;

pub type BtResult<T> = Result<T, BtError>;

#[macro_export]
macro_rules! throw {
    ($message:expr) => {
        $crate::error::BtError::__throw_new($message, file!(), line!());
    };
}

#[macro_export]
macro_rules! rethrow {
    ($error:expr, $message:expr) => {
        $crate::error::BtError::__rethrow($error, $message, file!(), line!());
    };
}

#[derive(Debug, Error)]
#[error("Out of memory")]
pub struct OutOfMemory;

#[derive(Debug, Error)]
pub enum BtError {
    #[error("Iteration ended")]
    End,
    #[error("Try again")]
    Again,
    #[error("Babeltrace2 error: {}", .0)]
    MemoryError(#[from] OutOfMemory),
    #[error("Babeltrace2 error: {}", .0)]
    Error(BtErrorWrapper),
}

impl BtError {
    const MODULE: &CStr = c"Rust Error";

    fn get_error() -> Option<BtError> {
        Some(Self::Error(BtErrorWrapper::get()?))
    }

    #[doc(hidden)]
    pub fn __throw_new(message: &str, file: &str, line: u32) {
        eprintln!("[{file}:{line}] {message}");
        Self::throw_common(None, message, file, line);
    }

    #[doc(hidden)]
    pub fn __rethrow(this: Self, message: &str, file: &str, line: u32) {
        eprintln!("[{}:{}] {}: {}", file, line, message, &this);
        match this {
            Self::Error(error) => {
                Self::throw_common(Some(error), message, file, line);
            }
            _ => {
                Self::throw_common(None, message, file, line);
            }
        }
    }

    fn throw_common(cause: Option<BtErrorWrapper>, message: &str, file: &str, line: u32) {
        let message = CString::new(message).unwrap();
        let file = CString::new(file).unwrap();

        unsafe {
            if let Some(cause) = cause {
                // Return taken error back to library
                bt_current_thread_move_error(cause.as_ptr());
            }

            bt_current_thread_error_append_cause_from_unknown(
                Self::MODULE.as_ptr(),
                file.as_ptr(),
                line.into(),
                message.as_ptr(),
            );
        }
    }
}

#[derive(Error)]
pub struct BtErrorWrapper(ConstNonNull<bt_error>);

impl BtErrorWrapper {
    pub(crate) fn get() -> Option<Self> {
        unsafe { bt_current_thread_take_error().try_into().ok().map(Self) }
    }

    #[inline]
    pub(crate) fn as_ptr(&self) -> *const bt_error {
        self.0.as_ptr()
    }
}

impl std::fmt::Debug for BtErrorWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut structure = f.debug_struct("BtErrorWrapper");
        unsafe {
            let cause_count = bt_error_get_cause_count(self.as_ptr());
            for cause_idx in 0..cause_count {
                let cause = bt_error_borrow_cause_by_index(self.as_ptr(), cause_idx);
                let message = bt_error_cause_get_message(cause);
                let message_cstr = CStr::from_ptr(message);

                structure.field(&cause_idx.to_string(), &message_cstr);
            }
        }

        structure.finish()
    }
}

impl Display for BtErrorWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            let cause_count = bt_error_get_cause_count(self.as_ptr());
            for cause_idx in 0..cause_count {
                let cause = bt_error_borrow_cause_by_index(self.as_ptr(), cause_idx);
                let message = bt_error_cause_get_message(cause);
                let message_cstr = CStr::from_ptr(message);

                writeln!(f, "{}", message_cstr.to_string_lossy())?;
            }
        }

        Ok(())
    }
}

pub(crate) trait IntoResult<T> {
    fn into_result(self) -> BtResult<T>;
}

impl IntoResult<()> for bt_message_iterator_next_status {
    fn into_result(self) -> BtResult<()> {
        match self {
            Self::BT_MESSAGE_ITERATOR_NEXT_STATUS_OK => Ok(()),
            Self::BT_MESSAGE_ITERATOR_NEXT_STATUS_END => Err(BtError::End),
            Self::BT_MESSAGE_ITERATOR_NEXT_STATUS_AGAIN => Err(BtError::Again),
            Self::BT_MESSAGE_ITERATOR_NEXT_STATUS_MEMORY_ERROR => {
                Err(BtError::MemoryError(OutOfMemory))
            }
            Self::BT_MESSAGE_ITERATOR_NEXT_STATUS_ERROR => Err(BtError::get_error().unwrap()),
            status => unreachable!(
                "Bug: unknown bt_message_iterator_next_status = {}",
                status.0
            ),
        }
    }
}

impl IntoResult<MessageIteratorState> for bt_graph_run_once_status {
    fn into_result(self) -> BtResult<MessageIteratorState> {
        match self {
            Self::BT_GRAPH_RUN_ONCE_STATUS_OK => Ok(MessageIteratorState::Running),
            Self::BT_GRAPH_RUN_ONCE_STATUS_END => Ok(MessageIteratorState::Ended),
            Self::BT_GRAPH_RUN_ONCE_STATUS_AGAIN => Err(BtError::Again),
            Self::BT_GRAPH_RUN_ONCE_STATUS_MEMORY_ERROR => Err(BtError::MemoryError(OutOfMemory)),
            Self::BT_GRAPH_RUN_ONCE_STATUS_ERROR => Err(BtError::get_error().unwrap()),
            status => unreachable!("Bug: unknown bt_graph_run_once_status = {}", status.0),
        }
    }
}
