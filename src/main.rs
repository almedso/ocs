pub mod cli;
pub mod git;

use crate::cli::git_common_args_extension;
use crate::git::determine_commits_to_analyse;

use clap::Arg;
use cli::{CommonArgs, GitArgs};

pub mod subcommands {
    #[macro_use]
    pub mod cloc;
    #[macro_use]
    pub mod hotspot;
    #[macro_use]
    pub mod revisions;
}

use crate::cli::{common_builder, setup_logger};
use clap::Command;
use std::ffi::OsString;
use std::path::PathBuf;

fn main() {
    let builder = common_builder()
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .subcommand(
            Command::new("config")
                .about("Configure general behavior and store it into the configuration file")
                .arg_required_else_help(true)
                .arg(Arg::new("config-key").help("config item to set")),
        );
    let builder = cloc_command!(builder);
    let builder = hotspot_command!(builder);
    let builder = summary_command!(builder);

    let matches = builder.get_matches();

    // handle common arguments
    let verbose = matches.get_count("verbose") as u64;
    setup_logger(verbose);
    let common_args = CommonArgs::new(matches.get_one::<PathBuf>("project_dir"));

    // process the respective subcommand
    match matches.subcommand() {
        Some(("config", sub_matches)) => {
            println!(
                "Pushing to {}",
                sub_matches.get_one::<String>("REMOTE").expect("required")
            );
        }
        Some((subcommands::cloc::COMMAND, _sub_matches)) => {
            subcommands::cloc::run(common_args);
        }
        Some((subcommands::hotspot::COMMAND, sub_matches)) => {
            let git_args = GitArgs::from_cli_args(sub_matches);
            subcommands::hotspot::run(common_args, git_args).unwrap();
        }
        Some((subcommands::revisions::COMMAND, sub_matches)) => {
            let git_args = GitArgs::from_cli_args(sub_matches);
            subcommands::revisions::run(common_args, git_args).unwrap();
        }

        // Further commands can be called as sub processes
        // Since they are not known at this point they will be not listed when calling help
        // see https://docs.rs/clap/latest/clap/_cookbook/git/index.html
        // for free subcommands
        Some((ext, sub_matches)) => {
            let args = sub_matches
                .get_many::<OsString>("")
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            println!("Calling out to {ext:?} with {args:?}");
        }
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable!()
    }
}
