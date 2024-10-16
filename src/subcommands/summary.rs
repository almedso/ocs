use crate::cli::{CommonArgs, GitArgs, OutputFormatter};
use git2::{Commit, ObjectType, Oid, Repository, TreeWalkMode, TreeWalkResult};

use serde::Serialize;
use std::collections::BTreeSet;
use std::str;
use std::{error::Error, io::Write};

use crate::determine_commits_to_analyse;
#[allow(unused_imports)]
use crate::git_common_args_extension;
use crate::progress;

use log::info;

pub const COMMAND: &str = "summary";

#[macro_export]
macro_rules! summary_command {
    ($command_builder:expr) => {
        $command_builder.subcommand(git_common_args_extension(
            Command::new(subcommands::summary::COMMAND).about("Git repository summary"),
        ))
    };
}

fn analyse_entries_in_commit(commit: &Commit, entries: &mut BTreeSet<String>) {
    commit
        .tree()
        .expect("Every commit has a tree object")
        .walk(TreeWalkMode::PreOrder, |_, entry| {
            if entry.kind() == Some(ObjectType::Blob) {
                if let Some(n) = entry.name() {
                    entries.insert(n.to_owned());
                }
            }
            TreeWalkResult::Ok
        })
        .unwrap();
}

fn analyse_entries_changed_in_commit(commit: &Commit, entries_changed: &mut BTreeSet<Oid>) {
    commit
        .tree()
        .expect("Every commit has a tree object")
        .walk(TreeWalkMode::PreOrder, |_, entry| {
            if entry.kind() == Some(ObjectType::Blob) {
                entries_changed.insert(entry.id().clone());
            }
            TreeWalkResult::Ok
        })
        .unwrap();
}

pub fn run(common_args: CommonArgs, git_args: GitArgs) -> Result<(), Box<dyn Error>> {
    info!("Run git revision summary");
    let repo = Repository::open(common_args.project_dir)?;

    let revwalk = determine_commits_to_analyse(&repo, git_args)?;

    // count various stuff
    let mut number_of_commits = 0_u64;
    let mut authors = BTreeSet::new();
    let mut entries: BTreeSet<String> = BTreeSet::new();
    let mut entries_changed = BTreeSet::<Oid>::new();

    progress::start_commit_analysing();
    for commit in revwalk {
        progress::increment_commit_analysing();
        number_of_commits += 1;
        let commit = commit?;
        let author = commit.author().to_owned();
        if let Some(name) = author.name() {
            authors.insert(name.to_owned());
        }
        analyse_entries_in_commit(&commit, &mut entries);
        analyse_entries_changed_in_commit(&commit, &mut entries_changed);
    }
    progress::finish_commit_analysing();

    let raw_data = SummaryRawData {
        no_of_commits: number_of_commits,
        no_of_authors: authors.len() as u64,
        no_of_entries: entries.len() as u64,
        no_of_entries_changed: entries_changed.len() as u64,
    };
    raw_data.output(common_args.format, common_args.output);

    Ok(())
}

struct SummaryRawData {
    no_of_commits: u64,
    no_of_authors: u64,
    no_of_entries: u64,
    no_of_entries_changed: u64,
}

#[derive(Serialize)]
struct Summary<'a> {
    statistics: &'a str,
    value: u64,
}

impl<'a> Summary<'a> {
    pub fn new(statistics: &'a str, value: u64) -> Self {
        Summary { statistics, value }
    }
}


impl OutputFormatter for SummaryRawData{

    fn csv_output(&self, writer: &mut dyn Write,) -> Result<(), Box<dyn Error>> {
        let mut wtr = csv::Writer::from_writer(writer);

        wtr.serialize(Summary::new("number-of-commits",self.no_of_commits))?;
        wtr.serialize(Summary::new("number-of-authors",self.no_of_authors))?;
        wtr.serialize(Summary::new("number-of-entries",self.no_of_entries))?;
        wtr.serialize(Summary::new(
            "number-of-entries-changed",
           self.no_of_entries_changed,
        ))?;

        wtr.flush()?;
        Ok(())
    }

    fn json_output(&self, writer: &mut dyn Write) -> Result<(), Box<dyn Error>> {
        let mut wtr = serde_json::Serializer::pretty(writer);

        let mut o = Vec::<Summary>::new();
        o.push(Summary::new("number-of-commits",self.no_of_commits));
        o.push(Summary::new("number-of-authors",self.no_of_authors));
        o.push(Summary::new("number-of-entries",self.no_of_entries));
        o.push(Summary::new(
            "number-of-entries-changed",
           self.no_of_entries_changed,
        ));

        o.serialize(&mut wtr)?;

        Ok(())
    }
}


