use std::collections::HashMap;

use anyhow::Error;
use chrono::{DateTime, Utc};
use path_abs::{FileRead, PathFile};
use syntect::highlighting::Style;
use textwrap::termwidth;

use crate::language::{CodeHighlight, Language};
use crate::utils;
use crate::utils::END_ANSI;

/// Stores information about a quote
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Snippet {
    /// Snippet index, used to retrieve, copy, or modify a snippet
    pub(crate) index: usize,
    /// Snippet description, what does it do?
    pub(crate) description: String,
    /// Language the snippet is written in
    pub(crate) language: String,
    /// extension
    pub(crate) extension: String,
    /// Tags attached to the snippet
    pub(crate) tags: Vec<String>,
    /// Date of recording the snippet
    pub(crate) date: DateTime<Utc>,
    /// Snippet code
    pub(crate) code: String,
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
        Snippet {
            index,
            description,
            language,
            extension,
            tags: utils::split_tags(tags),
            date,
            code,
        }
    }

    fn get_extension(language_name: &str, languages: &HashMap<String, Language>) -> String {
        let default = Language::default();
        if let Some(l) = languages.get(language_name) {
            l.extension.to_owned()
        } else {
            println!(
                "Couldn't find language {} in the list of extensions, defaulting to .txt",
                language_name
            );
            default.extension
        }
    }

    pub(crate) fn from_user(
        index: usize,
        languages: &HashMap<String, Language>,
        old_snippet: Option<&Snippet>,
    ) -> Result<Snippet, Error> {
        let (old_description, old_language, old_tags, old_date, old_code) = match old_snippet {
            Some(s) => (
                Some(s.description.as_str()),
                Some(s.language.as_str()),
                Some(s.tags.join(" ")),
                Some(s.date.date().format("%Y-%m-%d").to_string()),
                Some(s.code.as_str()),
            ),
            None => (None, None, None, None, None),
        };

        let description = utils::user_input("Description", old_description, false)?;
        let language = utils::user_input("Language", old_language, false)?.to_ascii_lowercase();
        let extension = Self::get_extension(&language, languages);
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
        Ok(Snippet::new(
            index,
            description,
            language,
            extension,
            &tags,
            date,
            code,
        ))
    }

    pub(crate) fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok(bincode::serialize(&self)?)
    }

    pub(crate) fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        Ok(bincode::deserialize(bytes)?)
    }

    /// Read snippets from a JSON file and return consumable iterator
    pub(crate) fn read_from_file(
        json_file: &PathFile,
    ) -> Result<impl Iterator<Item = serde_json::Result<Snippet>>, Error> {
        Ok(serde_json::Deserializer::from_reader(FileRead::open(json_file)?).into_iter::<Self>())
    }

    /// Filters snippets in date range
    pub(crate) fn filter_in_date_range(
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
    pub(crate) fn in_date_range(&self, from_date: DateTime<Utc>, to_date: DateTime<Utc>) -> bool {
        from_date <= self.date && self.date < to_date
    }

    /// Check if a snippet has a particular tag associated with it
    pub(crate) fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(&tag.into())
    }

    pub(crate) fn highlight_description(
        &self,
        highlighter: &CodeHighlight,
        language: &Language,
        style: Style,
    ) -> Result<Vec<String>, Error> {
        let mut colorized = Vec::new();
        let block = highlighter.highlight_block(&language.color)?;
        colorized.push(block);
        let text = format!("#{}. {}\n", self.index, self.description);
        colorized.push(highlighter.highlight_string(&text, style));
        Ok(colorized)
    }

    pub(crate) fn pretty_print(
        &self,
        highlighter: &CodeHighlight,
        language: &Language,
        styles: (Style, Style, Style),
        for_search: bool,
    ) -> Result<Vec<String>, Error> {
        let (main_style, accent_style, dim_style) = styles;
        let mut colorized = Vec::new();
        let width = termwidth() - 4;

        if !for_search {
            colorized.extend_from_slice(&self.highlight_description(
                highlighter,
                language,
                main_style,
            )?);
        }
        colorized.extend_from_slice(&highlighter.highlight_code(&self.code, &self.extension)?);
        colorized.push(highlighter.highlight_block(&language.color)?);
        let text = format!("{} | {} \n", self.language, self.tags.join(", "),);
        colorized.push(highlighter.highlight_string(&text, accent_style));

        if !for_search {
            let dashes = (0..width / 2).map(|_| '-').collect::<String>();
            colorized.push(highlighter.highlight_string(&format!("{}\n", dashes), dim_style));
        }
        colorized.push(END_ANSI.to_owned());
        Ok(colorized)
    }
}
