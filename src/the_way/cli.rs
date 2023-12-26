//! `Clap` data
use std::path::PathBuf;

use clap::Parser;
use clap_complete::Shell;

use crate::configuration::ConfigCommand;
use crate::the_way::filter::Filters;

#[derive(Debug, Parser)]
#[command(name = "the-way", author, version, about, long_about)]
/// A code snippets manager for your terminal
///
/// Record, retrieve, search, and categorize code snippets
pub struct TheWayCLI {
    /// Force colorization even when not in TTY mode
    #[clap(short, long)]
    pub colorize: bool,
    /// Turn off colorization
    #[clap(short, long, conflicts_with = "colorize")]
    pub plain: bool,
    #[clap(subcommand)]
    pub cmd: TheWaySubcommand,
}

#[derive(Debug, Parser)]
pub enum TheWaySubcommand {
    /// Add a new code snippet
    New,
    /// Add a new shell snippet
    Cmd {
        /// shell snippet code
        code: Option<String>,
    },
    /// Search to find a snippet and copy, edit or delete it
    Search {
        #[clap(flatten)]
        filters: Filters,
        /// Use exact search instead of fuzzy
        #[clap(long, short)]
        exact: bool,
        /// Print to stdout instead of copying (with Enter)
        #[clap(long, short)]
        stdout: bool,
        /// Don't ask for confirmation when deleting
        #[clap(long, short)]
        force: bool,
    },
    /// Sync snippets to a Gist
    ///
    /// Controlled by $THE_WAY_GITHUB_TOKEN env variable.
    /// Set this to an access token with the "gist" scope obtained from https://github.com/settings/tokens/new
    Sync {
        #[clap(subcommand)]
        cmd: SyncCommand,
        /// Don't ask for confirmation before deleting local snippets
        #[clap(long, short)]
        force: bool,
    },
    /// Lists (optionally filtered) snippets
    List {
        #[clap(flatten)]
        filters: Filters,
    },
    /// Imports code snippets from JSON.
    ///
    /// Looks for description, language, and code fields.
    Import {
        /// filename, reads from stdin if not given
        file: Option<PathBuf>,

        /// URL to a Gist, if provided will import snippets from given Gist
        ///
        /// Multiple files will be converted to separate snippets.
        /// Snippet description is created based on Gist description and file name with the format
        /// "<gist_description> - <gist_id> - <file_name>".
        /// Each snippet will be tagged with "gist" and its Gist ID.
        /// Works for both secret and public gists.
        #[clap(long, short, value_name = "URL")]
        gist_url: Option<String>,

        /// URL to a gist file produced by `the-way sync`. If provided will import snippets with
        /// descriptions and tags taken from the `index.md` index file in the gist.
        #[clap(long, short = 'w', conflicts_with = "gist_url", value_name = "URL")]
        the_way_url: Option<String>,
    },
    /// Saves (optionally filtered) snippets to JSON.
    Export {
        /// filename, writes to stdout if not given
        file: Option<PathBuf>,
        #[clap(flatten)]
        filters: Filters,
    },
    /// Clears all data
    Clear {
        /// Don't ask for confirmation
        #[clap(long, short)]
        force: bool,
    },
    /// Generate shell completions
    Complete {
        /// Shell to generate completions for
        #[clap(value_enum)]
        shell: Shell,
    },
    /// Manage syntax highlighting themes
    Themes {
        #[clap(subcommand)]
        cmd: ThemeCommand,
    },
    /// Manage the-way data locations.
    ///
    /// Controlled by $THE_WAY_CONFIG env variable,
    /// use this to have independent snippet sources for different projects.
    #[clap(alias = "configure")]
    Config {
        #[clap(subcommand)]
        cmd: ConfigCommand,
    },
    /// Change snippet
    Edit {
        /// Index of snippet to change, opens a search window if not given
        index: Option<usize>,
        #[clap(flatten)]
        filters: Filters,
        /// Use exact search instead of fuzzy
        #[clap(long, short)]
        exact: bool,
    },
    /// Delete snippet
    #[clap(alias = "delete")]
    Del {
        /// Index of snippet to delete, opens a search window if not given
        index: Option<usize>,
        #[clap(flatten)]
        filters: Filters,
        /// Use exact search instead of fuzzy
        #[clap(long, short)]
        exact: bool,
        /// Don't ask for confirmation
        #[clap(long, short)]
        force: bool,
    },
    /// Copy snippet to clipboard
    #[clap(alias = "copy")]
    Cp {
        /// Index of snippet to copy, opens a search window if not given
        index: Option<usize>,
        #[clap(flatten)]
        filters: Filters,
        /// Use exact search instead of fuzzy
        #[clap(long, short)]
        exact: bool,
        /// Print to stdout instead of copying
        #[clap(long, short)]
        stdout: bool,
    },
    /// View snippet
    View {
        /// Index of snippet to show, opens a search window if not given
        index: Option<usize>,
        #[clap(flatten)]
        filters: Filters,
        /// Use exact search instead of fuzzy
        #[clap(long, short)]
        exact: bool,
    },
    /// Lists (optionally filtered) tags
    Tags {
        #[clap(flatten)]
        filters: Filters,
    },
    /// Lists (optionally filtered) languages
    Languages {
        #[clap(flatten)]
        filters: Filters,
    },
}

#[derive(Parser, Debug)]
pub enum ThemeCommand {
    /// Set your preferred syntax highlighting theme
    Set { theme: Option<String> },
    /// Add a theme from a Sublime Text ".tmTheme" file.
    Add {
        /// .tmTheme file path
        file: PathBuf,
    },
    /// Add highlight support for a language using a ".sublime-syntax" file.
    Language {
        /// .sublime-syntax file path
        file: PathBuf,
    },
    /// Prints the current theme name
    Get,
}

#[derive(Parser, Debug, Eq, PartialEq)]
pub enum SyncCommand {
    /// Sync by comparing each snippet's updated date to Gist updated date
    Date,
    /// Use local snippets as source of truth, choose this after upgrading to a new release or if Gist is messed up
    Local,
    /// Use Gist snippets as source of truth, choose this to sync snippets across computers
    Gist,
}
