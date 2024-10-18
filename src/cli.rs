use clap::{
    builder::PossibleValue, crate_authors, crate_description, crate_name, crate_version,
    value_parser, Arg, ArgAction, ArgMatches, Command, ValueEnum,
};
use std::env;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::{error::Error, io, io::Write};

use git2::Time;

use time::{error, macros::format_description, Date, OffsetDateTime, UtcOffset};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum OutputFormat {
    Csv,
    Json,
    D3Graphics,
}

impl ValueEnum for OutputFormat {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            OutputFormat::Csv,
            OutputFormat::Json,
            OutputFormat::D3Graphics,
        ]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(match self {
            OutputFormat::Csv => PossibleValue::new("csv")
                .help("Character separated value, ',' is delimiter, 1st line is item name"),
            OutputFormat::Json => PossibleValue::new("json").help("JSON prettry printed output"),
            OutputFormat::D3Graphics => {
                PossibleValue::new("D3html").help("Render to D3 graphics as single html page")
            }
        })
    }
}

#[derive(Debug, Clone)]
pub struct CommonArgs<'a> {
    pub project_dir: String,
    pub format: OutputFormat,
    pub output: Option<&'a PathBuf>,
}

impl CommonArgs<'_> {
    pub fn new(project_dir: Option<&PathBuf>) -> Self {
        let project_dir = match project_dir {
            Some(x) => x.clone(),
            None => env::current_dir().unwrap(),
        };
        CommonArgs {
            project_dir: project_dir.into_os_string().into_string().unwrap().clone(),
            format: OutputFormat::Csv,
            output: None,
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
        Arg::new("DIRECTORY")
        .long("project_dir")
        .short('C')
        .required(false)
        .value_parser(value_parser!(PathBuf))
        .help(
            "Project directory where the git repository exists and the sources are checked out.
            Default is the current directory"
        )
    )
    .arg (
        Arg::new("progress")
        .long("progress")
        .short('p')
        .required(false)
        .action(ArgAction::SetTrue)
        .help(
            "Show progress"
        )
    )
    .arg (
        Arg::new("format")
        .long("format")
        .short('f')
        .default_value("csv")
        .value_parser(value_parser!(OutputFormat))
        .help(
            "Set the output format"
        )
    )
    .arg (
        Arg::new("FILE")
        .long("output")
        .short('o')
        .required(false)
        .value_parser(value_parser!(PathBuf))
        .help(
            "Write the output to a file instead of <stdout>"
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

pub trait OutputFormatter {
    fn csv_output(&self, writer: &mut dyn Write) -> Result<(), Box<dyn Error>>;
    fn json_output(&self, writer: &mut dyn Write) -> Result<(), Box<dyn Error>>;
    fn d3_html_output(&self, writer: &mut dyn Write) -> Result<(), Box<dyn Error>>;

    fn output(&self, format: OutputFormat, target: Option<&PathBuf>) {
        let mut writer = match target {
            Some(path) => {
                let path = Path::new(&path);
                Box::new(File::create(&path).expect("Unable to create file")) as Box<dyn Write>
            }
            None => Box::new(io::stdout()) as Box<dyn Write>,
        };
        match format {
            OutputFormat::Csv => self.csv_output(&mut writer).unwrap(),
            OutputFormat::Json => self.json_output(&mut writer).unwrap(),
            OutputFormat::D3Graphics => {
                self.d3_html_output(&mut writer).unwrap();
            }
        }
    }
}
