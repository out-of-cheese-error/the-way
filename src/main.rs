use structopt::StructOpt;

use the_way::language::get_languages;
use the_way::the_way::{cli::TheWayCLI, TheWay};

fn main() -> color_eyre::Result<()> {
    let languages_yml = include_str!("languages.yml");
    let languages = get_languages(languages_yml)?;
    let cli = TheWayCLI::from_args();
    TheWay::start(cli, languages)?;
    Ok(())
}
