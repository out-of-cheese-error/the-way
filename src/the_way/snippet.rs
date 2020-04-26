use std::collections::HashMap;

use anyhow::Error;
use chrono::{DateTime, Utc};
use path_abs::{FileRead, PathFile};

use crate::language::Language;
use crate::utils;

/// Stores information about a quote
#[derive(Serialize, Deserialize, Debug)]
pub struct Snippet {
    /// Snippet index, used to retrieve, copy, or modify a snippet
    pub index: usize,
    /// Snippet description, what does it do?
    pub description: String,
    /// Language the snippet is written in
    pub language: String,
    /// Tags attached to the snippet
    pub tags: Vec<String>,
    /// Date of recording the snippet
    pub date: DateTime<Utc>,
    /// Snippet source
    pub source: String,
    /// Snippet code
    pub code: String,
}

impl Snippet {
    /// New snippet
    pub fn new(
        index: usize,
        description: String,
        language: String,
        tags: &str,
        date: DateTime<Utc>,
        source: String,
        code: String,
    ) -> Self {
        Snippet {
            index,
            description,
            language,
            tags: utils::split_tags(tags),
            date,
            source,
            code,
        }
    }

    pub fn from_user(
        index: usize,
        languages: &HashMap<String, Language>,
        old_snippet: Option<Snippet>,
    ) -> Result<Snippet, Error> {
        let (old_description, old_language, old_tags, old_date, old_source, old_code) =
            match old_snippet {
                Some(s) => (
                    Some(s.description),
                    Some(s.language),
                    Some(s.tags.join(" ")),
                    Some(s.date.date().format("%Y-%m-%d").to_string()),
                    Some(s.source),
                    Some(s.code),
                ),
                None => (None, None, None, None, None, None),
            };

        let description = utils::user_input("Description", old_description.as_deref(), false)?;
        let language_name =
            utils::user_input("Language", old_language.as_deref(), false)?.to_ascii_lowercase();
        let default = Language::default();
        let (language, extension) = if let Some(language) = languages.get(&language_name) {
            (language_name, &language.extension)
        } else {
            (default.name, &default.extension)
        };
        println!("Predicted name {} extension {}", language, extension);

        let tags = utils::user_input("Tags (space separated)", old_tags.as_deref(), false)?;
        let source = utils::user_input("Source", old_source.as_deref(), false)?;
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
            code = utils::external_editor_input(old_code.as_deref(), extension)?;
        }
        Ok(Snippet::new(
            index,
            description,
            language,
            &tags,
            date,
            source,
            code,
        ))
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok(bincode::serialize(&self)?)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        Ok(bincode::deserialize(bytes)?)
    }

    /// Read snippets from a JSON file and return consumable iterator
    pub fn read_from_file(
        json_file: &PathFile,
    ) -> Result<impl Iterator<Item = serde_json::Result<Snippet>>, Error> {
        Ok(serde_json::Deserializer::from_reader(FileRead::open(json_file)?).into_iter::<Self>())
    }

    /// Filters snippets in date range
    pub fn filter_in_date_range(
        snippets: Vec<Snippet>,
        from_date: DateTime<Utc>,
        to_date: DateTime<Utc>,
    ) -> Result<Vec<Snippet>, Error> {
        Ok(snippets
            .into_iter()
            .filter(|snippet| snippet.in_date_range(from_date, to_date))
            .collect())
    }

    /// Checks if a snippet was recorded within a date range
    pub fn in_date_range(&self, from_date: DateTime<Utc>, to_date: DateTime<Utc>) -> bool {
        from_date <= self.date && self.date < to_date
    }

    /// Check if a snippet has a particular tag associated with it
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(&tag.into())
    }

    // TODO: Display a quote in the terminal prettily
    pub fn pretty_print(&self) {
        println!("{:?}", self);
    }
}
