//! This module handles the parsing of all possible commands that can be added
//! to a cell

use std::{
    error::Error,
    fmt::{Debug, Display},
    result::Result,
};

use chumsky::prelude::*;

/// All for this program relevant commands that a notebook cell can have.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Create a new page.
    NewPage,
    /// Start adding the content to the latest page.
    StartAddToPage,
    /// Stop adding the content to the latest page.
    StopAddToPage,
    /// Add the content of the injection to the latest page.
    InjectToPage(String),
    /// Wrapp the images of a markdown cell in the given string. A more
    /// detailed description can be found in the `readme.md`.
    WrapImage(String),
    /// Set the class of the latest page.
    PageClass(String),
}

/// Represents an error encountered during command comment parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    /// Indicates an undefined command was used. Contains the unknown command.
    UnknownCommand(String),
    /// Indicates an badly formatted contend was used. Contains the corresponding command.
    Content(String),
    /// Indicates a comma is missing after a command. Contains the remaining string.
    MissingComma(String),
    /// Indicates the stream was not fully parsed. Contains the remaining string.
    Remaining(String),
    /// Indicates another undefined parsing error occurred. Contains a vector of `Simple<char>` instances.
    Other(Vec<Simple<char>>),
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnknownCommand(err) => write!(f, "Unknown command '{err}'. "),
            ParseError::Content(err) => {
                write!(f, "Content after '{err}' could not be parsed correctly. ")
            }
            ParseError::MissingComma(err) => write!(f, "Missing comma before '{err}'. "),
            ParseError::Remaining(err) => write!(f, "Unable to parse remaining '{err}'. "),
            ParseError::Other(err) => write!(f, "Unable to parse '{err:?}'. "),
        }
    }
}

impl Error for ParseError {}

impl Command {
    /// The char sequence for the `new page` command
    pub const NEW_PAGE: &'static str = "new";
    /// The char sequence for the `start add to page` command
    pub const START_ADD_TO_PAGE: &'static str = "start-add";
    /// The char sequence for the `stop add to page` command
    pub const STOP_ADD_TO_PAGE: &'static str = "stop-add";
    /// The char sequence for the `inject to page` command
    pub const INJECT_TP_PAGE: &'static str = "inject";
    /// The char sequence for the `wrap image` command
    pub const WRAP_IMAGE: &'static str = "image";
    /// The char sequence for the `class` command
    pub const PAGE_CLASS: &'static str = "class";
}

/// Parse a list of contents in case of nested `[...]`
fn parse_content() -> impl Parser<char, Option<String>, Error = Simple<char>> {
    let empty = just(']').ignored().rewind().to(vec![]);
    let content = take_until(
        none_of('\\')
            .then(just('[').or(just(']')))
            .ignored()
            .rewind(),
    )
    .map(|(s, _)| s)
    .chain(any());

    let text = just('[')
        .then(empty.or(content))
        .then(just(']'))
        .map(|((_, s), _)| s);

    text.collect::<String>()
        .map(|f| {
            f.replace(r"\[", "[")
                .replace(r"\]", "]")
                .replace(r"\n", "\n")
                .replace("\\\"", "\"")
                .replace("\\'", "'")
        })
        .or_not()
}

/// A Parser that only parse to [`Command::NewPage`].
fn parse_new_page_command() -> impl Parser<char, Command, Error = Simple<char>> {
    just(Command::NEW_PAGE).to(Command::NewPage)
}

/// A Parser that only parse to [`Command::StartAddToPage`].
fn parse_start_add_to_page_command() -> impl Parser<char, Command, Error = Simple<char>> {
    just(Command::START_ADD_TO_PAGE).to(Command::StartAddToPage)
}

/// A Parser that only parse to [`Command::StopAddToPage`].
fn parse_stop_add_to_page_command() -> impl Parser<char, Command, Error = Simple<char>> {
    just(Command::STOP_ADD_TO_PAGE).to(Command::StopAddToPage)
}

/// A Parser that only parse to [`Command::InjectToPage`].
fn parse_inject_to_page_command(
) -> impl Parser<char, Result<Command, ParseError>, Error = Simple<char>> {
    just(Command::INJECT_TP_PAGE)
        .then(parse_content().padded())
        .map(|(name, content)| match content {
            Some(some) => Ok(Command::InjectToPage(some)),
            None => Err(ParseError::Content(name.to_string())),
        })
}

/// A Parser that only parse to [`Command::WrapImage`].
fn parse_wrap_image_command() -> impl Parser<char, Result<Command, ParseError>, Error = Simple<char>>
{
    just(Command::WRAP_IMAGE)
        .then(parse_content().padded())
        .map(|(name, content)| match content {
            Some(some) => Ok(Command::WrapImage(some)),
            None => Err(ParseError::Content(name.to_string())),
        })
}

