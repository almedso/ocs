use clap::{
    crate_authors, crate_description, crate_name, crate_version, value_parser, Arg, ArgAction,
    ArgMatches, Command,
};
use std::env;
use std::path::PathBuf;

use git2::Time;

use time::{error, macros::format_description, Date, OffsetDateTime, UtcOffset};
#[derive(Debug, Clone)]
pub struct CommonArgs {
    pub project_dir: String,
}

impl CommonArgs {
    pub fn new(project_dir: Option<&PathBuf>) -> Self {
        let project_dir = match project_dir {
            Some(x) => x.clone(),
            None => env::current_dir().unwrap(),
        };
        CommonArgs {
            project_dir: project_dir.into_os_string().into_string().unwrap().clone(),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct GitArgs {
    pub after: Option<Time>,
    pub before: Option<Time>,
    pub commit: Option<String>,
    pub commit_msg_grep: Option<String>,
}

impl GitArgs {
    pub fn from_cli_args(git_matches: &ArgMatches) -> Self {
        GitArgs {
            after: git_matches.get_one::<Time>("after").copied(),
            before: git_matches.get_one::<Time>("before").copied(),
            ..Default::default()
        }
    }
}

pub fn setup_logger(verbose_option: u64) {
    use log::LevelFilter;

    let mut builder = env_logger::Builder::new();

    let filter_level = match verbose_option {
        1 => LevelFilter::Warn,
        2 => LevelFilter::Info,
        3 => LevelFilter::Debug,
        _ => LevelFilter::Error,
    };

    builder.filter(None, filter_level);
    builder.init();
}

pub fn common_builder() -> Command {
    Command::new(crate_name!())
    .author(crate_authors!("\n"))
    .version(crate_version!())
    .about(crate_description!())

    .arg(
        Arg::new("verbose")
            .long("verbose")
            .short('v')
            .action(ArgAction::Count)
            .help(
                "Set verbosity level written ALWAYS to stderr. Levels are:
                -v: warning,
                -vv: info,
                -vvv: debug",
            ),
    )
    .arg (
        Arg::new("project_dir")
        .long("project_dir")
        .short('C')
        .required(false)
        .value_parser(value_parser!(PathBuf))
        .help(
            "Project directory where the git repository exists and the sources are checked out.
            Default is the current directory"
        )
    )
}

pub fn git_common_args_extension(builder: Command) -> Command {
    builder
        .arg(
            Arg::new("before")
                .long("before")
                .short('b')
                .value_parser(parse_iso_date_and_convert_to_git_time)
                .help("Only consider commits before the given date in the form YYYY-MM-DD"),
        )
        .arg(
            Arg::new("after")
                .long("after")
                .short('a')
                .value_parser(parse_iso_date_and_convert_to_git_time)
                .help("Only consider commits after the given date in the form YYYY-MM-DD"),
        )
}

fn parse_iso_date_and_convert_to_git_time(arg: &str) -> Result<Time, error::Parse> {
    let format = format_description!("[year]-[month]-[day]");
    let date = Date::parse(arg, &format)?;
    let offset_date_time = OffsetDateTime::new_in_offset(
        date,
        time::Time::from_hms(0, 0, 0).unwrap(),
        UtcOffset::from_hms(0, 0, 0).unwrap(),
    );
    Ok(Time::new(offset_date_time.unix_timestamp(), 0))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn verify_cli_declaration() {
        use clap::CommandFactory;
        Args::command().debug_assert();
    }

    #[test]
    fn verify_commit_timestamp_is_in_range() {
        assert!(commit_timestamp_is_in_range(Time::new(0, 0), None, None));

        assert!(commit_timestamp_is_in_range(
            Time::new(0, 0),
            Some(Time::new(1, 0)),
            None
        ));
        assert!(!commit_timestamp_is_in_range(
            Time::new(0, 0),
            Some(Time::new(-1, 0)),
            None
        ));

        assert!(!commit_timestamp_is_in_range(
            Time::new(0, 0),
            None,
            Some(Time::new(1, 0))
        ));
        assert!(commit_timestamp_is_in_range(
            Time::new(0, 0),
            None,
            Some(Time::new(-1, 0))
        ));

        assert!(!commit_timestamp_is_in_range(
            Time::new(0, 0),
            Some(Time::new(1, 0)),
            Some(Time::new(1, 0))
        ));
        assert!(!commit_timestamp_is_in_range(
            Time::new(0, 0),
            Some(Time::new(-1, 0)),
            Some(Time::new(-1, 0))
        ));
        assert!(commit_timestamp_is_in_range(
            Time::new(0, 0),
            Some(Time::new(1, 0)),
            Some(Time::new(-1, 0))
        ));
    }
}
