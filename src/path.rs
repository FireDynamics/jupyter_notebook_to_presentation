//! In order to display an image in a notebook, the path to the image must be defined.
//! In most cases, the path is relative and therefore must be updated to reflect the new
//! destination of the presentation file. This crate provides functionality for parsing
//! and manipulating the image paths.
use anyhow::Result;
use chumsky::{prelude::*, text::whitespace};
use std::{
    error::Error,
    ffi::OsStr,
    fmt::Display,
    num::ParseIntError,
    ops::Range,
    path::{Path, PathBuf},
};

/// Creates a parser wich returns the span of the content of a dubble qouted string.
fn duble_quote_string() -> impl Parser<char, (String, Range<usize>), Error = Simple<char>> {
    take_until(just::<_, _, Simple<char>>('"').ignored().rewind())
        .map_with_span(|(s, _), r| (s.into_iter().collect(), r))
        .padded_by(just('"'))
}

/// Creates a parser wich returns the span of the content of a single qouted string.
fn single_quote_string() -> impl Parser<char, (String, Range<usize>), Error = Simple<char>> {
    take_until(just::<_, _, Simple<char>>('\'').ignored().rewind())
        .map_with_span(|(s, _), r| (s.into_iter().collect(), r))
        .padded_by(just('\''))
}

/// Searches for a HTML element in a markdown stream and returns all possible src path spans.
fn find_paths_in_html() -> impl Parser<char, (String, Range<usize>), Error = Simple<char>> {
    let src = just::<_, _, Simple<char>>("src")
        .then(whitespace())
        .then(just('='))
        .then(whitespace());

    let inner = take_until(src.ignored())
        .ignored()
        .then(duble_quote_string().or(single_quote_string()))
        .map(|(_, s)| s)
        .then(take_until(just('>').ignored().rewind()))
        .map(|(s, _)| s);

    inner.delimited_by(just('<').ignored(), just('>').ignored())
}

/// Searches for a markdown image element in a markdown stream and returns the path span.
fn find_path_in_markdown_image() -> impl Parser<char, (String, Range<usize>), Error = Simple<char>>
{
    let start = take_until(just::<_, _, Simple<char>>(']').ignored().rewind())
        .ignored()
        .delimited_by(just("![").ignored(), just(']').ignored());

    let end = take_until(just(')').ignored().rewind())
        .map_with_span(|(s, _), r| (s.into_iter().collect(), r))
        .delimited_by(just('(').ignored(), just(')').ignored());

    start.then(end).map(|(_, s)| s)
}

/// Searches for all HTML or markdown image elements in a markdown stream and returns all possible path spans.
fn find_paths_in_markdown() -> impl Parser<char, Vec<(String, Range<usize>)>, Error = Simple<char>>
{
    take_until(find_path_in_markdown_image().or(find_paths_in_html()))
        .map(|(_, s)| s)
        .repeated()
}

/// All possible errors that can occur when applying a `wrap-image[...]` tag of a cell.
#[derive(Debug, PartialEq, Eq)]
pub enum WrapError {
    /// An error that occurs when an `usize` in an `{}` is not defined properly.
    ParseIntError(ParseIntError),
    /// This error occurs when the given markdown could not be parsed properly. Note that
    /// this error should only occur when the parse function has been improperly configured.
    MarkdownError(Vec<Simple<char>>),
    /// An error that occurs when the tag is not set up properly. (e.g. no closing `{}`)
    SplitError(Vec<Simple<char>>),
    /// An error that occurs when the `usize` in an `{}` is not less then the amount of
    /// possible images in a cell.
    OutOfIndex(usize, usize),
}
impl Display for WrapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WrapError::ParseIntError(err) => err.fmt(f),
            WrapError::SplitError(err) => {
                write!(f, "Unable to split the content properly. {:?}", err)
            }
            WrapError::OutOfIndex(len, i) => write!(f, "Out of index. Len: {} Index: {}", len, i),
            WrapError::MarkdownError(err) => {
                write!(f, "Unable to the markdown properly. {:?}", err)
            }
        }
    }
}
impl Error for WrapError {}

