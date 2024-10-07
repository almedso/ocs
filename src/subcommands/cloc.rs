use crate::cli::CommonArgs;
use log::info;

pub const COMMAND: &str = "cloc";

use tokei::{ Config, Languages, Report};

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
    let paths = &[common_args.project_dir];
    let excluded = &["target", "build"];

    languages.get_statistics(paths, excluded, &config);

    println!("file name;lines of code, lines of comments, lines of space");
    for (_name, language) in languages {
        let reports: Vec<&Report> = language.reports.iter().collect();

        let (a, b): (Vec<&Report>, Vec<&Report>) =
            reports.iter().partition(|&r| r.stats.blobs.is_empty());

        for reports in &[&a, &b] {
            for report in reports.iter() {
                println!(
                    "{};{};{};{}",
                    report.name.display(),
                    report.stats.code,
                    report.stats.comments,
                    report.stats.blanks
                );
            }
        }
    }
}
