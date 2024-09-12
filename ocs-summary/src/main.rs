use ocs::cli::git_common_args_extension;
use ocs::git::determine_commits_to_analyse;


use ocs::cli::{CommonArgs, GitArgs};
use ocs::cli::{common_builder, setup_logger};

use git2::Error;
use git2::{Commit, ObjectType, Oid, Repository, TreeWalkMode, TreeWalkResult};
use std::collections::BTreeSet;
use std::path::PathBuf;
use log::info;


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
    let mut number_of_commits = 0_u32;
    let mut authors = BTreeSet::new();
    let mut entries: BTreeSet<String> = BTreeSet::new();
    let mut entries_changed = BTreeSet::<Oid>::new();

    for commit in revwalk {
        number_of_commits += 1;
        let commit = commit?;
        let author = commit.author().to_owned();
        if let Some(name) = author.name() {
            authors.insert(name.to_owned());
        }
        analyse_entries_in_commit(&commit, &mut entries);
        analyse_entries_changed_in_commit(&commit, &mut entries_changed);
    }
    println!("statistic,value");
    println!("number-of-commits,{}", number_of_commits);
    println!("number-of-authors,{}", authors.len());
    println!("number-of-entries,{}", entries.len());
    println!("number-of-entries-changed,{}", entries_changed.len());
    Ok(())
}


fn main() {

    let builder = git_common_args_extension(common_builder());

    let matches = builder.get_matches();

    // handle common arguments
    let verbose = matches.get_count("verbose") as u64;
    setup_logger(verbose);
    let common_args = CommonArgs::new(matches.get_one::<PathBuf>("project_dir"));
    let git_args = GitArgs::from_cli_args(&matches);
    run(common_args, git_args).unwrap();
}
