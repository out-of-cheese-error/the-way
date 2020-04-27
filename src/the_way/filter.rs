use anyhow::Error;
use chrono::{DateTime, Utc};
use clap::{ArgMatches, Values};

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
