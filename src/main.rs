#![feature(exact_size_is_empty)]
#![warn(
    clippy::all,
    clippy::restriction,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo
)]
#[macro_use]
extern crate serde_derive;

use anyhow::Error;
use structopt::StructOpt;

use crate::language::get_languages;
use crate::the_way::cli::TheWayCLI;
use crate::the_way::TheWay;

mod configuration;
mod errors;
mod language;
mod the_way;
mod utils;

fn main() -> Result<(), Error> {
    let languages_yml = include_str!("languages.yml");
    let languages = get_languages(languages_yml)?;
    let cli = TheWayCLI::from_args();
    TheWay::start(cli, languages)?;
    Ok(())
}
