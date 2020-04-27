use std::io;
use std::str;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::Error;
use chrono::{Date, DateTime, Datelike, Utc, MAX_DATE, MIN_DATE};
use chrono_english::{parse_date_string, Dialect};
use clap::{ArgMatches, Values};
use clipboard::{ClipboardContext, ClipboardProvider};
use dialoguer::{theme, Editor, Input};
use termion::event::Key;
use termion::input::TermRead;

use crate::errors::LostTheWay;

pub const RAVEN: char = '\u{1313F}';
pub const END_ANSI: &str = "\x1b[0m";

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

pub fn get_months(min_date: Date<Utc>, max_date: Date<Utc>) -> Result<Vec<Date<Utc>>, Error> {
    let (min_year, min_month) = (min_date.year(), min_date.month());
    let (max_year, max_month) = (max_date.year(), max_date.month());
    let mut months = Vec::with_capacity((max_year - min_year) as usize * 12);
    let date = Utc::now().date();
    for month in min_month..=12 {
        months.push(
            date.with_year(min_year)
                .ok_or(LostTheWay::OutOfCheeseError {
                    message: format!("Invalid year {}", min_year),
                })?
                .with_month(month)
                .ok_or(LostTheWay::OutOfCheeseError {
                    message: format!("Invalid month {}", month),
                })?
                .with_day(1)
                .unwrap(),
        );
    }
    for year in min_year..max_year {
        for month in 1..=12 {
            months.push(
                date.with_year(year)
                    .ok_or(LostTheWay::OutOfCheeseError {
                        message: format!("Invalid year {}", year),
                    })?
                    .with_month(month)
                    .unwrap()
                    .with_day(1)
                    .unwrap(),
            );
        }
    }
    for month in 1..=max_month {
        months.push(
            date.with_year(max_year)
                .ok_or(LostTheWay::OutOfCheeseError {
                    message: format!("Invalid year {}", max_year),
                })?
                .with_month(month)
                .ok_or(LostTheWay::OutOfCheeseError {
                    message: format!("Invalid month {}", month),
                })?
                .with_day(1)
                .unwrap(),
        );
    }
    Ok(months)
}

pub enum Event<I> {
    Input(I),
    Tick,
}

/// A small event handler that wraps termion input and tick events. Each event
/// type is handled in its own thread and returned to a common `Receiver`
pub struct Events {
    rx: mpsc::Receiver<Event<Key>>,
    input_handle: thread::JoinHandle<()>,
    tick_handle: thread::JoinHandle<()>,
}

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub exit_key: Key,
    pub tick_rate: Duration,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            exit_key: Key::Char('q'),
            tick_rate: Duration::from_millis(250),
        }
    }
}

impl Events {
    pub fn new() -> Events {
        Events::with_config(Config::default())
    }

    pub fn with_config(config: Config) -> Events {
        let (tx, rx) = mpsc::channel();
        let input_handle = {
            let tx = tx.clone();
            thread::spawn(move || {
                let stdin = io::stdin();
                for evt in stdin.keys() {
                    if let Ok(key) = evt {
                        if tx.send(Event::Input(key)).is_err() {
                            return;
                        }
                        if key == config.exit_key {
                            return;
                        }
                    }
                }
            })
        };
        let tick_handle = {
            thread::spawn(move || loop {
                tx.send(Event::Tick).unwrap();
                thread::sleep(config.tick_rate);
            })
        };
        Events {
            rx,
            input_handle,
            tick_handle,
        }
    }

    pub fn next(&self) -> Result<Event<Key>, mpsc::RecvError> {
        self.rx.recv()
    }
}
