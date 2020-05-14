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
    /// Change snippet
    Edit {
        /// Index of snippet to change
        index: usize,
    },
    /// Delete snippet
    Del {
        /// Index of snippet to delete
        index: usize,
        /// Don't ask for confirmation
        #[structopt(long, short)]
        force: bool,
    },
    /// Copy snippet to clipboard
    Cp {
        /// Index of snippet to copy
        index: usize,
    },
    /// View snippet
    View {
        /// Index of snippet to show
        index: usize,
    },
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
    /// Clears all data
    Clear {
        /// Don't ask for confirmation
        #[structopt(long, short)]
        force: bool,
    },
    /// Generate shell completions
    Complete {
        #[structopt(possible_values = & Shell::variants())]
        shell: Shell,
    },
    /// Manage syntax highlighting themes
    Themes {
        #[structopt(subcommand)]
        cmd: ThemeCommand,
    },
    /// Manage snippets data location.
    /// Controlled by $THE_WAY_CONFIG env variable,
    /// use this to have independent snippet sources for different projects.
    Config {
        #[structopt(subcommand)]
        cmd: ConfigCommand,
    },
}

#[derive(StructOpt, Debug)]
pub(crate) enum ThemeCommand {
    /// List all theme choices (default + user-added)
    List,
    /// Set your preferred syntax highlighting theme
    Set { theme: String },
    /// Add a theme from a Sublime Text ".tmTheme" file.
    Add {
        #[structopt(parse(from_os_str))]
        file: PathBuf,
    },
    /// Prints the current theme name
    Get,
}
