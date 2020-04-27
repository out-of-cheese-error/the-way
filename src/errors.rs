use thiserror::Error;

/// Errors which can be caused by normal quoth operation.
/// Those caused by external libraries throw their own errors
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
    /// Thrown when argument is given empty input like '' or ' '
    #[error("Type something already!")]
    NoInputError,
    /// Thrown when explicit Y not received from user for destructive things
    #[error("{message:?}\nDoing nothing.")]
    DoingNothing { message: String },
    /// Thrown when $HOME is not set
    #[error("$HOME not set")]
    Homeless,
    /// Thrown when badly formatted file given for parsing
    #[error("I can't read {filename:?}. See x for the expected format")]
    FileParseError { filename: String },
    /// Thrown when trying to load a theme which hasn't been added / doesn't exist
    #[error(
        "I don't have the {theme_name:?} theme. Add it from a theme file with the-way themes --add"
    )]
    ThemeNotFound { theme_name: String },
    /// Catch-all for stuff that should never happen
    #[error("{message:?}\nRedo from start.")]
    OutOfCheeseError { message: String },
}
