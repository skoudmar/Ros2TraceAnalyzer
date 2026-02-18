use std::ffi::{CStr, CString};
use std::path::Path;

use bt2_sys::graph::component::BtComponentType;
use bt2_sys::query::support_info;
use color_eyre::eyre::ensure;
use walkdir::WalkDir;

use crate::argsv2::Args;

// Valid trace path should have a weight set to 0.75 so we set the threshold slightly lower.
const TRACE_PATH_LIKELIHOOD_THRESHOLD: f64 = 0.74;

pub fn is_trace_path(path: &CStr) -> bool {
    let support_info_query =
        support_info::Query::new_prepared("ctf", "fs", BtComponentType::Source)
            .expect("Failed to prepare support info query");

    let path_cstr = CString::new(path.to_str().unwrap()).unwrap();

    let result = support_info_query
        .query(bt2_sys::query::SupportInfoParams::Directory(&path_cstr))
        .expect("Failed to query support info");

    result.weight() > TRACE_PATH_LIKELIHOOD_THRESHOLD
}

pub fn find_trace_paths(search_path: &Path) -> Vec<CString> {
    let support_info_query =
        support_info::Query::new_prepared("ctf", "fs", BtComponentType::Source)
            .expect("Failed to prepare support info query");

    let mut trace_paths = Vec::new();
    for dir in WalkDir::new(search_path)
        .into_iter()
        .filter_entry(|e| e.file_type().is_dir())
    {
        let dir = dir.expect("Failed to read directory");
        let path = dir.path();
        let path_cstr = CString::new(path.to_str().unwrap()).unwrap();

        let result = support_info_query
            .query(bt2_sys::query::SupportInfoParams::Directory(&path_cstr))
            .expect("Failed to query support info");

        if result.weight() > TRACE_PATH_LIKELIHOOD_THRESHOLD {
            trace_paths.push(path_cstr);
        }
    }

    trace_paths
}

pub fn prepare_trace_paths() -> color_eyre::Result<Vec<CString>> {
    let trace_paths: Vec<_> = if Args::get_analyses_args().is_exact_path() {
        Args::get_analyses_args()
            .trace_paths_cstring()
            .into_iter()
            .filter_map(|path| {
                if is_trace_path(&path) {
                    Some(path)
                } else {
                    None
                }
            })
            .collect()
    } else {
        Args::get_analyses_args()
            .trace_paths()
            .iter()
            .map(AsRef::as_ref)
            .flat_map(find_trace_paths)
            .collect()
    };

    ensure!(
        !trace_paths.is_empty(),
        "No traces found in the provided paths."
    );

    println!("Found traces:");
    for path in &trace_paths {
        println!("  {}", path.to_string_lossy());
    }

    Ok(trace_paths)
}
