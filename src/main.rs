use structopt::StructOpt;

use the_way::language::get_languages;
use the_way::the_way::{cli::TheWayCLI, TheWay};

fn main() -> color_eyre::Result<()> {
    color_eyre::config::HookBuilder::blank()
        .display_env_section(false)
        .install()?;
    let languages_yml = include_str!("languages.yml");
    let languages = get_languages(languages_yml)?;
    let cli = TheWayCLI::from_args();
    TheWay::start(cli, languages)?;
    Ok(())
}
