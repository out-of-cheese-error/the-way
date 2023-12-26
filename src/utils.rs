use std::collections::HashSet;
use std::io::Write;
use std::process::{Command, Stdio};
use std::str;

use chrono::{DateTime, NaiveDateTime, Utc};
use chrono_english::{parse_date_string, Dialect};
use color_eyre::Help;
use dialoguer::{Completion, Confirm, Editor, Input};
use syntect::highlighting::Style;
use syntect::util::as_24_bit_terminal_escaped;

use crate::errors::LostTheWay;

/// To clear ANSI styling
pub const END_ANSI: &str = "\x1b[0m";

/// language color box
pub const BOX: &str = "\u{25a0}";

/// Name of the app, used for making project directories and reading the YAML file
pub const NAME: &str = "the-way";

/// ASCII code of semicolon
pub const SEMICOLON: u8 = 59;

/// Defines the default supported clipboard copy commands.
/// A `String` containing the copy command with the arguments is returned
/// according to the conditional compilation on the detected OS.
pub(crate) fn get_default_copy_cmd() -> Option<String> {
    if cfg!(target_os = "linux") {
        Some("xclip -in -selection clipboard".to_string())
    } else if cfg!(target_os = "macos") {
        Some("pbcopy".to_string())
    } else if cfg!(target_os = "android") {
        Some("termux-clipboard-set".to_string())
    } else {
        None
    }
}

/// Set clipboard contents to text
/// See [issue](https://github.com/aweinstock314/rust-clipboard/issues/28#issuecomment-534295371)
pub fn copy_to_clipboard(copy_cmd_field: &Option<String>, text: &str) -> color_eyre::Result<()> {
    let copy_cmd_vec = copy_cmd_field
        .as_ref()
        .ok_or(LostTheWay::NoDefaultCopyCommand)?
        .split_whitespace()
        .map(|s| s.to_owned())
        .collect::<Vec<String>>();

    let default_copy_cmd_vec: Vec<String>;
    let (copy_cmd, copy_args) = match copy_cmd_vec.split_first() {
        Some((cmd, args)) => (cmd, args),
        _ => {
            default_copy_cmd_vec = get_default_copy_cmd()
                .ok_or(LostTheWay::NoDefaultCopyCommand)?
                .split_whitespace()
                .map(|s| s.to_owned())
                .collect();
            let (cmd, args) = match default_copy_cmd_vec.split_first() {
                Some((cmd, args)) => (cmd, args),
                // Should never fails due to previous checking
                _ => unreachable!(),
            };
            eprintln!("The `copy_cmd` field is empty, defaulting to `{cmd}`");
            (cmd, args)
        }
    };

    let mut child = Command::new(copy_cmd)
        .args(copy_args)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| LostTheWay::ClipboardError {
            message: format!(
                "{e}: is {copy_cmd} available? Also check your `copy_cmd` settings ({})",
                // Never fails as it's checked above
                copy_cmd_field.as_ref().unwrap()
            ),
        })?;

    // When stdin is dropped the fd is automatically closed. See
    // https://doc.rust-lang.org/std/process/struct.ChildStdin.html.
    {
        let stdin = child.stdin.as_mut().ok_or(LostTheWay::ClipboardError {
            message: "Could not access stdin".to_string(),
        })?;
        stdin.write_all(text.as_bytes())?;
    }

    // Wait on copy command to finish.
    child.wait()?;

    Ok(())
}

/// Splits input by space
pub fn split_tags(input: &str) -> Vec<String> {
    input
        .split(' ')
        .map(|word| word.trim().to_owned())
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
        .map(std::string::ToString::to_string)
        .collect::<Vec<String>>()
        .join(str::from_utf8(&[SEMICOLON])?)
        .as_bytes()
        .to_vec())
}

/// Makes a date from a string, can be colloquial like "next Friday"
pub fn parse_date(date_string: &str) -> color_eyre::Result<DateTime<Utc>> {
    if date_string.to_ascii_lowercase() == "today" {
        Ok(Utc::now())
    } else {
        Ok(parse_date_string(date_string, Utc::now(), Dialect::Uk)?)
    }
}

/// Some(date) => date
/// None => minimum possible date
pub fn date_start(from_date: Option<DateTime<Utc>>) -> DateTime<Utc> {
    from_date.unwrap_or_else(|| DateTime::from_utc(NaiveDateTime::MIN, Utc))
}

/// Some(date) => date
/// None => maximum possible date
pub fn date_end(to_date: Option<DateTime<Utc>>) -> DateTime<Utc> {
    to_date.unwrap_or_else(|| DateTime::from_utc(NaiveDateTime::MAX, Utc))
}

