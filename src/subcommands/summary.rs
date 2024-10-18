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

impl OutputFormatter for SummaryRawData {
    fn csv_output(&self, writer: &mut dyn Write) -> Result<(), Box<dyn Error>> {
        let mut wtr = csv::Writer::from_writer(writer);

        wtr.serialize(Summary::new("number-of-commits", self.no_of_commits))?;
        wtr.serialize(Summary::new("number-of-authors", self.no_of_authors))?;
        wtr.serialize(Summary::new("number-of-entries", self.no_of_entries))?;
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
        o.push(Summary::new("number-of-commits", self.no_of_commits));
        o.push(Summary::new("number-of-authors", self.no_of_authors));
        o.push(Summary::new("number-of-entries", self.no_of_entries));
        o.push(Summary::new(
            "number-of-entries-changed",
            self.no_of_entries_changed,
        ));

        o.serialize(&mut wtr)?;

        Ok(())
    }

    fn d3_html_output(&self, writer: &mut dyn Write) -> Result<(), Box<dyn Error>> {

        writer.write(D3_HTML_PREFIX.as_bytes())?;
        self.json_output(writer)?;
        writer.write(D3_HTML_POSTFIX.as_bytes())?;
        Ok(())
    }
}


const D3_HTML_PREFIX: &'static str = "
<!DOCTYPE html>
<div id=\"container\"></div>
<script src=\"https://cdn.jsdelivr.net/npm/d3@7\"></script>
<script type=\"module\">

const data =
";

const D3_HTML_POSTFIX: &'static str = "
;

const width = 928;
  const height = width;
  const margin = 1; // to avoid clipping the root circle stroke
  const name = d => d.statistics.split('.').pop(); // 'Strings' of 'flare.util.Strings'
  const group = d => d.statistics.split('.')[1]; // 'util' of 'flare.util.Strings'
  const names = d => name(d).split(/(?=[A-Z][a-z])|\\s+/g); // ['Legend', 'Item'] of 'flare.vis.legend.LegendItems'

  // Specify the number format for values.
  const format = d3.format(',d');

  // Create a categorical color scale.
  const color = d3.scaleOrdinal(d3.schemeTableau10);

  // Create the pack layout.
  const pack = d3.pack()
      .size([width - margin * 2, height - margin * 2])
      .padding(3);

  // Compute the hierarchy from the (flat) data; expose the values
  // for each node; lastly apply the pack layout.
  const root = pack(d3.hierarchy({children: data})
      .sum(d => d.value));

  // Create the SVG container.
  const svg = d3.create('svg')
      .attr('width', width)
      .attr('height', height)
      .attr('viewBox', [-margin, -margin, width, height])
      .attr('style', 'max-width: 100%; height: auto; font: 10px sans-serif;')
      .attr('text-anchor', 'middle');

  // Place each (leaf) node according to the layout’s x and y values.
  const node = svg.append('g')
    .selectAll()
    .data(root.leaves())
    .join('g')
      .attr('transform', d => `translate(${d.x},${d.y})`);

  // Add a title.
  node.append('title')
      .text(d => `${d.data.statistics}\n${format(d.value)}`);

  // Add a filled circle.
  node.append('circle')
      .attr('fill-opacity', 0.7)
      .attr('fill', d => color(group(d.data)))
      .attr('r', d => d.r);

  // Add a label.
  const text = node.append('text')
      .attr('clip-path', d => `circle(${d.r})`);

  // Add a tspan for each CamelCase-separated word.
  text.selectAll()
    .data(d => names(d.data))
    .join('tspan')
      .attr('x', 0)
      .attr('y', (d, i, nodes) => `${i - nodes.length / 2 + 0.35}em`)
      .text(d => d);

  // Add a tspan for the node’s value.
  text.append('tspan')
      .attr('x', 0)
      .attr('y', d => `${names(d.data).length / 2 + 0.35}em`)
      .attr('fill-opacity', 0.7)
      .text(d => format(d.value));

// Append the SVG element.
container.append(svg.node());

</script>
";
