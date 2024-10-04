use crate::cli::CommonArgs;
use log::info;

pub const COMMAND: &str = "cloc";

use tokei::{Config, Languages, LanguageType};

#[macro_export]
macro_rules! cloc_command {
    ($command_builder:expr) => {
        $command_builder.subcommand(
            Command::new(subcommands::cloc::COMMAND)
                .about("Count lines of code, comments and empty lines")
                .after_help("Output is in csv only; first line is column header")
                .help_expected(true),
        )
    };
}

pub fn run(common_args: CommonArgs) {
    info!("Run cloc - count lines of code");

    let config = Config::default();
    let mut languages = Languages::new();
    let paths = &[ common_args.project_dir ];
    let excluded = &[ "target", "build"];

    languages.get_statistics(paths, excluded, &config);
    let rust = &languages[&LanguageType::Rust];

    println!("Lines of code: {}", rust.code);
}
