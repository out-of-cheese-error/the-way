#![allow(dead_code)]
#![feature(exact_size_is_empty)]
#[macro_use]
extern crate serde_derive;

use anyhow::Error;
use clap::{load_yaml, App};

use crate::the_way::TheWay;

mod config;
mod errors;
mod the_way;
mod utils;

fn main() -> Result<(), Error> {
    let yaml = load_yaml!("the_way.yml");
    let matches = App::from(yaml).get_matches();
    TheWay::start(matches)?;
    Ok(())
}
