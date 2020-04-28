use std::str;

use anyhow::Error;
use chrono::{Date, DateTime, Utc, MAX_DATE, MIN_DATE};
use chrono_english::{parse_date_string, Dialect};
use clap::{ArgMatches, Values};
use clipboard::{ClipboardContext, ClipboardProvider};
use dialoguer::{theme, Editor, Input};

use crate::errors::LostTheWay;

pub const RAVEN: char = '\u{1313F}';
pub const END_ANSI: &str = "\x1b[0m";
pub const NAME: &str = "the-way";

/// ASCII code of semicolon
pub const SEMICOLON: u8 = 59;

pub fn copy_to_clipboard(text: String) {
    let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
    ctx.set_contents(text).unwrap();
}

/// Splits input by comma
pub fn split_tags(input: &str) -> Vec<String> {
    input
        .split(' ')
        .map(|word| word.trim().to_string())
        .collect::<Vec<String>>()
}

/// Converts an array of bytes to a string
pub fn u8_to_str(input: &[u8]) -> Result<String, Error> {
    Ok(str::from_utf8(input)?.to_owned())
}

/// Splits byte array by semicolon into strings
pub fn split_values_string(index_list: &[u8]) -> Result<Vec<String>, Error> {
    let index_list_string = str::from_utf8(index_list)?;
    Ok(index_list_string
        .split(str::from_utf8(&[SEMICOLON])?)
        .map(|s| s.to_string())
        .collect())
}

/// Splits byte array by semicolon into usize
pub fn split_indices_usize(index_list: &[u8]) -> Result<Vec<usize>, Error> {
    let index_list_string = str::from_utf8(index_list)?;
    Ok(index_list_string
        .split(str::from_utf8(&[SEMICOLON])?)
        .map(|word: &str| word.parse::<usize>())
        .collect::<Result<Vec<_>, _>>()?)
}

/// List of usize into semicolon-joined byte array
pub fn make_indices_string(index_list: &[usize]) -> Result<Vec<u8>, Error> {
    Ok(index_list
        .iter()
        .map(|index| index.to_string())
        .collect::<Vec<String>>()
        .join(str::from_utf8(&[SEMICOLON])?)
        .as_bytes()
        .to_vec())
}

pub fn parse_date(date_string: &str) -> Result<Date<Utc>, Error> {
    if date_string.to_ascii_lowercase() == "today" {
        Ok(Utc::now().date())
    } else {
        Ok(parse_date_string(date_string, Utc::now(), Dialect::Uk)?.date())
    }
}

/// Some(date) => date
/// None => minimum possible date
pub fn date_start(from_date: Option<DateTime<Utc>>) -> DateTime<Utc> {
    from_date.unwrap_or_else(|| MIN_DATE.and_hms(0, 0, 0))
}

/// Some(date) => date
/// None => maximum possible date
pub fn date_end(to_date: Option<DateTime<Utc>>) -> DateTime<Utc> {
    to_date.unwrap_or_else(|| MAX_DATE.and_hms(23, 59, 59))
}

/// Gets input from external editor, optionally displays default text in editor
pub fn external_editor_input(default: Option<&str>, extension: &str) -> Result<String, Error> {
    match Editor::new()
        .extension(extension)
        .edit(default.unwrap_or(""))?
    {
        Some(input) => Ok(input),
        None => Err(LostTheWay::EditorError.into()),
    }
}

/// Takes user input from terminal, optionally has a default and optionally displays it.
pub fn user_input(
    message: &str,
    default: Option<&str>,
    show_default: bool,
) -> Result<String, Error> {
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

/// Extracts value of a given argument from matches if present
pub fn get_argument_value<'a>(
    name: &str,
    matches: &'a ArgMatches,
) -> Result<Option<&'a str>, Error> {
    match matches.value_of(name) {
        Some(value) => {
            if value.trim().is_empty() {
                Err(LostTheWay::NoInputError.into())
            } else {
                Ok(Some(value.trim()))
            }
        }
        None => Ok(None),
    }
}

/// Extracts (multiple) values of a given argument from matches if present
pub fn get_argument_values<'a>(
    name: &str,
    matches: &'a ArgMatches,
) -> Result<Option<Values<'a>>, Error> {
    match matches.values_of(name) {
        Some(values) => {
            if values.is_empty() {
                Err(LostTheWay::NoInputError.into())
            } else {
                Ok(Some(values))
            }
        }
        None => Ok(None),
    }
}
