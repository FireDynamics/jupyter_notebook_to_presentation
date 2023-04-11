//! This program creates a presentation file from `.ipynb` notebook files. For
//! a more in-depth definition, please refer to the `README.md` file.

#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]
#![warn(clippy::missing_errors_doc)]
#![warn(clippy::missing_panics_doc)]
#![warn(clippy::doc_markdown)]

mod arguments;
mod commands;
mod create_presentation;
mod get_files;
mod notebook;
mod path;

use anyhow::Result;
use arguments::get_arguments;
use log::{error, LevelFilter};
use simple_logger::SimpleLogger;
use std::{path::PathBuf, str::FromStr};

fn main() {
    let run = run();
    match run {
        Ok(_) => {
            println!("The presentation was successfully created.")
        }
        Err(err) => {
            error!("{}", err)
        }
    }
}

/// Run the program and return an error if any occurs.
///
/// # Errors
///
/// This function will return an error if any intern system fails. Normally
/// only, when a tag was wrongly defined.
fn run() -> Result<()> {
    let args = match get_arguments() {
        Ok(ok) => ok,
        Err(err) => {
            println!("{err}");
            return Err(err);
        }
    };

    SimpleLogger::new()
        .with_level(LevelFilter::Off)
        .with_module_level(
            "presentation",
            if matches!(args.verbose, true) {
                LevelFilter::Info
            } else {
                LevelFilter::Warn
            },
        )
        .init()?;

    let output_path = PathBuf::from_str(&args.output)?;
    if !args.force && output_path.is_file() && output_path.exists() {
        return Err(anyhow::Error::msg(format!(
            r#"File already exist {:?}. Use "-f" to force an override."#,
            output_path
        )));
    }

    let paths = get_files::get_paths_from_strings(&args.input)?;
    let pages = create_presentation::collect_pages(PathBuf::from_str(&args.output)?, &paths)?;
    let output_path = PathBuf::from_str(&args.output)?;
    create_presentation::write_presentation(output_path, pages)?;

    Ok(())
}
