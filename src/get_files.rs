//! Retrieve all possible paths as a [`Vec<PathBuf>`] from the given arguments. If a directory path is passed,
//! this function will recursively search for all `.ipynb` notebooks within the directory.
use log::info;
use std::{ffi::OsStr, fs, path::PathBuf};

/// Converts a slice of [`String`] paths into a [`Vec<PathBuf>`] and includes
/// all `.ipynb` files in any directories encountered during the process.
///
/// If any of the paths passed in represent directories, this function will
/// search the directory recursively and add any `.ipynb` files found to the
/// final output.
///
/// # Errors
///
/// This function will return an error in the following situations, but is not limited to just these cases:
/// - The provided path doesn't exist.
/// - The process lacks permissions to view the contents.
/// - The path points at a non-directory file.
pub fn get_paths_from_strings(paths: &[String]) -> Result<Vec<PathBuf>, std::io::Error> {
    let paths = paths
        .iter()
        .map(|path| get_path_from_string(path))
        .collect::<Result<Vec<Vec<PathBuf>>, std::io::Error>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    info!("The following paths are evaluated: {paths:?}");

    Ok(paths)
}

/// Helper function for `get_paths_from_strings`
fn get_path_from_string(path: &str) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut paths = vec![PathBuf::from(path)];
    let mut i = 0;

    while i < paths.len() {
        let path = &paths[i];

        if path.is_dir() {
            let dirs = fs::read_dir(path)?;
            for dir in dirs {
                let dir = dir?;
                let path = dir.path();
                if path.is_dir() || path.extension() == Some(OsStr::new("ipynb")) {
                    paths.push(path);
                }
            }
            paths.remove(i);
        } else {
            i += 1;
        }
    }

    Ok(paths)
}
