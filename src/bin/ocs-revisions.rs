use clap::Parser;
use git2::Error;
use git2::{Commit, ObjectType, Oid, Repository, Time, TreeWalkMode, TreeWalkResult};
use std::cmp::{Ord, Ordering};
use std::collections::BTreeSet;
use std::str;
use time::{error, macros::format_description, Date, OffsetDateTime, UtcOffset};

#[derive(Parser)]
#[command(version, about, name = "ocs-revisions")]
struct Args {
    #[arg(name = "pattern", long = "grep")]
    /// pattern to filter commit messages by
    flag_grep: Option<String>,
    #[arg(name = "dir", long = "git-dir", short = 'C')]
    /// alternative git directory to use
    flag_git_dir: Option<String>,
    #[arg(long = "before", short = 'b', value_parser = parse_iso_date_and_convert_to_git_time)]
    /// Only consider commits before the given date in the form YYYY-MM-DD
    before: Option<Time>,
    #[arg(long = "after", short = 'a', value_parser = parse_iso_date_and_convert_to_git_time)]
    /// Only consider commits after the given date in the form YYYY-MM-DD
    after: Option<Time>,
    #[arg(name = "commit")]
    /// commit or list of commits to be considered. A commit can be a
    /// sha-1, a branch, a tag or a refspec
    arg_commit: Vec<String>,
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

fn run(args: &Args) -> Result<(), Error> {
    let path = args.flag_git_dir.as_ref().map(|s| &s[..]).unwrap_or(".");
    let repo = Repository::open(path)?;
    let mut revwalk = repo.revwalk()?;

    // Prepare the revwalk based on CLI parameters
    revwalk.set_sorting(git2::Sort::NONE)?;
    for commit in &args.arg_commit {
        if commit.starts_with('^') {
            let obj = repo.revparse_single(&commit[1..])?;
            revwalk.hide(obj.id())?;
            continue;
        }
        let revspec = repo.revparse(commit)?;
        if revspec.mode().contains(git2::RevparseMode::SINGLE) {
            revwalk.push(revspec.from().unwrap().id())?;
        } else {
            let from = revspec.from().unwrap().id();
            let to = revspec.to().unwrap().id();
            revwalk.push(to)?;
            if revspec.mode().contains(git2::RevparseMode::MERGE_BASE) {
                let base = repo.merge_base(from, to)?;
                let o = repo.find_object(base, Some(ObjectType::Commit))?;
                revwalk.push(o.id())?;
            }
            revwalk.hide(from)?;
        }
    }
    if args.arg_commit.is_empty() {
        revwalk.push_head()?;
    }

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

    // Filter our revwalk based on the CLI parameters
    macro_rules! filter_try {
        ($e:expr) => {
            match $e {
                Ok(t) => t,
                Err(e) => return Some(Err(e)),
            }
        };
    }
    let revwalk = revwalk
        .filter_map(|id| {
            let id = filter_try!(id);
            let commit = filter_try!(repo.find_commit(id));

            if !commit_message_matches(commit.message(), &args.flag_grep) {
                return None;
            }
            if !commit_timestamp_is_in_range(commit.time(), args.before, args.after) {
                return None;
            }
            Some(Ok(commit))
        })
        .take(!0);

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

fn commit_message_matches(msg: Option<&str>, grep: &Option<String>) -> bool {
    match (grep, msg) {
        (&None, _) => true,
        (&Some(_), None) => false,
        (&Some(ref s), Some(msg)) => msg.contains(s),
    }
}

fn commit_timestamp_is_in_range(
    timestamp: Time,
    before: Option<Time>,
    after: Option<Time>,
) -> bool {
    if let Some(b) = before {
        if b < timestamp {
            return false;
        }
    }
    if let Some(a) = after {
        return a < timestamp;
    }
    true
}

fn main() {
    let args = Args::parse();
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
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
