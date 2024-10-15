use std::{
    ffi::{CStr, CString},
    fmt::{Display, Error, Write},
};

use thiserror::Error;

use crate::{iterator::MessageIteratorState, raw_bindings::{
    bt_current_thread_error_append_cause_from_unknown, bt_current_thread_move_error, bt_current_thread_take_error, bt_error, bt_error_borrow_cause_by_index, bt_error_cause_get_message, bt_error_get_cause_count, bt_graph_run_once_status, bt_message_iterator_next_status
}};

pub type BtResult<T> = Result<T, BtError>;

#[macro_export]
macro_rules! throw_error {
    ($message:expr) => {
        BtError::throw_new($message, file!(), line!() as u64);
    };
}

#[macro_export]
macro_rules! rethrow_error {
    ($error:expr, $message:expr) => {
        $error.rethrow($message, file!(), line!() as u64);
    };
}

#[derive(Debug, Error)]
pub enum OutOfMemoryError {
    #[error("Out of memory")]
    OutOfMemory,
}

#[derive(Debug, Error)]
pub enum BtError {
    #[error("Iteration ended")]
    End,
    #[error("Try again")]
    Again,
    #[error("Babeltrace2 error: {}", .0)]
    MemoryError(OutOfMemoryError),
    #[error("Babeltrace2 error: {}", .0)]
    Error(BtErrorWrapper),
}

impl BtError {
    fn get_error() -> Option<BtError> {
        Some(Self::Error(BtErrorWrapper::get()?))
    }

    pub(crate) fn throw_new(message: &str, file: &str, line: u64) {
        eprintln!("[{}:{}] {}", file, line, message);

        const MODULE: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"Rust Error\0") };
        let message = CString::new(message).unwrap();
        let file = CString::new(file).unwrap();
        unsafe {
            bt_current_thread_error_append_cause_from_unknown(
                MODULE.as_ptr(),
                file.as_ptr(),
                line,
                message.as_ptr(),
            );
        }
    }

    pub(crate) fn rethrow(self, message: &str, file: &str, line: u64) {
        eprintln!("[{}:{}] {}: {}", file, line, message, &self);
        match self {
            Self::Error(error) => {
                const MODULE: &CStr =
                    unsafe { CStr::from_bytes_with_nul_unchecked(b"Rust Error\0") };
                let message = CString::new(message).unwrap();
                let file = CString::new(file).unwrap();
                unsafe {
                    bt_current_thread_move_error(error.0);
                    bt_current_thread_error_append_cause_from_unknown(
                        MODULE.as_ptr(),
                        file.as_ptr(),
                        line,
                        message.as_ptr(),
                    );
                }
            }
            _ => {
                Self::throw_new(message, file, line);
            }
        }
    }
}

#[derive(Debug, Error)]
pub struct BtErrorWrapper(*const bt_error);

impl BtErrorWrapper {
    pub(crate) fn get() -> Option<Self> {
        unsafe {
            let error = bt_current_thread_take_error();
            if error.is_null() {
                return None;
            }

            Some(Self(error))
        }
    }
}

impl Display for BtErrorWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            let cause_count = bt_error_get_cause_count(self.0);
            for cause_idx in 0..cause_count {
                let cause = bt_error_borrow_cause_by_index(self.0, cause_idx);
                let message = bt_error_cause_get_message(cause);
                let message_cstr = CStr::from_ptr(message);

                let Ok(message_str) = message_cstr.to_str() else {
                    eprintln!("failed to convert error message");
                    return Err(Error);
                };
                f.write_str(message_str)?;
                f.write_char('\n')?;
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
            Self::BT_MESSAGE_ITERATOR_NEXT_STATUS_MEMORY_ERROR => Err(BtError::MemoryError(OutOfMemoryError::OutOfMemory)),
            Self::BT_MESSAGE_ITERATOR_NEXT_STATUS_ERROR => Err(BtError::get_error().unwrap()),
        }
    }
}

impl IntoResult<MessageIteratorState> for bt_graph_run_once_status {
    fn into_result(self) -> BtResult<MessageIteratorState> {
        match self {
            Self::BT_GRAPH_RUN_ONCE_STATUS_OK => Ok(MessageIteratorState::Running),
            Self::BT_GRAPH_RUN_ONCE_STATUS_END => Ok(MessageIteratorState::Ended),
            Self::BT_GRAPH_RUN_ONCE_STATUS_AGAIN => Err(BtError::Again),
            Self::BT_GRAPH_RUN_ONCE_STATUS_MEMORY_ERROR => Err(BtError::MemoryError(OutOfMemoryError::OutOfMemory)),
            Self::BT_GRAPH_RUN_ONCE_STATUS_ERROR => Err(BtError::get_error().unwrap()),
        }
    }
}
