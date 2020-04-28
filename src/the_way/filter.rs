use std::collections::HashSet;

use anyhow::Error;
use chrono::{DateTime, Utc};
use clap::{ArgMatches, Values};

use crate::the_way::snippet::Snippet;
use crate::the_way::TheWay;
use crate::utils;

/// Stores (language, tag, date) filters parsed from command-line arguments to restrict the snippets to look at
pub(crate) struct Filters<'a> {
    pub(crate) languages: Option<Values<'a>>,
    pub(crate) tags: Option<Values<'a>>,
    pub(crate) from_date: Option<DateTime<Utc>>,
    pub(crate) to_date: Option<DateTime<Utc>>,
}

impl<'a> Filters<'a> {
    /// Parses filters (on author, book, tag, date) from command-line arguments
    pub(crate) fn get_filters(matches: &'a ArgMatches) -> Result<Filters<'a>, Error> {
        let on_date = utils::get_argument_value("on", matches)?;
        let from_date = if on_date.is_some() {
            on_date
        } else {
            utils::get_argument_value("from", matches)?
        }
        .map(|date| utils::parse_date(date))
        .transpose()?
        .map(|date| date.and_hms(0, 0, 0));

        let to_date = if on_date.is_some() {
            on_date
        } else {
            utils::get_argument_value("to", &matches)?
        }
        .map(|date| utils::parse_date(date))
        .transpose()?
        .map(|date| date.and_hms(23, 59, 59));

        let (languages, tags) = (
            utils::get_argument_values("language", matches)?,
            utils::get_argument_values("tag", matches)?,
        );
        Ok(Filters {
            languages,
            tags,
            from_date,
            to_date,
        })
    }
}

impl<'a> TheWay<'a> {
    /// Filters a list of snippets by given language/tag/date
    pub(crate) fn filter_snippets(&self, filters: &Filters<'_>) -> Result<Vec<Snippet>, Error> {
        let from_date = utils::date_start(filters.from_date);
        let to_date = utils::date_end(filters.to_date);
        let snippets: Option<Vec<_>> = match filters.languages.clone() {
            Some(languages) => Some(
                self.get_snippets(
                    &languages
                        .flat_map(|language| {
                            self.get_language_snippets(language).unwrap_or_default()
                        })
                        .collect::<Vec<_>>(),
                )?,
            ),
            None => None,
        };
        match (filters.tags.clone(), snippets) {
            (Some(tags), Some(snippets)) => {
                let tags: Vec<_> = tags.map(|t| t).collect();
                Ok(snippets
                    .into_iter()
                    .filter(|snippet| {
                        snippet.in_date_range(from_date, to_date)
                            && tags.iter().any(|tag| snippet.has_tag(tag))
                    })
                    .collect())
            }
            (Some(tags), None) => {
                let indices = tags
                    .flat_map(|tag| self.get_tag_snippets(tag).unwrap_or_default())
                    .collect::<HashSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();
                Snippet::filter_in_date_range(self.get_snippets(&indices)?, from_date, to_date)
            }
            (None, Some(snippets)) => Snippet::filter_in_date_range(snippets, from_date, to_date),
            (None, None) => self.list_snippets_in_date_range(from_date, to_date),
        }
    }
}
