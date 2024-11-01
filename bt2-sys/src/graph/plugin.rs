use core::ffi;
use std::ffi::CStr;
use std::ptr;

use thiserror::Error;

use crate::error::{BtErrorWrapper, OutOfMemory};
use crate::raw_bindings::{
    bt_plugin, bt_plugin_borrow_filter_component_class_by_name_const,
    bt_plugin_borrow_sink_component_class_by_name_const,
    bt_plugin_borrow_source_component_class_by_name_const, bt_plugin_find, bt_plugin_find_status,
    bt_plugin_get_author, bt_plugin_get_description, bt_plugin_get_license, bt_plugin_get_name,
    bt_plugin_get_path, bt_plugin_get_ref, bt_plugin_get_version, bt_plugin_put_ref,
};
use crate::utils::{BtProperyAvailabilty, ConstNonNull};

use super::component::{
    BtComponentClassFilterConst, BtComponentClassSinkConst, BtComponentClassSourceConst,
};

#[derive(Debug, Error)]
pub enum BtPluginLoadError {
    #[error("Plugin not found.")]
    NotFound,

    #[error(transparent)]
    Memory(#[from] OutOfMemory),

    #[error("Other error caused by: {0}")]
    Other(#[from] BtErrorWrapper),
}

/// Owned representation of a plugin.
///
/// Does not allow modification of the plugin.
#[repr(transparent)]
pub struct BtPlugin(ConstNonNull<bt_plugin>);

impl BtPlugin {
    pub(crate) unsafe fn new_unchecked(ptr: ConstNonNull<bt_plugin>) -> Self {
        Self(ptr)
    }

    pub(crate) fn as_ptr(&self) -> *const bt_plugin {
        self.0.as_ptr()
    }

    pub fn find_anywhere(name: &CStr) -> Result<Self, BtPluginLoadError> {
        let mut plugin = ptr::null();
        unsafe {
            let status: bt_plugin_find_status =
                bt_plugin_find(name.as_ptr(), 1, 1, 1, 1, 1, &mut plugin);
            match status {
                bt_plugin_find_status::BT_PLUGIN_FIND_STATUS_OK => {
                    Ok(Self::new_unchecked(ConstNonNull::new_unchecked(plugin)))
                }
                bt_plugin_find_status::BT_PLUGIN_FIND_STATUS_NOT_FOUND => {
                    Err(BtPluginLoadError::NotFound)
                }
                bt_plugin_find_status::BT_PLUGIN_FIND_STATUS_MEMORY_ERROR => {
                    Err(BtPluginLoadError::Memory(OutOfMemory))
                }
                bt_plugin_find_status::BT_PLUGIN_FIND_STATUS_ERROR => {
                    let error = BtErrorWrapper::get().unwrap();
                    Err(BtPluginLoadError::Other(error))
                }
                _ => unreachable!("Unknown bt_plugin_find_status = {}", status.0),
            }
        }
    }

    /// Get the name of the plugin.
    ///
    /// # Panics
    /// If the name is not valid UTF-8.
    #[must_use]
    pub fn name(&self) -> &str {
        unsafe {
            let name = bt_plugin_get_name(self.as_ptr());
            CStr::from_ptr(name).to_str().unwrap()
        }
    }

    /// Get the description of the plugin.
    ///
    /// # Panics
    /// If the description is not valid UTF-8.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        unsafe {
            let description = bt_plugin_get_description(self.as_ptr());
            if description.is_null() {
                None
            } else {
                Some(CStr::from_ptr(description).to_str().unwrap())
            }
        }
    }

    /// Get the author of the plugin.
    ///
    /// # Panics
    /// If the author is not valid UTF-8.
    #[must_use]
    pub fn author(&self) -> Option<&str> {
        unsafe {
            let author = bt_plugin_get_author(self.as_ptr());
            if author.is_null() {
                None
            } else {
                Some(CStr::from_ptr(author).to_str().unwrap())
            }
        }
    }

