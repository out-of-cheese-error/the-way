#![warn(
    clippy::all,
    clippy::restriction,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo
)]
#[macro_use]
extern crate serde_derive;

pub mod configuration;
mod errors;
pub mod gist;
pub mod language;
pub mod the_way;
mod utils;
