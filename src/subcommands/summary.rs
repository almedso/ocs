use crate::cli::{CommonArgs, GitArgs};
use git2::Error;
use git2::{Commit, ObjectType, Oid, Repository, TreeWalkMode, TreeWalkResult};
use std::collections::BTreeSet;
use std::str;

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

pub fn run(common_args: CommonArgs, git_args: GitArgs) -> Result<(), Error> {
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

    println!("statistic,value");
    println!("number-of-commits,{}", number_of_commits);
    println!("number-of-authors,{}", authors.len());
    println!("number-of-entries,{}", entries.len());
    println!("number-of-entries-changed,{}", entries_changed.len());
    Ok(())
}
