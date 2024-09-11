use crate::cli::{CommonArgs, GitArgs};
use git2::Error;
use git2::{Commit, ObjectType, Oid, Repository, TreeWalkMode, TreeWalkResult};
use std::cmp::{Ord, Ordering};
use std::collections::BTreeSet;
use std::str;

use crate::determine_commits_to_analyse;
#[allow(unused_imports)]
use crate::git_common_args_extension;

use log::info;

pub const COMMAND: &str = "hotspot";

#[macro_export]
macro_rules! hotspot_command {
    ($command_builder:expr) => {
        $command_builder.subcommand(git_common_args_extension(
            Command::new(subcommands::hotspot::COMMAND).about("Determine hotspots"),
        ))
    };
}

#[derive(Clone)]
struct EntryRevisions {
    name: String,
    revisions: BTreeSet<Oid>,
}

impl EntryRevisions {
    pub fn new(name: String) -> EntryRevisions {
        EntryRevisions {
            name,
            revisions: BTreeSet::new(),
        }
    }
}

impl Ord for EntryRevisions {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for EntryRevisions {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for EntryRevisions {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for EntryRevisions {}

fn analyse_entries_in_commit(commit: &Commit, entries: &mut BTreeSet<EntryRevisions>) {
    commit
        .tree()
        .expect("Every commit has a tree object")
        .walk(TreeWalkMode::PreOrder, |_, entry| {
            if entry.kind() == Some(ObjectType::Blob) {
                if let Some(n) = entry.name() {
                    let entry_revision = EntryRevisions::new(n.to_owned());
                    entries.insert(entry_revision.clone());
                    if let Some(entry_revision) = entries.get(&entry_revision) {
                        let mut e = entry_revision.clone();
                        e.revisions.insert(entry.id().clone());
                        entries.replace(e);
                    }
                }
            }
            TreeWalkResult::Ok
        })
        .unwrap();
}

pub fn run(common_args: CommonArgs, git_args: GitArgs) -> Result<(), Error> {
    info!("Run git revision summary");
    let repo = Repository::open(common_args.project_dir)?;

    let revwalk = determine_commits_to_analyse(&repo, git_args)?;
    let mut entries: BTreeSet<EntryRevisions> = BTreeSet::new();

    for commit in revwalk {
        let commit = commit?;
        analyse_entries_in_commit(&commit, &mut entries);
    }
    println!("entry,n-revs");
    for entry_revision in entries {
        println!("{},{}", entry_revision.name, entry_revision.revisions.len());
    }

    Ok(())
}
