//! Load and read a `.ipynb` notebook with `serde` and apply the assigned tags.
use anyhow::Result;
use chumsky::prelude::*;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fmt::Display,
    fs,
    path::{Path, PathBuf},
};

use crate::path::{replace_paths, wrap_image};

/// This struct represents the metadata of a notebook cell. The `tags` property
/// is used to execute the commands defined by the tags.
#[derive(Serialize, Deserialize, Debug)]
struct Metadata {
    /// The tags in a cell represented as `String`
    tags: Option<Vec<String>>,
}

/// The output types of a notebook cell, including only the
/// necessary properties.
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum Output {
    /// Output for when a code cell fails.
    Error {
        /// Kind off the Error.
        ename: String,
        /// Further description of the error.
        evalue: String,
    },
    /// Output of a cell when it writes to the io stream.
    Stream {
        /// The content of the stream.
        text: Vec<String>,
    },
    /// Every other output type, wich are ignored in this program. Needs to be
    /// defined to withheld errors caused by `serde` not finding a fitting
    /// enum variant to parse to.
    Other {},
}

/// Representation of single cell of a notebook, including only the
/// necessary properties, wich are automatically populated by the crate `serde`.
#[derive(Serialize, Deserialize, Debug)]
struct Cell {
    /// Type of the cell (e.g. Markdown or code)
    cell_type: String,
    /// The metadata bounded to the cell. Relevant are only the tags.
    metadata: Metadata,
    /// Possible outputs of a cell, e.g. an error of a code cell.
    outputs: Option<Vec<Output>>,
    /// The content of a cell.
    source: Vec<String>,
}
impl Cell {
    /// TODO
    fn get_source_without_commands_comment(&self) -> Result<String> {
        match self.cell_type.as_str() {
            "markdown" => todo!(),
            "code" => todo!(),
            _ => Err(anyhow::Error::msg(format!(
                "Cell type '{}' is currently not supported.",
                self.cell_type
            ))),
        }
    }

    /// TODO
    fn prosses_to_presentation() -> Result<()> {
        todo!()
    }

    /// Returns the source, so the user defined input of this [`Cell`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the `cell_type` type is not
    /// supported.
    fn get_source(&self) -> Result<String> {
        match self.cell_type.as_str() {
            "markdown" => Ok(self.source.join("")),
            "code" => Ok(format!(
                "```Python\n{}\n```",
                self.source.join("").trim_end()
            )),
            _ => Err(anyhow::Error::msg(format!(
                "Cell type '{}' is currently not supported.",
                self.cell_type
            ))),
        }
    }

    /// Returns the stream of this [`Cell`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the [`Cell`] has no stream output.
    fn get_stream(&self) -> Result<String> {
        if let Some(outputs) = &self.outputs {
            for output in outputs {
                if let Output::Stream { text } = output {
                    return Ok(format!("```\n{}\n```", text.join("").trim_end()));
                }
            }
        } else {
            return Err(anyhow::Error::msg("No output in cell.".to_string()));
        }
        Err(anyhow::Error::msg("No stream in Cell".to_string()))
    }

    /// Returns the error of this [`Cell`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the [`Cell`] has no error output.
    fn get_error(&self) -> Result<String> {
        if let Some(outputs) = &self.outputs {
            for output in outputs {
                if let Output::Error { ename, evalue } = output {
                    return Ok(format!("```\n{}: {}\n```", ename, evalue));
                }
            }
        } else {
            return Err(anyhow::Error::msg("No output in cell.".to_string()));
        }
        Err(anyhow::Error::msg("No error in Cell".to_string()))
    }
}

