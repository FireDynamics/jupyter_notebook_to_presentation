//! Get all passed in arguments

use anyhow::Result;
use arg::{self, Args};
use std::env;

// The dock comments above and in this struct are automatically converted to
// the description when running the program.

///Create a presentation from passed `.ipynb` notebooks.
#[derive(Args, Debug)]
pub struct Arguments {
    ///The path where the presentation will be saved.
    #[arg(short = "o", long, required)]
    pub output: String,

    ///Force override the file if it already exists.
    #[arg(short = "f", long)]
    pub force: bool,

    ///Enable verbose output.
    #[arg(short = "v", long)]
    pub verbose: bool,

    ///Enable debug output.
    #[arg(short = "d", long)]
    pub debug: bool,

    ///The source paths of the notebooks or folders.
    pub input: Vec<String>,
}

/// Get all passed in arguments.
///
/// # Errors
///
/// This function will return an error if help is requested or a an argument
/// that is not supported was passed.
pub fn get_arguments() -> Result<Arguments> {
    let args = env::args().collect::<Vec<String>>().join(" ");
    let args = Arguments::from_text(&args);

    match args {
        Ok(args) => Ok(args),
        Err(err) => Err(anyhow::Error::msg(err.to_string())),
    }
}
