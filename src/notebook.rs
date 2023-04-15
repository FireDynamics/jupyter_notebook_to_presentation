//! Load and read a `.ipynb` notebook with `serde` and apply the assigned tags.
use anyhow::Result;
use log::{error, debug};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    commands::{self, Command},
    path::{replace_paths, wrap_image},
};

/// Possible states of a command sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandSequenceState {
    /// Represents the state when the current line starts or is within a
    /// command sequence.
    Within,
    /// Represents the state when the current line ends a command sequence.
    End,
    /// Represents a state that is neither within a command sequence nor ends
    /// one.
    Outside,
}


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
    // TODO Remove Metadata since it has no use anymore
    /// The metadata bounded to the cell. Relevant are only the tags.
    metadata: Metadata,
    /// Possible outputs of a cell, e.g. an error of a code cell.
    outputs: Option<Vec<Output>>,
    /// The content of a cell.
    source: Vec<String>,
}
impl Cell {
    /// Returns the get source without commands comment of this [`Cell`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the cell type is not `markdown` or `code`.
    fn get_source_without_commands_comment(&self) -> Result<String> {
        match self.cell_type.as_str() {
            "markdown" => {
                let mut is_command = false;
                let text = self
                    .source
                    .iter()
                    .filter(|f| {
                        let trimmed = f.trim();
                        if trimmed.starts_with("<!--!") {
                            is_command = true;
                        }
                        if trimmed.ends_with("-->") {
                            is_command = false;
                        }
                        !is_command
                    })
                    .cloned()
                    .collect::<String>();
                Ok(text)
            }
            _ => Err(anyhow::Error::msg(format!(
                "Cell type '{}' is currently not supported.",
                self.cell_type
            ))),
        }
    }

