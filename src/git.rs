use git2::{Commit, Error, ObjectType, Repository, Time};

use crate::cli::GitArgs;

pub fn determine_commits_to_analyse(
    repo: &Repository,
    args: GitArgs,
) -> Result<impl Iterator<Item = Result<Commit<'_>, Error>>, Error> {
    let mut revwalk = repo.revwalk()?;

    // Prepare the revwalk based on CLI parameters
    revwalk.set_sorting(git2::Sort::NONE)?;
    for commit in &args.commit {
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
    if args.commit.is_none() {
        revwalk.push_head()?;
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
    let revwalk = revwalk.filter_map(move |id| {
        let id = filter_try!(id);
        let commit = filter_try!(repo.find_commit(id));

        if !commit_message_matches(commit.message(), &args.commit_msg_grep) {
            return None;
        }
        if !commit_timestamp_is_in_range(commit.time(), args.before, args.after) {
            return None;
        }
        Some(Ok(commit))
    });

    Ok(revwalk)
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
