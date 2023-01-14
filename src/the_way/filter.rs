//! Code related to filtering search, list, and export results
use std::collections::HashSet;
use std::ffi::OsString;

use chrono::{DateTime, Utc};
use clap::Parser;
use regex::Regex;

use crate::the_way::{snippet::Snippet, TheWay};
use crate::utils;

#[derive(Parser, Debug)]
pub struct Filters {
    /// Snippets written in <language> (multiple with 'lang1 lang2')
    #[clap(short, long)]
    pub(crate) languages: Option<Vec<String>>,
    /// Snippets with <tag> (multiple with 'tag1 tag2')
    #[clap(short, long)]
    pub(crate) tags: Option<Vec<String>>,
    /// Snippets from <date> ("last friday" works too!)
    #[clap(long, value_parser = utils::parse_date)]
    pub(crate) from: Option<DateTime<Utc>>,
    /// Snippets before <date>
    #[clap(long, value_parser = utils::parse_date)]
    pub(crate) to: Option<DateTime<Utc>>,
    /// Snippets matching pattern
    #[clap(short, long)]
    pub(crate) pattern: Option<OsString>,
}

impl TheWay {
    /// Filters a list of snippets by given language/tag/date
    pub(crate) fn filter_snippets(&self, filters: &Filters) -> color_eyre::Result<Vec<Snippet>> {
        let from_date = utils::date_start(filters.from);
        let to_date = utils::date_end(filters.to);
        let snippets: Option<Vec<_>> = match &filters.languages {
            Some(languages) => Some(
                self.get_snippets(
                    &languages
                        .iter()
                        .flat_map(|language| {
                            self.get_language_snippets(language).unwrap_or_default()
                        })
                        .collect::<Vec<_>>(),
                )?,
            ),
            None => None,
        };
        let snippets = match (filters.tags.clone(), snippets) {
            (Some(tags), Some(snippets)) => Ok(snippets
                .into_iter()
                .filter(|snippet| {
                    snippet.in_date_range(from_date, to_date)
                        && tags.iter().any(|tag| snippet.has_tag(tag))
                })
                .collect()),
            (Some(tags), None) => {
                let indices = tags
                    .iter()
                    .flat_map(|tag| self.get_tag_snippets(tag).unwrap_or_default())
                    .collect::<HashSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();
                Ok(Snippet::filter_in_date_range(
                    self.get_snippets(&indices)?,
                    from_date,
                    to_date,
                ))
            }
            (None, Some(snippets)) => {
                Ok(Snippet::filter_in_date_range(snippets, from_date, to_date))
            }
            (None, None) => self.list_snippets_in_date_range(from_date, to_date),
        };
        match &filters.pattern {
            Some(pattern) => {
                let regex = Regex::new(&pattern.to_string_lossy())?;
                snippets.map(|snippets| {
                    snippets
                        .into_iter()
                        .filter(|snippet| {
                            regex.is_match(&snippet.description)
                                || snippet.tags.iter().any(|tag| regex.is_match(tag))
                                || regex.is_match(&snippet.code)
                        })
                        .collect()
                })
            }
            None => snippets,
        }
    }
}