/// A Parser that only parse to [`Command::PageClass`].
fn parse_page_class_command() -> impl Parser<char, Result<Command, ParseError>, Error = Simple<char>>
{
    just(Command::PAGE_CLASS)
        .then(parse_content().padded())
        .map(|(name, content)| match content {
            Some(some) => Ok(Command::PageClass(some.trim().to_string())),
            None => Err(ParseError::Content(name.to_string())),
        })
}

/// A parser that parse to [`Command`]
fn parse_command() -> impl Parser<char, Result<Command, ParseError>, Error = Simple<char>> {
    parse_new_page_command()
        .or(parse_start_add_to_page_command())
        .or(parse_stop_add_to_page_command())
        .map(Ok)
        .or(parse_inject_to_page_command())
        .or(parse_wrap_image_command())
        .or(parse_page_class_command())
        .or(text::ident().map(|f| Err(ParseError::UnknownCommand(f))))
}
/// A parser that parse to [`Vec<Command>`]
fn parse_commands(
) -> impl Parser<char, (Result<Vec<Command>, ParseError>, String), Error = Simple<char>> {
    parse_command()
        .then(
            just(';')
                .ignored()
                .to(Ok(()))
                .or(take_until(end())
                    .map(|(s, _)| Err(ParseError::MissingComma(s.into_iter().collect()))))
                .padded(),
        )
        .map(|(command, comma)| match (command, comma) {
            (Ok(ok), Ok(_)) => Ok(ok),
            (Ok(_), Err(err)) => Err(err),
            (Err(err), Ok(_)) => Err(err),
            (Err(err), Err(_)) => Err(err),
        })
        .repeated()
        .collect()
        .then(take_until(end()).map(|(s, _)| s).collect())
        .padded()
}

/// Parses the given input `stream` and returns a `Result` containing a vector of `Command`s
/// on success, or a `ParseError` on failure.
///
/// # Errors
///
/// Returns a `ParseError` if the `stream` input could not be fully parsed.
pub fn parse(stream: &str) -> Result<Vec<Command>, ParseError> {
    match parse_commands().parse(stream) {
        Ok((result, end)) => {
            if result.is_ok() && !end.is_empty() {
                Err(ParseError::Remaining(end))
            } else {
                result
            }
        }
        Err(err) => Err(ParseError::Other(err)),
    }
}

#[cfg(test)]
mod test {
    use chumsky::Parser;

    use crate::commands::{
        parse,
        Command::{self, *},
    };

    use super::parse_content;

    #[test]
    fn test_parse_content() {
        let parser = parse_content();

        let result = parser.parse(r"[]");
        assert_eq!(result, Ok(Some(r"".to_string())));
        let result = parser.parse(r"[content]");
        assert_eq!(result, Ok(Some(r"content".to_string())));
        let result = parser.parse(r"[content \[ content]");
        assert_eq!(result, Ok(Some(r"content [ content".to_string())));
        let result = parser.parse(r"[content \] content]");
        assert_eq!(result, Ok(Some(r"content ] content".to_string())));
        let result = parser.parse(r"[content \[ content \] content]");
        assert_eq!(result, Ok(Some(r"content [ content ] content".to_string())));

        let result = parser.parse(r"[content []");
        assert_eq!(result, Ok(None));
        let result = parser.parse(r"[content ]]");
        assert_eq!(result, Ok(Some(r"content ".to_string())));
    }

    #[test]
    fn test_parse_commands() {
        let result = parse(&format!(
            r#"
        {};
        {};
        {};
        {} [
            content
        ];
        {} [
            content
        ];
        {} [
            class
        ];
        "#,
            Command::NEW_PAGE,
            Command::START_ADD_TO_PAGE,
            Command::STOP_ADD_TO_PAGE,
            Command::INJECT_TP_PAGE,
            Command::WRAP_IMAGE,
            Command::PAGE_CLASS,
        ));

        assert_eq!(
            result,
            Ok(vec![
                NewPage,
                StartAddToPage,
                StopAddToPage,
                InjectToPage("\n            content\n        ".to_string()),
                WrapImage("\n            content\n        ".to_string()),
                PageClass("class".to_string()),
            ])
        );

        let result = parse(&format!(r#" {}; "#, Command::NEW_PAGE));

        assert_eq!(result, Ok(vec![NewPage,]));
    }

    #[test]
    fn test_wrap_image() {
        let result = parse(&format!("{}[content];", Command::WRAP_IMAGE));
        assert_eq!(result, Ok(vec![Command::WrapImage("content".to_string())]));
        let result = parse(&format!("{}[!\\[\\]({})];", Command::WRAP_IMAGE, "{}"));
        assert_eq!(result, Ok(vec![Command::WrapImage("![]({})".to_string())]));
    }
}