    /// Processes the current [`Cell`] and executes all contained commands. The
    /// contents of the cell are read line by line, and any command comments
    /// encountered are collected until the comment ends. All collected
    /// commands are then executed in the order they were encountered, except
    /// for [`Command::PageClass`].
    ///
    /// # Errors
    ///
    /// This function may return an error if:
    /// - The cell type is not `markdown` or `code`.
    /// - The command comment cannot be parsed.
    /// - The [`Command::StartAddToPage`], [`Command::AddStreamToPage`],
    /// [`Command::AddErrorToPage`], [`Command::InjectToPage`],
    /// [`Command::WrapImage`], and [`Command::PageClass`] commands are used
    /// before a page is initialized.
    /// - The `markdown` command comment is not properly closed.
    fn proses_to_presentation(
        &self,
        pages: &mut Vec<String>,
        page_class: &mut Option<String>,
    ) -> Result<()> {
        match self.cell_type.as_str() {
            "markdown" => (),
            cell_type => {
                debug!("Cell type: '{cell_type}' is currently not supported.");
                return Ok(());
            }
        }

        let mut command_line = 0;
        let mut command_sequence_state = CommandSequenceState::Outside;
        let mut command_sequence = vec![];
        let mut add_to_page = false;

        let mut lines = self.source.iter().enumerate().peekable();
        while let Some((i, line)) = lines.next() {
            if command_sequence_state == CommandSequenceState::Outside {
                command_line = i;
            }

            let trimmed = line.trim();
            match (trimmed.starts_with("<!--!"), trimmed.ends_with("-->")) {
                (true, true) => {
                    command_sequence_state = CommandSequenceState::End;
                    command_sequence.push(&trimmed[5..(trimmed.len() - 3)]);
                }
                (true, false) => {
                    command_sequence_state = CommandSequenceState::Within;
                    command_sequence.push(&trimmed[5..trimmed.len()]);
                }
                (false, true) => {
                    if command_sequence_state == CommandSequenceState::Within {
                        command_sequence_state = CommandSequenceState::End;
                        command_sequence.push(&trimmed[0..(trimmed.len() - 3)]);
                    }
                }
                (false, false) => {
                    if command_sequence_state == CommandSequenceState::Within {
                        command_sequence.push(trimmed);
                    }
                }
            };

            || -> Result<()> {
                if command_sequence_state == CommandSequenceState::Outside || lines.peek().is_none() {
                    let stream = command_sequence.join("");
                    let stream = stream.trim();
                    if !stream.is_empty() {

                        let commands = commands::parse(stream)
                        .map_err(|err| {
                            anyhow::Error::msg(format!("Unable to parse commands. '{}' {} ", command_sequence.join("").trim(), err))
                        })?;
                        
                        debug!("{commands:?}");
                        
                        for command in commands {
                            match command {
                                Command::NewPage => {
                                    if let Some(class) = page_class {
                                        if let Some(last) = pages.last_mut() {
                                            *last = format!("class: {class}\n\n{last}");
                                            *page_class = None;
                                        } else {
                                            return Err(anyhow::Error::msg(
                                                "Tried to set a class page that was not initialized. ",
                                            ));
                                        }
                                    }
                                    pages.push(String::new());
                                },
                            Command::StartAddToPage => {
                                add_to_page = true;
                                },
                            Command::StopAddToPage => {
                                add_to_page = false;
                            },
                            Command::InjectToPage(content) => {
                                if let Some(last) = pages.last_mut() {
                                    *last = format!("{last}{}", content);
                                } else {
                                    return Err(anyhow::Error::msg(
                                        format!("Tried to insert '{content}' to a page that was not initialized. "),
                                    ));
                                }
                            }
                            Command::WrapImage(content) => {
                                if let Some(last) = pages.last_mut() {
                                    let wrap = wrap_image(
                                        &self.get_source_without_commands_comment()?,
                                        &content,
                                    )?;
                                    *last = format!("{last}{}", wrap);
                                } else {
                                    return Err(anyhow::Error::msg(
                                        "Tried to insert a 'WrapImage' to a page that was not initialized. ".to_string(),
                                    ));
                                }
                            }
                            Command::PageClass(class) => *page_class = Some(class),
                        }
                    }
                    }
                    command_sequence.clear()
                }
                    if add_to_page && command_sequence_state == CommandSequenceState::Outside{
                        if let Some(last) = pages.last_mut() {
                            if line.ends_with('\n'){
                                *last = format!("{last}{}", line.clone());
                            }else{
                                *last = format!("{last}{}\n", line.clone());
                            }
                        } else {
                            return Err(anyhow::Error::msg(
                                "Tried to insert to a page that was not initialized. "
                                .to_string(),
                            ));
                        }
                    }

                Ok(())
            }().map_err(|op| {
                let text = format!("Line {command_line} to {i}. {}",op);
                op.context(text)
            })?;
            
            if command_sequence_state == CommandSequenceState::End {
                command_sequence_state = CommandSequenceState::Outside;
            }
        }

        if command_sequence_state == CommandSequenceState::Within {
            return Err(anyhow::Error::msg(
                "Missing comment closing element. ".to_string(),
            ));
        }
        Ok(())
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

        debug!("Convert notebook {:?} into pages", self.path);
        for (i, cell) in self.cells.iter().enumerate() {
            debug!("Convert cell {} into pages", i);
            if let Err(err) = cell.proses_to_presentation(&mut pages, &mut page_class) {
                error!("Cell: {} in File: {:?}. {}", i, self.path, err)
            }
        }
        if let Some(class) = page_class{
            if let Some(last) = pages.last_mut(){
                *last = format!("class: {class}\n\n{last}");
            }else{
                error!("Cell: {} in File: {:?}. Tried to set a class page that was not initialized. ", self.cells.len(), self.path,)
            }
        }

        let pages = pages.join("\n---\n\n");
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

#[cfg(test)]
mod test {
    use crate::commands::Command;

    use super::Cell;

    #[test]
    fn test_cell_to_page() {
        let mut pages = vec![];
        let mut page_class = None;
        let cell = Cell {
            cell_type: "markdown".to_string(),
            outputs: None,
            source: vec![
                format!("<!--! {}; {}; -->\n", Command::NEW_PAGE, Command::START_ADD_TO_PAGE),
                "# Headline\n".to_string(),
            ],
            metadata: super::Metadata { tags: None },
        };
        cell.proses_to_presentation(&mut pages, &mut page_class).unwrap();
        assert_eq!(pages, vec!["# Headline\n".to_string()]);

        let mut pages = vec![];
        let mut page_class = None;
        let cell = Cell {
            cell_type: "markdown".to_string(),
            outputs: None,
            source: vec![
                "<!--!".to_string(),
                format!("{}\n;", Command::NEW_PAGE),
                format!("{}\n;", Command::START_ADD_TO_PAGE),
                "-->\n".to_string(),
                "# Headline\n".to_string(),
                "Text\n".to_string(),
                "More Text\n".to_string(),
            ],
            metadata: super::Metadata { tags: None },
        };
        cell.proses_to_presentation(&mut pages, &mut page_class).unwrap();
        assert_eq!(pages, vec!["# Headline\nText\nMore Text\n".to_string()]);
        
        let mut pages = vec![];
        let mut page_class = None;
        let cell = Cell {
            cell_type: "markdown".to_string(),
            outputs: None,
            source: vec![
                format!("<!--!{};\n", Command::NEW_PAGE),
                format!("{};\n", Command::NEW_PAGE),
                format!("{};\n", Command::NEW_PAGE),
                format!("{};\n", Command::NEW_PAGE),
                "-->\n".to_string(),
                "# Headline\n".to_string(),
                "Text".to_string(),
                ],
                metadata: super::Metadata { tags: None },
            };
        cell.proses_to_presentation(&mut pages, &mut page_class).unwrap();
        assert_eq!(pages, vec!["".to_string(),"".to_string(),"".to_string(),"".to_string()]);
    }
}
