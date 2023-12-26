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
    #[error("EditorError")]
    EditorError,
    /// Thrown when explicit Y not received from user for destructive things
    #[error("I'm a coward. Doing nothing.")]
    DoingNothing,
    /// Thrown when $HOME is not set
    #[error("Homeless: $HOME not set")]
    Homeless,
    /// Thrown when trying to load a theme which hasn't been added / doesn't exist
    #[error("ThemeError: {theme:?}")]
    ThemeError { theme: String },
    /// Thrown when trying to load a syntax which hasn't been added / doesn't exist
    #[error("SyntaxError: {message:?} {syntax_file:?}")]
    SyntaxError {
        syntax_file: String,
        message: String,
    },
    /// Thrown when there's an error while trying to access system clipboard
    #[error("ClipboardError: Couldn't copy to clipboard - {message}")]
    ClipboardError { message: String },
    #[error(
        "NoDefaultCopyCommand: No default command found for detected OS. \
        Please add a supported command to your configuration file (as copy_cmd)"
    )]
    NoDefaultCopyCommand,
    /// Thrown when `skim` search fails
    #[error("SearchError: Search failed")]
    SearchError,
    /// Errors related to changing the configuration file
    #[error("ConfigError: {message:?}")]
    ConfigError { message: String },
    /// Sync Error
    #[error("SyncError: {message:?}")]
    SyncError { message: String },
    /// Error due to invalid Gist URL
    #[error("GistUrlError: {message:?}")]
    GistUrlError { message: String },
    /// Error due to invalid the-way gist
    #[error("GistFormattingError: {message:?}")]
    GistFormattingError { message: String },
    /// Catch-all for stuff that should never happen
    #[error("OutOfCheeseError: {message:?}\nRedo from start.")]
    OutOfCheeseError { message: String },
}
