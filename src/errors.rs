use thiserror::Error;

/// Errors which can be caused by normal the-way operation.
/// Those caused by external libraries throw their own errors when possible
#[derive(Debug, Error)]
pub enum LostTheWay {
    /// Thrown when trying to access an unrecorded language
    #[error("I don't know what {language:?} is.")]
    LanguageNotFound { language: String },
    /// Thrown when trying to access a nonexistent snippet index
    #[error("You haven't written that snippet: {index:?}.")]
    SnippetNotFound { index: usize },
    /// Thrown when trying to access an unrecorded tag
    #[error("You haven't tagged anything as {tag:?} yet.")]
    TagNotFound { tag: String },
    /// Thrown when no text is returned from an external editor
    #[error("Your editor of choice didn't work.")]
    EditorError,
    /// Thrown when explicit Y not received from user for destructive things
    #[error("{message:?}\nDoing nothing.")]
    DoingNothing { message: String },
    /// Thrown when $HOME is not set
    #[error("$HOME not set")]
    Homeless,
    /// Thrown when trying to load a theme which hasn't been added / doesn't exist
    #[error("Couldn't load theme {theme:?}. Are you sure it exists and is valid?")]
    ThemeError { theme: String },
    #[error("Couldn't copy to clipboard")]
    ClipboardError,
    #[error("Search failed")]
    SearchError,
    /// Errors related to changing the configuration file
    #[error("{message:?}")]
    ConfigError { message: String },
    /// Catch-all for stuff that should never happen
    #[error("{message:?}\nRedo from start.")]
    OutOfCheeseError { message: String },
}