/// This function implements the `wrap-image[...]` tag for a cell by parsing the markdown content of a cell
/// to retrieve the paths to the images. These paths are then wrapped in the string provided by the content
/// of the tag.
///
/// # Errors
/// An error will be returned if the number inside a `{}` is defined incorrectly or is too large. Additionally,
/// an error may occur during parsing of the tag or markdown.
pub fn wrap_image(markdown: &str, wrap: &str) -> std::result::Result<String, WrapError> {
    let paths = match find_paths_in_markdown().parse(markdown) {
        Ok(ok) => ok.into_iter().map(|s| s.0).collect::<Vec<_>>(),
        Err(err) => return Err(WrapError::MarkdownError(err)),
    };

    let (splits, end) = match take_until(just::<_, _, Simple<char>>('{').ignored())
        .then(take_until(just('}').ignored()))
        .map(|((left, _), (middle, _))| {
            (
                left.into_iter().collect::<String>(),
                middle.into_iter().collect::<String>(),
            )
        })
        .repeated()
        .then(take_until(end()))
        .map(|(splits, (end, _))| (splits, end.into_iter().collect::<String>()))
        .parse(wrap)
    {
        Ok(ok) => ok,
        Err(err) => return Err(WrapError::SplitError(err)),
    };

    let start = splits
        .into_iter()
        .enumerate()
        .map(|(i, (left, right))| {
            let i = if right.is_empty() {
                i
            } else {
                match right.parse::<usize>() {
                    Ok(ok) => ok,
                    Err(err) => return Err(WrapError::ParseIntError(err)),
                }
            };

            if i >= paths.len() {
                return Err(WrapError::OutOfIndex(i, paths.len()));
            }

            Ok(format!("{}{}", left, paths[i]))
        })
        .collect::<Result<String, _>>()?;

    let text = format!("{start}{end}");
    Ok(text)
}

/// Since the paths in a notebook are relative, this function replaces the paths to point to the images relative to the `output_path`.
/// This function will return `None`, if neither the `output_path` nor the `notebook_path` have a parent directory. Note that
/// this scenario should not occur, as both paths are file paths.
pub fn replace_paths(
    output_path: &Path,
    notebook_path: &Path,
    mut markdown: String,
) -> Option<String> {
    let paths = find_paths_in_markdown()
        .parse::<_, &str>(&markdown)
        .unwrap();

    for (path, range) in paths.into_iter().rev() {
        if path.starts_with('/') || path.starts_with("http://") || path.starts_with("https://") {
            continue;
        }
        if let Some(new_path) =
            generate_new_path(output_path, notebook_path, Path::new(&path))?.to_str()
        {
            let left = &markdown.chars().take(range.start()).collect::<String>();
            let right = &markdown.chars().skip(range.end()).collect::<String>();
            markdown = format!("{left}{new_path}{right}");
        }
    }

    Some(markdown)
}

