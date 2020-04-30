//! Snippet information and methods
use std::collections::HashMap;

use anyhow::Error;
use chrono::{DateTime, Utc};
use path_abs::{FileEdit, FileRead, PathFile};

use crate::language::{CodeHighlight, Language};
use crate::utils;

/// Stores information about a quote
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Snippet {
    /// Snippet index, used to retrieve, copy, or modify a snippet
    #[serde(default)]
    pub(crate) index: usize,
    /// Snippet description, what does it do?
    description: String,
    /// Language the snippet is written in
    pub(crate) language: String,
    /// Snippet code
    pub(crate) code: String,
    /// extension
    #[serde(default)]
    extension: String,
    /// Tags attached to the snippet
    #[serde(default)]
    pub(crate) tags: Vec<String>,
    /// Date of recording the snippet
    #[serde(default = "Utc::now")]
    date: DateTime<Utc>,
}

impl Snippet {
    /// New snippet
    fn new(
        index: usize,
        description: String,
        language: String,
        extension: String,
        tags: &str,
        date: DateTime<Utc>,
        code: String,
    ) -> Self {
        Self {
            index,
            description,
            language,
            extension,
            tags: utils::split_tags(tags),
            date,
            code,
        }
    }

    pub(crate) fn set_extension(
        &mut self,
        language_name: &str,
        languages: &HashMap<String, Language>,
    ) {
        self.extension = Language::get_extension(language_name, languages);
    }

    /// Queries user for new snippet info
    /// TODO: make sure empty tags work, empty description doesn't make sense though I think
    pub(crate) fn from_user(
        index: usize,
        languages: &HashMap<String, Language>,
        old_snippet: Option<&Self>,
    ) -> Result<Self, Error> {
        let (old_description, old_language, old_tags, old_date, old_code) = match old_snippet {
            Some(s) => (
                Some(s.description.as_str()),
                Some(s.language.as_str()),
                Some(s.tags.join(" ")),
                Some(s.date.date().format("%Y-%m-%d").to_string()),
                Some(s.code.as_str()),
            ),
            None => (None, None, Some("".into()), None, None),
        };

        let description = utils::user_input("Description", old_description, false)?;
        let language = utils::user_input("Language", old_language, false)?.to_ascii_lowercase();
        let extension = Language::get_extension(&language, languages);
        let tags = utils::user_input("Tags (space separated)", old_tags.as_deref(), false)?;
        let date = match old_date {
            Some(_) => utils::parse_date(&utils::user_input("Date", old_date.as_deref(), true)?)?
                .and_hms(0, 0, 0),
            None => Utc::now(),
        };
        let mut code = utils::user_input(
            "Code snippet (<RET> to edit in external editor)",
            Some("\n"),
            false,
        )?;
        if code.is_empty() {
            code = utils::external_editor_input(old_code.as_deref(), &extension)?;
        }
        Ok(Self::new(
            index,
            description,
            language,
            extension,
            &tags,
            date,
            code,
        ))
    }

    /// write snippet to database
    pub(crate) fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok(bincode::serialize(&self)?)
    }

    /// read snippet from database
    pub(crate) fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        Ok(bincode::deserialize(bytes)?)
    }

    /// Read snippets from a JSON file and return consumable iterator
    pub(crate) fn read_from_file(
        json_file: &PathFile,
    ) -> Result<impl Iterator<Item = serde_json::Result<Self>>, Error> {
        Ok(serde_json::Deserializer::from_reader(FileRead::open(json_file)?).into_iter::<Self>())
    }

    /// Appends a snippet to a JSON object/file
    pub(crate) fn to_json(&self, json_writer: &mut FileEdit) -> Result<(), Error> {
        serde_json::to_writer(json_writer, self)?;
        Ok(())
    }

    /// Filters snippets in date range
    pub(crate) fn filter_in_date_range(
        snippets: Vec<Self>,
        from_date: DateTime<Utc>,
        to_date: DateTime<Utc>,
    ) -> Result<Vec<Self>, Error> {
        Ok(snippets
            .into_iter()
            .filter(|snippet| snippet.in_date_range(from_date, to_date))
            .collect())
    }

    /// Checks if a snippet was recorded within a date range
    pub(crate) fn in_date_range(&self, from_date: DateTime<Utc>, to_date: DateTime<Utc>) -> bool {
        from_date <= self.date && self.date < to_date
    }

    /// Check if a snippet has a particular tag associated with it
    pub(crate) fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(&tag.into())
    }

    /// Gets the title as plain text for searching
    pub(crate) fn get_header(&self) -> String {
        format!(
            "{} #{}. {} | {} :{}:\n",
            utils::BOX,
            self.index,
            self.description,
            self.language,
            self.tags.join(":")
        )
    }

    /// Highlights the title: "■ #index. description | language :tag1:tag2:\n"
    /// the block is colored according to the language
    /// language uses accent_style
    /// tags use dim_style
    /// everything else is in main_style
    pub(crate) fn pretty_print_header(
        &self,
        highlighter: &CodeHighlight,
        language: &Language,
    ) -> Result<Vec<String>, Error> {
        let mut colorized = Vec::new();
        let block = highlighter.highlight_block(language.color)?;
        colorized.push(block);
        let text = format!("#{}. {} ", self.index, self.description);
        colorized.push(CodeHighlight::highlight_string(
            &text,
            highlighter.main_style,
        ));

        let text = format!("| {} ", self.language);
        colorized.push(CodeHighlight::highlight_string(
            &text,
            highlighter.accent_style,
        ));

        let text = format!(":{}:\n", self.tags.join(":"));
        colorized.push(CodeHighlight::highlight_string(
            &text,
            highlighter.dim_style,
        ));
        colorized.push(utils::END_ANSI.to_owned());
        Ok(colorized)
    }

    /// Highlights code (with newlines before and after for readability)
    pub(crate) fn pretty_print_code(
        &self,
        highlighter: &CodeHighlight,
    ) -> Result<Vec<String>, Error> {
        let mut colorized = vec![String::from("\n")];
        colorized.extend_from_slice(&highlighter.highlight_code(&self.code, &self.extension)?);
        colorized.push(String::from("\n"));
        colorized.push(String::from("\n"));
        colorized.push(String::from(utils::END_ANSI));
        Ok(colorized)
    }

    pub(crate) fn pretty_print(
        &self,
        highlighter: &CodeHighlight,
        language: &Language,
    ) -> Result<Vec<String>, Error> {
        let mut colorized = vec![String::from("\n")];
        colorized.extend_from_slice(&self.pretty_print_header(highlighter, language)?);
        colorized.push("\n".into());
        colorized.extend_from_slice(&self.pretty_print_code(highlighter)?);
        colorized.push("\n".into());
        Ok(colorized)
    }
}