/// This function removes extraneous and sensitive data from an error message
/// displayed in a notebook. Note that this function is deprecated, as the
/// formatting of the `traceback` property can vary depending on how the
/// notebook was compiled. As a result, only the error message from the
/// `evalue` property is now displayed, which requires no further processing.
///
/// # Errors
/// An error will be returned if the internal parse function has been
/// improperly configured. However, no errors should occur if it is properly
/// configured.
#[deprecated(note = "please use `new_method` instead")]
pub fn _remove_theming_from_err(text: &str) -> Result<String> {
    let color = just::<_, _, Simple<char>>("\u{001b}[")
        .then(take_until(just('m')))
        .ignored();
    let html = take_until(just("</a>").rewind())
        .ignored()
        .delimited_by(just("<a"), just("</a>"));

    let parser = take_until(color.or(html))
        .map(|(s, _)| s)
        .collect::<String>()
        .repeated()
        .map(|f| f)
        .chain::<String, String, _>(take_until(end()).map(|(s, _)| s).collect::<String>())
        .collect::<String>();

    // HACK clippy throws an error when using parser.parse(...)
    match Parser::parse(&parser, text) {
        Ok(ok) => Ok(ok),
        Err(err) => Err(anyhow::Error::msg(format!("{err:?}"))),
    }
}

/// Representation of a whole `.ipynb` notebook containing the parsed file and
/// a path to the file.
#[derive(Serialize, Deserialize, Debug)]
pub struct Notebook {
    /// All [`Cell`]s in the notebook
    cells: Vec<Cell>,
    #[serde(skip)]
    /// The path to the notebook.
    path: PathBuf,
}

impl Notebook {
    /// Converts the whole [`Notebook`] to pages for the presentation.
    ///
    /// # Errors
    ///
    /// This function will return an error if either the output or notebook
    /// path has no parent. Note this case should never happen.
    pub fn into_pages(self, output_path: &Path) -> Result<String> {
        let mut pages = vec![];

        let mut page_class = None;

        for (n, cell) in self.cells.into_iter().enumerate() {
            if let Some(tags) = &cell.metadata.tags {
                for tag in tags {
                    match Tag::try_from(tag.as_str()) {
                        Ok(ok) => match ok {
                            Tag::NewPage => {
                                if let Some(class) = page_class {
                                    if let Some(last) = pages.last_mut() {
                                        *last = format!("class: {}\n{}", class, last);
                                    } else {
                                        warn!("Tried to set a class page that was not initialized. <Cell: {} in File: {:?}>", n, self.path);
                                    }
                                }

                                page_class = None;
                                pages.push(String::default());
                            }
                            Tag::AddToPage => {
                                if let Some(last) = pages.last_mut() {
                                    match cell.get_source() {
                                        Ok(text) => {
                                            *last = format!("{}\n{}", last, text);
                                        }
                                        Err(err) => {
                                            error!(
                                                "{}. <Cell: {} in File: {:?}>",
                                                err, n, self.path
                                            )
                                        }
                                    }
                                } else {
                                    warn!("Tried to add to a page that was not initialized. <Cell: {} in File: {:?}>", n, self.path);
                                }
                            }
                            Tag::AddStreamToPage => {
                                if let Some(last) = pages.last_mut() {
                                    match cell.get_stream() {
                                        Ok(text) => {
                                            *last = format!("{}\n{}", last, text);
                                        }
                                        Err(err) => {
                                            error!(
                                                "{}. <Cell: {} in File: {:?}>",
                                                err, n, self.path
                                            )
                                        }
                                    }
                                } else {
                                    warn!("Tried to add stream to a page that was not initialized. <Cell: {} in File: {:?}>", n, self.path);
                                }
                            }
                            Tag::AddErrorToPage => {
                                if let Some(last) = pages.last_mut() {
                                    match cell.get_error() {
                                        Ok(text) => {
                                            *last = format!("{}\n{}", last, text);
                                        }
                                        Err(err) => {
                                            error!(
                                                "{}. <Cell: {} in File: {:?}>",
                                                err, n, self.path
                                            );
                                        }
                                    }
                                } else {
                                    warn!("Tried to add error to a page that was not initialized. <Cell: {} in File: {:?}>", n, self.path);
                                }
                            }
                            Tag::InjectToPage(text) => {
                                if let Some(last) = pages.last_mut() {
                                    *last = format!("{}\n{}", last, text);
                                } else {
                                    warn!("Tried to insert '{}' to a page that was not initialized. <Cell: {} in File: {:?}>", text, n, self.path);
                                }
                            }
                            Tag::WrapImage(wrap) => {
                                let markdown = cell.get_source()?;
                                if let Some(last) = pages.last_mut() {
                                    match wrap_image(&markdown, &wrap) {
                                        Ok(ok) => {
                                            *last = format!("{}\n{}", last, ok);
                                        }
                                        Err(err) => {
                                            error!(
                                                "{}. <Cell: {} in File: {:?}>",
                                                err, n, self.path
                                            );
                                        }
                                    }
                                } else {
                                    warn!("Tried to insert 'wrap-image' to a page that was not initialized. <Cell: {} in File: {:?}>", n, self.path);
                                }
                            }
                            Tag::PageClass(class) => page_class = Some(class),
                        },
                        Err(err) => match err {
                            TagError::UnknownTag(err) => {
                                info!("{} <Cell: {} in File: {:?}>", err, n, self.path);
                            }
                            err => {
                                error!(
                                    "Unable to read tag '{}'. {} <Cell: {} in File: {:?}>",
                                    tag, err, n, self.path
                                );
                            }
                        },
                    }
                }
            }
        }
        let pages = pages.join("\n\n---\n");
        let Some(pages) = replace_paths(output_path, &self.path, pages) else{
            return Err(anyhow::Error::msg(format!("Either the output path {:?} or the notebook path {:?} has no parent.", output_path,self.path)))
        };
        Ok(pages)
    }