    /// Get the licence of the plugin.
    ///
    /// # Panics
    /// If the licence is not valid UTF-8.
    #[must_use]
    pub fn licence(&self) -> Option<&str> {
        unsafe {
            let licence = bt_plugin_get_license(self.as_ptr());
            if licence.is_null() {
                None
            } else {
                Some(CStr::from_ptr(licence).to_str().unwrap())
            }
        }
    }

    /// Get the path of file containing the plugin.
    ///
    /// Note: Static plugins do not have a path.
    ///
    /// # Panics
    /// If the path is not valid UTF-8.
    #[must_use]
    pub fn path(&self) -> Option<&str> {
        unsafe {
            let path = bt_plugin_get_path(self.as_ptr());
            if path.is_null() {
                None
            } else {
                Some(CStr::from_ptr(path).to_str().unwrap())
            }
        }
    }

    /// Get the version of the plugin.
    #[must_use]
    pub fn version(&self) -> Option<BtPluginVersion> {
        let mut major: ffi::c_uint = 0;
        let mut minor: ffi::c_uint = 0;
        let mut patch: ffi::c_uint = 0;
        let mut extra: *const ffi::c_char = ptr::null();
        unsafe {
            let available = bt_plugin_get_version(
                self.as_ptr(),
                &mut major,
                &mut minor,
                &mut patch,
                &mut extra,
            )
            .into();
            match available {
                BtProperyAvailabilty::Available => {
                    let extra = if extra.is_null() {
                        None
                    } else {
                        Some(CStr::from_ptr(extra).to_string_lossy().into_owned())
                    };
                    #[allow(clippy::useless_conversion, reason = "ffi::c_uint might not be u32")]
                    Some(BtPluginVersion {
                        major: major.into(),
                        minor: minor.into(),
                        patch: patch.into(),
                        extra,
                    })
                }
                BtProperyAvailabilty::NotAvailable => None,
            }
        }
    }

    #[must_use]
    pub fn borrow_source_component_class_by_name<'a>(
        &'a self,
        name: &CStr,
    ) -> Option<BtComponentClassSourceConst<'a>> {
        unsafe {
            let ptr =
                bt_plugin_borrow_source_component_class_by_name_const(self.as_ptr(), name.as_ptr());
            if ptr.is_null() {
                None
            } else {
                Some(BtComponentClassSourceConst::new_unchecked(ptr))
            }
        }
    }

    #[must_use]
    pub fn borrow_filter_component_class_by_name<'a>(
        &'a self,
        name: &CStr,
    ) -> Option<BtComponentClassFilterConst<'a>> {
        unsafe {
            let ptr =
                bt_plugin_borrow_filter_component_class_by_name_const(self.as_ptr(), name.as_ptr());
            if ptr.is_null() {
                None
            } else {
                Some(BtComponentClassFilterConst::new_unchecked(ptr))
            }
        }
    }

    #[must_use]
    pub fn borrow_sink_component_class_by_name<'a>(
        &'a self,
        name: &CStr,
    ) -> Option<BtComponentClassSinkConst<'a>> {
        unsafe {
            let ptr =
                bt_plugin_borrow_sink_component_class_by_name_const(self.as_ptr(), name.as_ptr());
            if ptr.is_null() {
                None
            } else {
                Some(BtComponentClassSinkConst::new_unchecked(ptr))
            }
        }
    }
}

impl std::fmt::Debug for BtPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BtPlugin")
            .field("name", &self.name())
            .field("description", &self.description())
            .field("author", &self.author())
            .field("licence", &self.licence())
            .field("path", &self.path())
            .field("version", &self.version())
            .finish()
    }
}

impl std::fmt::Display for BtPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Clone for BtPlugin {
    fn clone(&self) -> Self {
        unsafe {
            bt_plugin_get_ref(self.as_ptr());

            Self::new_unchecked(self.0)
        }
    }
}

impl Drop for BtPlugin {
    fn drop(&mut self) {
        unsafe {
            bt_plugin_put_ref(self.as_ptr());
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BtPluginVersion {
    major: u32,
    minor: u32,
    patch: u32,
    extra: Option<String>,
}

impl std::fmt::Display for BtPluginVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(extra) = &self.extra {
            write!(f, "{}.{}.{}-{}", self.major, self.minor, self.patch, extra)
        } else {
            write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
        }
    }
}
