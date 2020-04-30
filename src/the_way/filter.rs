//! Code related to filtering search, list, and export results
use std::collections::HashSet;

use anyhow::Error;

use crate::the_way::cli::Filters;
use crate::the_way::snippet::Snippet;
use crate::the_way::TheWay;
use crate::utils;

impl TheWay {
    /// Filters a list of snippets by given language/tag/date
    pub(crate) fn filter_snippets(&self, filters: &Filters) -> Result<Vec<Snippet>, Error> {
        let from_date = utils::date_start(filters.from);
        let to_date = utils::date_end(filters.to);
        let snippets: Option<Vec<_>> = match filters.languages.clone() {
            Some(languages) => Some(
                self.get_snippets(
                    &languages
                        .into_iter()
                        .flat_map(|language| {
                            self.get_language_snippets(&language).unwrap_or_default()
                        })
                        .collect::<Vec<_>>(),
                )?,
            ),
            None => None,
        };
        match (filters.tags.clone(), snippets) {
            (Some(tags), Some(snippets)) => Ok(snippets
                .into_iter()
                .filter(|snippet| {
                    snippet.in_date_range(from_date, to_date)
                        && tags.iter().any(|tag| snippet.has_tag(tag))
                })
                .collect()),
            (Some(tags), None) => {
                let indices = tags
                    .into_iter()
                    .flat_map(|tag| self.get_tag_snippets(&tag).unwrap_or_default())
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