    /// Try to create a [`Notebook`] from a file in json format.
    ///
    /// # Errors
    ///
    /// This function will return an error if the file could not be read or not parsed from json.
    pub fn try_from_path(path: &PathBuf) -> Result<Notebook> {
        let text = fs::read_to_string(path)?;
        let mut notebook: Notebook = serde_json::from_str(&text)?;
        notebook.path = path.clone();

        Ok(notebook)
    }
}

/// All for this program relevant tags that a notebook cell can have.
#[derive(Debug, Clone)]
pub enum Tag {
    /// Create a new page.
    NewPage,
    /// Add only the content to the latest page.
    AddToPage,
    /// Add only the stream to the latest page. (e.g. the output of a `print()`
    /// function)
    AddStreamToPage,
    /// Add only the error to the latest page. (Only the error not the position)
    AddErrorToPage,
    /// Add the content of the injection to the latest page.
    InjectToPage(String),
    /// Wrapp the images of a markdown cell in the given string. A more
    /// detailed description can be found in the `readme.md`.
    WrapImage(String),
    /// Set the class of the latest page.
    PageClass(String),
}

impl TryFrom<&str> for Tag {
    type Error = TagError;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        if value == "new-page" {
            Ok(Tag::NewPage)
        } else if value == "add-to-page" {
            Ok(Tag::AddToPage)
        } else if value == "add-stream-to-page" {
            Ok(Tag::AddStreamToPage)
        } else if value == "add-error-to-page" {
            Ok(Tag::AddErrorToPage)
        } else if value.starts_with("inject-to-page") {
            if value.starts_with("inject-to-page[") && value.ends_with(']') {
                let content = value[15..(value.len() - 1)].to_string();
                Ok(Tag::InjectToPage(content))
            } else {
                Err(TagError::NoClosedBrackets)
            }
        } else if value.starts_with("wrap-image") {
            if value.starts_with("wrap-image[") && value.ends_with(']') {
                let content = value[11..(value.len() - 1)].to_string();
                Ok(Tag::WrapImage(content))
            } else {
                Err(TagError::NoClosedBrackets)
            }
        } else if value.starts_with("class") {
            if value.starts_with("class[") && value.ends_with(']') {
                let content = value[6..(value.len() - 1)].to_string();
                Ok(Tag::PageClass(content))
            } else {
                Err(TagError::NoClosedBrackets)
            }
        } else {
            Err(TagError::UnknownTag(value.to_string()))
        }
    }
}

/// The errors that can occur while parsing a tag.
#[derive(Debug)]
pub enum TagError {
    /// The tag is not known to the program.
    UnknownTag(String),
    /// Tags with definitions have no closing brackets `[]`.
    NoClosedBrackets,
}
impl Display for TagError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TagError::UnknownTag(tag) => write!(f, "Unknown Tag '{}'.", tag),
            TagError::NoClosedBrackets => write!(f, "Brackets are not closed."),
        }
    }
}
impl Error for TagError {}
