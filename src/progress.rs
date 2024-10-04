//! Handle progress visualization at the commandline
//!
//! ## Requirements
//!
//! - Progress visualization is a global cli flag
//! - Visualization will done on stderr in order to not interfer with output to
//!   stdout. (That is the default with the indicatif - crate)
//! - Progress visualization is to give feedback while big repositories are analyzed.
//!   After the analysis is done, all progress feedback shall be removed from
//!   terminal.
//! - Desired output is controlled via logging and verbose level
//!
//! ## Design Decisions
//!
//! - Progress visualization is process global (like logging) bound to the terminal
//!   Thus, all specific visualization is done in this module controlled by
//!   static resources
//!

use indicatif::{ProgressBar, ProgressStyle};

struct Progress {
    show_progress: bool,
    commit_analysing: Option<ProgressBar>,
}

static mut PROGRESS: Progress = Progress {
    show_progress: false,
    commit_analysing: None,
};

pub fn configure_progress_visualization(show_progress: bool) {
    unsafe {
        PROGRESS.show_progress = show_progress;
        PROGRESS.commit_analysing = None;
    }
}

pub fn start_commit_analysing() {
    unsafe {
        if PROGRESS.show_progress {
            let pb = ProgressBar::new(0);
            pb.set_style(ProgressStyle::with_template("{msg}: {pos:>7}").unwrap());
            pb.set_message("Analyse commits");

            PROGRESS.commit_analysing = Some(pb);
        }
    }
}

pub fn increment_commit_analysing() {
    unsafe {
        if let Some(pb) = &PROGRESS.commit_analysing {
            pb.inc_length(1);
        }
    }
}

pub fn finish_commit_analysing() {
    unsafe {
        if let Some(pb) = &PROGRESS.commit_analysing {
            pb.set_message("Commits analyzed");
            pb.finish_and_clear();
        }
    }
}