/// Gets input from external editor, optionally displays default text in editor
pub fn external_editor_input(default: Option<&str>, extension: &str) -> color_eyre::Result<String> {
    Editor::new()
        .extension(extension)
        .edit(default.unwrap_or(""))
        .suggestion("Set your default editor using the $EDITOR or $VISUAL environment variables")?
        .ok_or(LostTheWay::EditorError)
        .suggestion("Make sure to save next time if you want to record a snippet!")
}

/// Takes user input from terminal, optionally has a default and optionally displays it.
pub fn user_input(
    message: &str,
    default: Option<&str>,
    show_default: bool,
    allow_empty: bool,
    completions: TheWayCompletion,
) -> color_eyre::Result<String> {
    let theme = dialoguer::theme::ColorfulTheme::default();
    match default {
        Some(default) => {
            let mut input = Input::with_theme(&theme)
                .with_prompt(message)
                .completion_with(&completions)
                .allow_empty(allow_empty)
                .default(default.to_owned())
                .show_default(false);
            if show_default {
                input = input.with_initial_text(default);
            }
            Ok(input.interact_text()?.trim().to_owned())
        }
        None => Ok(Input::<String>::with_theme(&theme)
            .with_prompt(message)
            .completion_with(&completions)
            .allow_empty(allow_empty)
            .interact_text()?
            .trim()
            .to_owned()),
    }
}

/// Get a yes/no answer from the user
pub fn confirm(prompt: &str, default: bool) -> color_eyre::Result<bool> {
    let theme = dialoguer::theme::ColorfulTheme::default();
    Ok(Confirm::with_theme(&theme)
        .with_prompt(prompt)
        .default(default)
        .show_default(false)
        .interact()?)
}

/// Make an indicatif spinner with given message
pub fn get_spinner(message: &str) -> indicatif::ProgressBar {
    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_message(message.to_owned());
    spinner
}

/// Color a string for the terminal
pub fn highlight_string(line: &str, style: Style) -> String {
    let mut s = as_24_bit_terminal_escaped(&[(style, line)], false);
    s.push_str(END_ANSI);
    s
}

/// Color strings for the terminal
pub fn highlight_strings(inputs: &[(Style, String)], bg: bool) -> String {
    if bg {
        let mut s = String::new();
        for (style, line) in inputs {
            s.push_str(&as_24_bit_terminal_escaped(&[(*style, line)], true));
            s.push_str(END_ANSI);
        }
        s
    } else {
        as_24_bit_terminal_escaped(
            &inputs
                .iter()
                .map(|(style, line)| (*style, line.as_ref()))
                .collect::<Vec<_>>(),
            false,
        )
    }
}

/// Print with color if stdout is tty else without
/// if colorize, always uses color
/// if plain, doesn't use color
pub fn smart_print(
    inputs: &[(Style, String)],
    bg: bool,
    colorize: bool,
    plain: bool,
) -> color_eyre::Result<()> {
    write!(
        grep_cli::stdout(termcolor::ColorChoice::Auto),
        "{}",
        if !plain & (grep_cli::is_tty_stdout() | colorize) {
            highlight_strings(inputs, bg)
        } else {
            inputs
                .iter()
                .map(|(_, s)| s.to_string())
                .collect::<Vec<_>>()
                .join("")
        }
    )?;
    Ok(())
}

#[derive(Debug)]
pub enum TheWayCompletion {
    Language(Vec<String>),
    Tag(Vec<String>),
    Empty,
}

impl Completion for TheWayCompletion {
    fn get(&self, input: &str) -> Option<String> {
        match self {
            Self::Language(languages) => {
                let matches = languages
                    .iter()
                    .filter(|option| option.starts_with(input))
                    .collect::<Vec<_>>();

                if !matches.is_empty() {
                    Some(matches[0].to_string())
                } else {
                    None
                }
            }
            Self::Tag(tags) => {
                let current_tags_list = input
                    .split(' ')
                    .map(|s| s.to_string())
                    .collect::<HashSet<_>>();
                let last_input = input.split(' ').last().unwrap_or("");
                let last_space = input.rfind(' ').unwrap_or(0);
                let matches = tags
                    .iter()
                    .filter(|option| {
                        option.starts_with(last_input) && !current_tags_list.contains(*option)
                    })
                    .collect::<Vec<_>>();

                if !matches.is_empty() {
                    Some(
                        (input[..last_space].trim().to_string() + " " + matches[0])
                            .trim()
                            .to_string(),
                    )
                } else {
                    None
                }
            }
            Self::Empty => None,
        }
    }
}
