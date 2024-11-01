use std::ffi::CStr;
use std::ptr::{self, NonNull};
use std::string::ToString;

use thiserror::Error;

use crate::error::{BtErrorWrapper, OutOfMemory, TryAgain};
use crate::graph::component::BtComponentClassConst;
use crate::logging::LogLevel;
use crate::raw_bindings::{
    bt_query_executor, bt_query_executor_create, bt_query_executor_get_logging_level,
    bt_query_executor_put_ref, bt_query_executor_query, bt_query_executor_query_status,
    bt_query_executor_set_logging_level,
};
use crate::utils::Const;
use crate::value::{BtValue, BtValueMap, BtValueRealConst, BtValueStringConst, BtValueTypedConst};

pub struct BtQueryExecutor(NonNull<bt_query_executor>);

#[derive(Debug, Error)]
pub enum BtQueryError {
    #[error("Unknown object to query.")]
    UnknownObject,

    #[error(transparent)]
    Memory(#[from] OutOfMemory),

    #[error(transparent)]
    TryAgain(#[from] TryAgain),

    #[error("Other error caused by: {0}")]
    Other(#[from] BtErrorWrapper),
}

impl BtQueryExecutor {
    pub(crate) unsafe fn new_unchecked(ptr: NonNull<bt_query_executor>) -> Self {
        Self(ptr)
    }

    fn as_ptr(&self) -> *mut bt_query_executor {
        self.0.as_ptr()
    }

    #[must_use]
    pub fn new(component: &BtComponentClassConst, object_name: &CStr, params: &BtValue) -> Self {
        unsafe {
            let ptr =
                bt_query_executor_create(component.as_ptr(), object_name.as_ptr(), params.as_ptr());
            Self::new_unchecked(NonNull::new_unchecked(ptr))
        }
    }

    pub fn set_logging_level(&mut self, level: LogLevel) {
        unsafe {
            bt_query_executor_set_logging_level(self.as_ptr(), level.into());
        }
    }

    #[must_use]
    pub fn get_log_level(&self) -> LogLevel {
        unsafe { bt_query_executor_get_logging_level(self.as_ptr()).into() }
    }

    /// Execute the query.
    ///
    /// # Errors
    /// - `UnknownObject`: The object to query is unknown.
    /// - `Memory`: Out of memory.
    /// - `TryAgain`: The query should be retried.
    /// - `Other`: Other error.
    pub fn query(&mut self) -> Result<Const<BtValue>, BtQueryError> {
        let mut result = ptr::null();
        unsafe {
            let status = bt_query_executor_query(self.as_ptr(), &mut result);
            match status {
                bt_query_executor_query_status::BT_QUERY_EXECUTOR_QUERY_STATUS_OK => {
                    // Safety: result is guaranteed to be non-null by C API.
                    // The BtValue is valid only for const access.
                    Ok(Const::new(BtValue::new_unchecked(result.cast_mut())))
                }
                bt_query_executor_query_status::BT_QUERY_EXECUTOR_QUERY_STATUS_AGAIN => {
                    Err(BtQueryError::TryAgain(TryAgain))
                }
                bt_query_executor_query_status::BT_QUERY_EXECUTOR_QUERY_STATUS_UNKNOWN_OBJECT => {
                    Err(BtQueryError::UnknownObject)
                }
                bt_query_executor_query_status::BT_QUERY_EXECUTOR_QUERY_STATUS_MEMORY_ERROR => {
                    Err(BtQueryError::Memory(OutOfMemory))
                }
                bt_query_executor_query_status::BT_QUERY_EXECUTOR_QUERY_STATUS_ERROR => {
                    Err(BtQueryError::Other(
                        BtErrorWrapper::get().expect("Error should be set on error status."),
                    ))
                }
                _ => unreachable!("Unknown query status: {}", status.0),
            }
        }
    }
}

impl Drop for BtQueryExecutor {
    fn drop(&mut self) {
        unsafe {
            bt_query_executor_put_ref(self.as_ptr());
        }
    }
}

pub enum SupportInfoParams<'a> {
    String(&'a CStr),
    File(&'a CStr),
    Directory(&'a CStr),
}

impl<'a> SupportInfoParams<'a> {
    fn input(&self) -> &CStr {
        match self {
            SupportInfoParams::File(s)
            | SupportInfoParams::Directory(s)
            | SupportInfoParams::String(s) => s,
        }
    }

    fn typ(&self) -> &'static CStr {
        match self {
            SupportInfoParams::String(_) => c"string",
            SupportInfoParams::File(_) => c"file",
            SupportInfoParams::Directory(_) => c"directory",
        }
    }
}

impl<'a> TryFrom<SupportInfoParams<'a>> for BtValueMap {
    type Error = OutOfMemory;

    fn try_from(params: SupportInfoParams<'a>) -> Result<Self, Self::Error> {
        let mut map = Self::new()?;
        map.insert_string_with_cstr_key_val(c"type", params.typ())?;
        map.insert_string_with_cstr_key_val(c"input", params.input())?;
        Ok(map)
    }
}

#[derive(Debug, Clone)]
pub struct SupportInfoResult {
    weight: f64,
    group: Option<String>,
}

impl SupportInfoResult {
    #[must_use]
    pub fn weight(&self) -> f64 {
        self.weight
    }

    #[must_use]
    pub fn group(&self) -> Option<&str> {
        self.group.as_deref()
    }
}

#[derive(Debug, Error)]
#[error("Incorrect layout of BtValue.")]
pub enum SupportInfoResultError {
    #[error("Incorrect layout of BtValue.")]
    IncorrectLayout,

    #[error("Group cannot be converted to a string.")]
    GroupConversion(#[from] std::str::Utf8Error),

    #[error("Support info query failed. Caused by: {0}")]
    QueryError(#[from] BtQueryError),

    #[error("Not supported.")]
    NotSupported,
}

impl TryFrom<Const<BtValue>> for SupportInfoResult {
    type Error = SupportInfoResultError;

    fn try_from(value: Const<BtValue>) -> Result<Self, Self::Error> {
        let value = value.as_const();
        match value.cast() {
            BtValueTypedConst::Real(real) => Ok(SupportInfoResult {
                weight: real.get(),
                group: None,
            }),
            BtValueTypedConst::Map(map) => {
                let weight = map
                    .get_with_cstr_key(c"weight")
                    .ok_or(SupportInfoResultError::IncorrectLayout)?;
                let weight = BtValueRealConst::try_from(weight)
                    .map_err(|_| SupportInfoResultError::IncorrectLayout)?
                    .get();

                let group = map
                    .get_with_cstr_key(c"group")
                    .map(|v| {
                        BtValueStringConst::try_from(v)
                            .map_err(|_| SupportInfoResultError::IncorrectLayout)
                    })
                    .transpose()?
                    .map(|v| v.get().map(ToString::to_string))
                    .transpose()?;

                Ok(SupportInfoResult { weight, group })
            }
            _ => Err(SupportInfoResultError::IncorrectLayout),
        }
    }
}