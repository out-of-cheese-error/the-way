#![allow(dead_code)]
#![feature(exact_size_is_empty)]
#[macro_use]
extern crate serde_derive;

use anyhow::Error;
use clap::{load_yaml, App};

use crate::language::get_languages;
use crate::the_way::TheWay;

mod config;
mod errors;
mod language;
mod the_way;
mod utils;

fn main() -> Result<(), Error> {
    let languages_yml = include_str!("languages.yml");
    let languages = get_languages(languages_yml)?;
    let yaml = load_yaml!("the_way.yml");
    let matches = App::from(yaml).get_matches();
    TheWay::start(matches, languages)?;

    Ok(())
}
