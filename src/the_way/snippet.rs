//! Snippet information and methods
use std::borrow::Cow;
use std::collections::{BTreeSet, HashMap};
use std::hash::Hash;
use std::io;

use chrono::{DateTime, Utc};
use regex::Regex;
use syntect::highlighting::Style;

use crate::language::{CodeHighlight, Language};
use crate::utils;

/// Stores information about a quote
#[derive(Serialize, Deserialize, Debug, Eq)]
pub struct Snippet {
    /// Snippet index, used to retrieve, copy, or modify a snippet
    #[serde(default)]
    pub index: usize,
    /// Snippet description, what does it do?
    pub description: String,
    /// Language the snippet is written in
    pub language: String,
    /// Snippet code
    pub code: String,
    /// extension
    #[serde(default)]
    pub extension: String,
    /// Tags attached to the snippet
    #[serde(default)]
    pub tags: Vec<String>,
    /// Time of recording the snippet
    #[serde(default = "Utc::now")]
    pub date: DateTime<Utc>,
    /// Time of last update
    #[serde(default = "Utc::now")]
    pub updated: DateTime<Utc>,
}

impl PartialEq for Snippet {
    fn eq(&self, other: &Self) -> bool {
        self.description == other.description
            && self.language.to_ascii_lowercase() == other.language.to_ascii_lowercase()
            && self.code.trim() == other.code.trim()
            && self.tags.iter().collect::<BTreeSet<_>>()
                == other.tags.iter().collect::<BTreeSet<_>>()
    }
}

impl Hash for Snippet {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.description.hash(state);
        self.language.to_ascii_lowercase().hash(state);
        self.code.trim().hash(state);
        self.tags.iter().collect::<BTreeSet<_>>().hash(state);
    }
}

impl Snippet {
    /// New snippet
    pub(crate) fn new(
        index: usize,
        description: String,
        language: String,
        extension: String,
        tags: &str,
        date: DateTime<Utc>,
        updated: DateTime<Utc>,
        code: String,
    ) -> Self {
        Self {
            index,
            description,
            language,
            extension,
            tags: utils::split_tags(tags),
            date,
            updated,
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
    pub(crate) fn from_user(
        index: usize,
        languages: &HashMap<String, Language>,
        old_snippet: Option<&Self>,
    ) -> color_eyre::Result<Self> {
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

        let description = utils::user_input("Description", old_description, true, false)?;
        let language =
            utils::user_input("Language", old_language, true, false)?.to_ascii_lowercase();
        let extension = Language::get_extension(&language, languages);
        let tags = utils::user_input("Tags (space separated)", old_tags.as_deref(), true, true)?;
        let date = match old_date {
            Some(_) => utils::parse_date(&utils::user_input(
                "Date",
                old_date.as_deref(),
                true,
                false,
            )?)?
            .and_hms(0, 0, 0),
            None => Utc::now(),
        };

        let code = if let Some(old) = old_code {
            if utils::confirm("Edit snippet? [y/N]", false)? {
                utils::external_editor_input(old_code, &extension)?
            } else {
                old.to_owned()
            }
        } else {
            let mut input = utils::user_input(
                "Code snippet (leave empty to open external editor)",
                None,  // default
                false, // show default
                true,  // allow empty
            )?;
            if input.is_empty() {
                input = utils::external_editor_input(None, &extension)?;
            }
            input
        };
        Ok(Self::new(
            index,
            description,
            language,
            extension,
            &tags,
            date,
            Utc::now(),
            code,
        ))
    }

    /// Queries user for new shell snippet info
    pub(crate) fn cmd_from_user(index: usize, code: Option<&str>) -> color_eyre::Result<Self> {
        let code = utils::user_input("Command", code, true, false)?;
        let description = utils::user_input("Description", None, true, false)?;
        let tags = utils::user_input("Tags (space separated)", None, true, true)?;
        Ok(Self::new(
            index,
            description,
            "sh".into(),
            ".sh".into(),
            &tags,
            Utc::now(),
            Utc::now(),
            code,
        ))
    }

    /// write snippet to database
    pub(crate) fn to_bytes(&self) -> color_eyre::Result<Vec<u8>> {
        Ok(bincode::serialize(&self)?)
    }

    /// read snippet from database
    pub(crate) fn from_bytes(bytes: &[u8]) -> color_eyre::Result<Self> {
        Ok(bincode::deserialize(bytes)?)
    }

    /// Read snippets from a JSON stream and return consumable iterator
    pub(crate) fn read(
        json_reader: &mut dyn io::Read,
    ) -> impl Iterator<Item = serde_json::Result<Self>> + '_ {
        serde_json::Deserializer::from_reader(json_reader).into_iter::<Self>()
    }

    /// Appends a snippet to a JSON object/file
    pub(crate) fn to_json(&self, json_writer: &mut dyn io::Write) -> color_eyre::Result<()> {
        serde_json::to_writer(json_writer, self)?;
        Ok(())
    }

    /// Filters snippets in date range
    pub(crate) fn filter_in_date_range(
        snippets: Vec<Self>,
        from_date: DateTime<Utc>,
        to_date: DateTime<Utc>,
    ) -> Vec<Self> {
        snippets
            .into_iter()
            .filter(|snippet| snippet.in_date_range(from_date, to_date))
            .collect()
    }

    /// Checks if a snippet was recorded within a date range
    pub(crate) fn in_date_range(&self, from_date: DateTime<Utc>, to_date: DateTime<Utc>) -> bool {
        from_date <= self.date && self.date < to_date
    }

    /// Check if a snippet has a particular tag associated with it
    pub(crate) fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(&tag.into())
    }

