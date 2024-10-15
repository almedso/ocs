use crate::cli::{CommonArgs, GitArgs};
use crate::progress;
use git2::Error;
use git2::{Commit, ObjectType, Oid, Repository};
use std::cmp::{Ord, Ordering};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::str;

use crate::determine_commits_to_analyse;
#[allow(unused_imports)]
use crate::git_common_args_extension;

use log::info;

pub const COMMAND: &str = "revisions";

#[macro_export]
macro_rules! revisions_command {
    ($command_builder:expr) => {
        $command_builder.subcommand(git_common_args_extension(
            Command::new(subcommands::revisions::COMMAND).about("Git revision frequency"),
        ))
    };
}

#[derive(Clone)]
struct EntryRevisions {
    name: PathBuf,
    revisions: BTreeSet<Oid>,
}

impl EntryRevisions {
    pub fn new(name: PathBuf) -> EntryRevisions {
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

fn analyze_tree_object(
    repo: &Repository,
    tree: git2::Tree,
    path: PathBuf,
    entries: &mut BTreeSet<EntryRevisions>,
) {
    for entry in tree.iter() {
        if let Some(n) = entry.name() {
            let mut p = path.clone();
            p.push(n.to_owned());
            if entry.kind() == Some(ObjectType::Blob) {
                let entry_revision = EntryRevisions::new(p);
                entries.insert(entry_revision.clone());
                if let Some(entry_revision) = entries.get(&entry_revision) {
                    let mut e = entry_revision.clone();
                    e.revisions.insert(entry.id().clone());
                    entries.replace(e);
                }
            } else {
                if entry.kind() == Some(ObjectType::Tree) {
                    let tree = repo.find_tree(entry.id()).unwrap();
                    analyze_tree_object(repo, tree, p, entries);
                }
            }
        }
    }
}

fn analyse_entries_in_commit(
    repo: &Repository,
    commit: &Commit,
    path: PathBuf,
    entries: &mut BTreeSet<EntryRevisions>,
) {
    analyze_tree_object(
        repo,
        commit.tree().expect("Every commit has a tree object"),
        path,
        entries,
    );
}

pub fn run(common_args: CommonArgs, git_args: GitArgs) -> Result<(), Error> {
    info!("Run git revision frequencies");
    let repo = Repository::open(common_args.project_dir.clone())?;

    let revwalk = determine_commits_to_analyse(&repo, git_args)?;
    let mut entries: BTreeSet<EntryRevisions> = BTreeSet::new();
    progress::start_commit_analysing();

    for commit in revwalk {
        progress::increment_commit_analysing();
        let commit = commit?;
        let path = PathBuf::from(common_args.project_dir.clone());
        analyse_entries_in_commit(&repo, &commit, path, &mut entries);
    }
    progress::finish_commit_analysing();

    println!("entry,n-revs");
    for entry_revision in entries {
        println!(
            "{},{}",
            entry_revision.name.display(),
            entry_revision.revisions.len()
        );
    }

    Ok(())
}
