//! StructOpt data
use std::path::PathBuf;

use structopt::clap::AppSettings;
use structopt::clap::Shell;
use structopt::StructOpt;

use crate::configuration::ConfigCommand;
use crate::the_way::filter::Filters;

#[derive(Debug, StructOpt)]
#[structopt(
name = "the-way",
about = "Record, retrieve, search, and categorize code snippets",
rename_all = "kebab-case",
global_settings = & [AppSettings::DeriveDisplayOrder]
)]
pub(crate) enum TheWayCLI {
    /// Add a new snippet
    New,
    /// Fuzzy search and copy selected to clipboard
    Search {
        #[structopt(flatten)]
        filters: Filters,
    },
    /// Copy snippet to clipboard
    Copy { index: usize },
    /// Change snippet
    Change { index: usize },
    /// Delete snippet
    Delete {
        index: usize,
        /// Don't ask for confirmation
        #[structopt(long, short)]
        force: bool,
    },
    /// Show snippet
    Show { index: usize },
    /// Lists (optionally filtered) snippets
    List {
        #[structopt(flatten)]
        filters: Filters,
    },
    /// Imports code snippets from JSON. Looks for description, language, and code fields
    Import {
        /// filename, reads from stdin if not given
        #[structopt(parse(from_os_str))]
        file: Option<PathBuf>,
    },
    /// Saves (optionally filtered) snippets to JSON.
    Export {
        /// filename, writes to stdout if not given
        #[structopt(parse(from_os_str))]
        file: Option<PathBuf>,
        #[structopt(flatten)]
        filters: Filters,
    },
    /// Generate shell completions
    Complete {
        #[structopt(possible_values = & Shell::variants())]
        shell: Shell,
    },
    /// View syntax highlighting themes (default + user-added)
    Themes {
        #[structopt(subcommand)]
        cmd: Option<ThemeCommand>,
    },
    /// Clears all data
    Clear {
        /// Don't ask for confirmation
        #[structopt(long, short)]
        force: bool,
    },
    /// See / change where your data is stored
    /// Controlled by $THE_WAY_CONFIG env variable
    Config {
        #[structopt(subcommand)]
        cmd: ConfigCommand,
    },
}

#[derive(StructOpt, Debug)]
pub(crate) enum ThemeCommand {
    /// Set your preferred syntax highlighting theme
    Set { theme: String },
    /// Add a theme from a .tmTheme file
    Add {
        #[structopt(parse(from_os_str))]
        file: PathBuf,
    },
    /// Prints the current theme name
    Current,
}
