use std::ptr::NonNull;

use crate::raw_bindings::bt_plugin;

pub struct BtPluginConst(*const bt_plugin);
pub struct BtPlugin(NonNull<bt_plugin>);