/// Since the paths in a notebook are relative, this function corrects the paths to point to the images relative to the `output_path`.
/// This function will return `None`, if neither the `output_path` nor the `notebook_path` have a parent directory. Note that
/// this scenario should not occur, as both paths are file paths.
fn generate_new_path(
    output_path: &Path,
    notebook_path: &Path,
    element_path: &Path,
) -> Option<PathBuf> {
    Some(
        output_path
            .parent()?
            .iter()
            .map(|_| OsStr::new(".."))
            .collect::<PathBuf>()
            .join(notebook_path.parent()?)
            .join(element_path),
    )
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use chumsky::Parser;

    use super::{
        duble_quote_string, find_path_in_markdown_image, find_paths_in_html,
        find_paths_in_markdown, replace_paths, single_quote_string, wrap_image,
    };

    #[test]
    fn test_single_qoute_string() {
        let text = r#"'./bilder/digital_state_clock.svg'"#;
        let parser = single_quote_string();
        let r = parser.parse(text);
        assert_eq!(
            Ok(("./bilder/digital_state_clock.svg".to_string(), 1..33)),
            r
        );
    }

    #[test]
    fn test_duble_qoute_string() {
        let text = r#""./bilder/digital_state_clock.svg""#;
        let parser = duble_quote_string();
        let r = parser.parse(text);
        assert_eq!(
            Ok(("./bilder/digital_state_clock.svg".to_string(), 1..33)),
            r
        );
    }

    #[test]
    fn test_find_paths_in_html() {
        let text = "<img src=\"./images/image.png\" width=\"60%\">\n";
        let parser = find_paths_in_html();
        let r = parser.parse(text);
        assert_eq!(Ok(("./images/image.png".to_string(), 10..28)), r);
    }

    #[test]
    fn test_find_path_in_markdown_image() {
        let text = "![Some Deskription](./images/image.png)";
        let parser = find_path_in_markdown_image();
        let r = parser.parse(text);
        assert_eq!(Ok(("./images/image.png".to_string(), 20..38)), r);
    }

    #[test]
    fn test_find_paths_in_markdown() {
        let text = r#"
        # Some Header

        ![Some Deskription](./image1.png)

        some simple text

        <img src="./image2.png" width="60%">
        "#;
        let parser = find_paths_in_markdown();
        let r = parser.parse(text);
        assert_eq!(
            Ok(vec![
                ("./image1.png".to_string(), 52..64),
                ("./image2.png".to_string(), 111..123)
            ]),
            r
        );
    }

    #[test]
    fn test_find_paths_in_markdown2() {
        let text = r#"<img src="./image1.png">
![Image1](./image2.png)
<img src="./image3.png">"#;
        let parser = find_paths_in_markdown();
        let r = parser.parse(text);
        assert_eq!(
            Ok(vec![
                ("./image1.png".to_string(), 10..22),
                ("./image2.png".to_string(), 35..47),
                ("./image3.png".to_string(), 59..71)
            ]),
            r
        );
    }

    #[test]
    fn test_wrap_image() {
        let wrap = "![Some Image]({})  \n![Some Image]({})";
        let markdown = "![](./images/image1.png)\nsome text\n![](./images/image2.png)";

        let wrapped = wrap_image(markdown, wrap);
        assert_eq!(
            Ok(
                "![Some Image](./images/image1.png)  \n![Some Image](./images/image2.png)"
                    .to_string()
            ),
            wrapped
        );
    }

    #[test]
    fn test_replace_path() {
        let markdown =
            "# Header\n![](./images/image1.png)\n<src = \"./images/image2.png\">\n![](https://webimage/image.png)\nSome Text"
                .to_string();

        let output_path = Path::new("presentations/output.rmd");
        let notebook_path = Path::new("notebooks/input.ipynb");

        let markdown = replace_paths(output_path, notebook_path, markdown);

        assert_eq!(markdown, Some("# Header\n![](../notebooks/./images/image1.png)\n<src = \"../notebooks/./images/image2.png\">\n![](https://webimage/image.png)\nSome Text".to_string()));
    }

    #[test]
    fn test_replace_path2() {
        let markdown =
        "Here Is a cell with images.  \n<img src = \"./../images/image1.png\">  The text gets ignored.  \n![Image1](./../images/image2.png)  \n".to_string();

        let wrap = "wrap-image[<img src=\"{}\">\n\n![Image1]({})]";
        let markdown = wrap_image(&markdown, wrap).unwrap();

        let output_path = Path::new("presentations/output.rmd");
        let notebook_path = Path::new("notebooks/input.ipynb");

        let markdown = replace_paths(output_path, notebook_path, markdown);

        assert_eq!(markdown, Some("wrap-image[<img src=\"../notebooks/./../images/image1.png\">\n\n![Image1](../notebooks/./../images/image2.png)]".to_string()));
    }
}
