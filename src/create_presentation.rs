//! creates a presentation by stitching together the generated pages from a
//! notebook or file.
use anyhow::Result;
use std::{fs::File, io::Write, path::PathBuf};

use crate::notebook::Notebook;

/// This function takes a slice of [`PathBuf`] paths as input. If a given path
/// corresponds to a `.ipynb` file, the function attempts to read it as a
/// notebook and create pages from it.  If the path corresponds to a file of
/// another type, the function reads and passes it in completely.
///
/// # Errors
///
/// This function will return an error if:
/// - `output_path` does not already exist.
/// - the notebook file could not be read or not parsed from json.
/// - either the output or notebook path has no parent.
pub fn collect_pages(output_path: PathBuf, paths: &[PathBuf]) -> Result<Vec<String>> {
    let mut pages = vec![];
    for path in paths {
        if let Some(ext) = path.extension() {
            match ext.to_str() {
                Some("ipynb") => {
                    let notebook = Notebook::try_from_path(path)?;
                    pages.push(notebook.into_pages(&output_path)?);
                }
                _ => {
                    let text = std::fs::read_to_string(path)?;
                    pages.push(text);
                }
            }
        }
    }

    Ok(pages)
}

/// Combines a list of [`String`]s representing one or multiple pages.
///
/// # Errors
///
/// This function will return an error if the content could not write to a file.
pub fn write_presentation(output_path: PathBuf, pages: Vec<String>) -> Result<()> {
    let mut file = File::create(output_path)?;

    for page in pages {
        if !page.is_empty() {
            file.write_all(b"\n\n---\n\n")?;
            file.write_all(page.as_bytes())?
        }
    }
    Ok(())
}
