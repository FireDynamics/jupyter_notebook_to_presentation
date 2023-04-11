use chumsky::{prelude::*, text::whitespace};

/// All for this program relevant commands that a notebook cell can have.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Create a new page.
    NewPage,
    /// Start adding the content to the latest page.
    StartAddToPage,
    /// Stop adding the content to the latest page.
    StopAddToPage,
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

/// Makdown inside a Jupyter Notebook, can spann over multiple lines. For this
/// reason, the different cases must be considered
#[derive(Debug, Clone, PartialEq)]
pub enum MarkdownCommandState {
    /// The passed line starts with `<!--!` but has no end `-->`.
    OnlyStart(Vec<Command>),
    /// The passed line does not starts with `<!--!` nore ends with `-->`.
    Inner(Vec<Command>),
    /// The passed line starts with `<!--!` and ends with `-->`.
    Full(Vec<Command>),
    /// The passed line does not starts with `<!--!` but ends with `-->`.
    OnlyEnd(Vec<Command>),
}

/// Parse a list of contents in case of nested `[...]`
fn parse_content() -> impl Parser<char, String, Error = Simple<char>> {
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
        .map(|f| f.replace(r"\[", "[").replace(r"\]", "]"))
}

/// A Parser that only parse to [`Command::NewPage`].
fn parse_new_page_command() -> impl Parser<char, Command, Error = Simple<char>> {
    just("NewPage").to(Command::NewPage)
}

/// A Parser that only parse to [`Command::StartAddToPage`].
fn parse_start_add_to_page_command() -> impl Parser<char, Command, Error = Simple<char>> {
    just("StartAddToPage").to(Command::StartAddToPage)
}

/// A Parser that only parse to [`Command::StopAddToPage`].
fn parse_stop_add_to_page_command() -> impl Parser<char, Command, Error = Simple<char>> {
    just("StopAddToPage").to(Command::StopAddToPage)
}

/// A Parser that only parse to [`Command::AddStreamToPage`].
fn parse_add_stream_to_page_command() -> impl Parser<char, Command, Error = Simple<char>> {
    just("AddStreamToPage").to(Command::AddStreamToPage)
}

/// A Parser that only parse to [`Command::AddErrorToPage`].
fn parse_add_error_to_page_command() -> impl Parser<char, Command, Error = Simple<char>> {
    just("AddErrorToPage").to(Command::AddErrorToPage)
}

/// A Parser that only parse to [`Command::InjectToPage`].
fn parse_inject_to_page_command() -> impl Parser<char, Command, Error = Simple<char>> {
    just("InjectToPage")
        .ignore_then(parse_content().padded())
        .map(Command::InjectToPage)
}

/// A Parser that only parse to [`Command::WrapImage`].
fn parse_wrap_image_command() -> impl Parser<char, Command, Error = Simple<char>> {
    just("WrapImage")
        .ignore_then(parse_content().padded())
        .map(Command::WrapImage)
}

/// A Parser that only parse to [`Command::PageClass`].
fn parse_page_class_command() -> impl Parser<char, Command, Error = Simple<char>> {
    just("PageClass")
        .ignore_then(parse_content().padded())
        .map(|s| Command::PageClass(s.trim().to_string()))
}

/// A parser that parse to [`Command`]
fn parse_command() -> impl Parser<char, Command, Error = Simple<char>> {
    parse_new_page_command()
        .or(parse_start_add_to_page_command())
        .or(parse_stop_add_to_page_command())
        .or(parse_add_stream_to_page_command())
        .or(parse_add_error_to_page_command())
        .or(parse_inject_to_page_command())
        .or(parse_wrap_image_command())
        .or(parse_page_class_command())
}
/// A parser that parse to [`Vec<Command>`]
fn parse_commands() -> impl Parser<char, Vec<Command>, Error = Simple<char>> {
    parse_command()
        .then_ignore(just(',').padded())
        .repeated()
        .chain(parse_command())
        .padded()
}

//TODO Sollte in im Notebook gehandhabt werden.
/// Parse a Markdown command to [`Vec<Command>`] as long the command starts with `<!--!`
fn parse_commands_from_markdown_comment() -> impl Parser<char, Vec<Command>, Error = Simple<char>> {
    just("<!--!")
        .ignore_then(parse_commands())
        .then_ignore(just("-->"))
        .padded()
}

#[cfg(test)]
mod test {
    use chumsky::Parser;

    use crate::commands::MarkdownCommandState;

    use super::{parse_commands, parse_commands_from_markdown_comment, parse_content, Command::*};

    #[test]
    fn test_parse_content() {
        let parser = parse_content();

        let result = parser.parse(r"[]");
        assert_eq!(result, Ok(r"".to_string()));
        let result = parser.parse(r"[content]");
        assert_eq!(result, Ok(r"content".to_string()));
        let result = parser.parse(r"[content \[ content]");
        assert_eq!(result, Ok(r"content [ content".to_string()));
        let result = parser.parse(r"[content \] content]");
        assert_eq!(result, Ok(r"content ] content".to_string()));
        let result = parser.parse(r"[content \[ content \] content]");
        assert_eq!(result, Ok(r"content [ content ] content".to_string()));

        let result = parser.parse(r"[content []");
        assert!(result.is_err());
        let result = parser.parse(r"[content ]]");
        assert_eq!(result, Ok(r"content ".to_string()));
    }

    #[test]
    fn test_parse_commands() {
        let parser = parse_commands();

        let result = parser.parse(
            r#"
        NewPage,
        StartAddToPage,
        StopAddToPage,
        AddStreamToPage,
        AddErrorToPage,
        InjectToPage [
            content
        ],
        WrapImage [
            content
        ],
        PageClass [
            class
        ]
        "#,
        );

        assert_eq!(
            result,
            Ok(vec![
                NewPage,
                StartAddToPage,
                StopAddToPage,
                AddStreamToPage,
                AddErrorToPage,
                InjectToPage("\n            content\n        ".to_string()),
                WrapImage("\n            content\n        ".to_string()),
                PageClass("class".to_string()),
            ])
        );
    }

    #[test]
    fn test_parse_commands_from_markdown_comment() {
        let parser = parse_commands_from_markdown_comment();

        let result = parser.parse(r#"<!--! NewPage, InjectToPage[content] -->"#);
        assert_eq!(
            result,
            Ok(vec![NewPage, InjectToPage("content".to_string())],)
        );
    }
}
