use std::io::Write;
use std::process::{Command, Stdio};
use std::str;

use chrono::{Date, DateTime, Utc, MAX_DATE, MIN_DATE};
use chrono_english::{parse_date_string, Dialect};
use color_eyre::Help;
use dialoguer::{theme, Editor, Input};

use crate::errors::LostTheWay;

/// To clear ANSI styling
pub const END_ANSI: &str = "\x1b[0m";

/// language color box
pub const BOX: &str = "\u{25a0}";

/// Name of the app, used for making project directories and reading the YAML file
pub const NAME: &str = "the-way";

/// ASCII code of semicolon
pub const SEMICOLON: u8 = 59;

/// Set clipboard contents to text
/// See https://github.com/aweinstock314/rust-clipboard/issues/28#issuecomment-534295371
pub fn copy_to_clipboard(text: String) -> color_eyre::Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = Command::new("pbcopy");

    #[cfg(target_os = "linux")]
    let mut command = {
        let mut c = Command::new("xclip");
        c.arg("-in");
        c.arg("-selection");
        c.arg("clipboard");
        c
    };

    let mut child = command.stdin(Stdio::piped()).spawn()?;

    // When stdin is dropped the fd is automatically closed. See
    // https://doc.rust-lang.org/std/process/struct.ChildStdin.html.
    {
        let stdin = child.stdin.as_mut().ok_or(LostTheWay::ClipboardError)?;
        stdin.write_all(text.as_bytes())?;
    }

    // Wait on pbcopy/xclip to finish.
    child.wait()?;

    Ok(())
}

/// Splits input by space
pub fn split_tags(input: &str) -> Vec<String> {
    input
        .split(' ')
        .map(|word| word.trim().to_string())
        .collect::<Vec<String>>()
}

/// Converts an array of bytes to a string
pub fn u8_to_str(input: &[u8]) -> color_eyre::Result<String> {
    Ok(str::from_utf8(input)?.to_owned())
}

/// Splits byte array by semicolon into usize
pub fn split_indices_usize(index_list: &[u8]) -> color_eyre::Result<Vec<usize>> {
    let index_list_string = str::from_utf8(index_list)?;
    Ok(index_list_string
        .split(str::from_utf8(&[SEMICOLON])?)
        .map(str::parse)
        .collect::<color_eyre::Result<Vec<_>, _>>()?)
}

/// List of usize into semicolon-joined byte array
pub fn make_indices_string(index_list: &[usize]) -> color_eyre::Result<Vec<u8>> {
    Ok(index_list
        .iter()
        .map(|index| index.to_string())
        .collect::<Vec<String>>()
        .join(str::from_utf8(&[SEMICOLON])?)
        .as_bytes()
        .to_vec())
}

/// Makes a date from a string, can be colloquial like "next Friday"
pub fn parse_date(date_string: &str) -> color_eyre::Result<Date<Utc>> {
    if date_string.to_ascii_lowercase() == "today" {
        Ok(Utc::now().date())
    } else {
        Ok(parse_date_string(date_string, Utc::now(), Dialect::Uk)?.date())
    }
}

/// Some(date) => date
/// None => minimum possible date
pub fn date_start(from_date: Option<Date<Utc>>) -> DateTime<Utc> {
    match from_date {
        Some(from_date) => from_date.and_hms(0, 0, 0),
        None => MIN_DATE.and_hms(0, 0, 0),
    }
}

/// Some(date) => date
/// None => maximum possible date
pub fn date_end(to_date: Option<Date<Utc>>) -> DateTime<Utc> {
    match to_date {
        Some(to_date) => to_date.and_hms(23, 59, 59),
        None => MAX_DATE.and_hms(23, 59, 59),
    }
}

/// Gets input from external editor, optionally displays default text in editor
pub fn external_editor_input(default: Option<&str>, extension: &str) -> color_eyre::Result<String> {
    Ok(Editor::new()
        .extension(extension)
        .edit(default.unwrap_or(""))?
        .ok_or(LostTheWay::EditorError)
        .suggestion("Set your default editor using the $EDITOR environment variable")?)
}

/// Takes user input from terminal, optionally has a default and optionally displays it.
pub fn user_input(
    message: &str,
    default: Option<&str>,
    show_default: bool,
) -> color_eyre::Result<String> {
    match default {
        Some(default) => Ok(Input::with_theme(&theme::ColorfulTheme::default())
            .with_prompt(message)
            .default(default.to_owned())
            .show_default(show_default)
            .interact()?
            .trim()
            .to_owned()),
        None => Ok(
            Input::<String>::with_theme(&theme::ColorfulTheme::default())
                .with_prompt(message)
                .interact()?
                .trim()
                .to_owned(),
        ),
    }
}