    /// Highlights the title: "■ #index. description | language :tag1:tag2:\n"
    /// the block is colored according to the language
    /// language uses `accent_style`
    /// tags use `dim_style`
    /// everything else is in `main_style`
    pub(crate) fn pretty_print_header(
        &self,
        highlighter: &CodeHighlight,
        language: &Language,
    ) -> Vec<(Style, String)> {
        let mut colorized = Vec::new();
        let block = CodeHighlight::highlight_block(language.color);
        colorized.push(block);
        let text = format!("#{}. {} ", self.index, self.description);
        colorized.push((highlighter.main_style, text));
        let text = format!("| {} ", self.language);
        colorized.push((highlighter.accent_style, text));
        let text = format!(":{}:\n", self.tags.join(":"));
        colorized.push((highlighter.tag_style, text));
        colorized
    }

    pub(crate) fn pretty_print(
        &self,
        highlighter: &CodeHighlight,
        language: &Language,
    ) -> Vec<(Style, String)> {
        let mut colorized = vec![(Style::default(), String::from("\n"))];
        colorized.extend_from_slice(&self.pretty_print_header(highlighter, language));
        colorized.push((Style::default(), String::from("\n")));
        colorized.extend_from_slice(&highlighter.highlight_code(&self.code, &self.extension));
        colorized.push((Style::default(), String::from("\n\n")));
        colorized
    }

    fn is_shell_snippet(&self) -> bool {
        // sh, bash, csh, tcsh
        matches!(self.language.as_str(), "sh" | "bash" | "csh" | "tcsh")
    }

    /// If snippet is a shell snippet, interactively fill parameters
    pub(crate) fn fill_snippet(&self, highlight_style: Style) -> color_eyre::Result<Cow<str>> {
        // other languages, return as is
        if !self.is_shell_snippet() {
            return Ok(Cow::Borrowed(self.code.as_str()));
        }
        // Matches the param or param=value **inside** the angular brackets
        let re1 = Regex::new("<(?P<parameter>[^<>]+)>")?;
        // Matches <param> or <param=value>
        let re2 = Regex::new("(?P<match><[^<>]+>)")?;

        // Highlight parameters to fill
        eprintln!(
            "{}",
            re2.replace_all(&self.code, |caps: &regex::Captures| {
                utils::highlight_string(&caps["match"], highlight_style)
            })
        );
        // Ask user to fill in (unique) parameters
        let mut filled_parameters = HashMap::new();
        for capture in re1.captures_iter(&self.code) {
            let mut parts = capture["parameter"].split('=');
            let parameter_name = parts.next().unwrap().to_owned();
            let default = parts.next();
            if !filled_parameters.contains_key(&parameter_name) {
                let filled = utils::user_input(&parameter_name, default, true, false)?;
                filled_parameters.insert(parameter_name, filled);
            }
        }

        // Replace parameters in code
        Ok(re2.replace_all(&self.code, |caps: &regex::Captures| {
            let parameter = caps["match"][1..caps["match"].len() - 1]
                .split('=')
                .next()
                .unwrap();
            &filled_parameters[parameter]
        }))
    }
}
