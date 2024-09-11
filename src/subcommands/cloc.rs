use crate::cli::CommonArgs;
use log::info;

pub const COMMAND: &str = "cloc";

// use tokei;

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

pub fn run(_common_args: CommonArgs) {
    info!("Run cloc - count lines of code");
}
