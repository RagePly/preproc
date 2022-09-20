use std::collections::HashSet;
use std::fmt;

mod process;
mod filefetcher;
pub mod deps;

use deps::InsertionPoint;

pub use deps::{Dependencies, generate_dependencies, create_depfile};
pub use process::{ParseLine, CommentParser};
pub use filefetcher::{FileFetcher, FilesystemFetcher, MemoryFetcher};

const JOIN_SEPARATOR: &'static str = "\n";

#[derive(Debug)]
pub enum PreprocessError {
    FetchError(String),
    ParseError(String),
}

impl fmt::Display for PreprocessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FetchError(s) => write!(f, "FetchError({})", s),
            Self::ParseError(s) => write!(f, "ParseError({})", s),
        }
    }
}

pub fn build_file(dependencies: &Dependencies) -> Result<String, String> {
    if dependencies.is_empty() {
        return Err("empty dependency tree".into());
    }
    // figure out top scope
    let mentioned: HashSet<_> = dependencies
        .values()
        .map(|d| d.points.iter().map(|p| &p.fname))
        .flatten()
        .collect();
    let sources: HashSet<_> = dependencies.keys().collect();

    let not_mentioned: Vec<_> = sources.difference(&mentioned).map(|s| *s).collect();
    
    let roots: Vec<_> = if not_mentioned.is_empty() {
        dependencies.keys().take(1).collect()
    } else {
        not_mentioned
    };

    let mut acc = Vec::new();
    let mut visited = HashSet::new();

    for root in roots {
        subbuild_file(root.clone(), &mut acc, dependencies, &mut visited);
    }

    Ok(acc.as_slice().join(JOIN_SEPARATOR))
}

fn subbuild_file<'a>(fname: String, acc: &mut Vec<&'a str>, dependencies: &'a Dependencies, visited: &mut HashSet<String>) {
    // get lines and insert-points
    let deps::FileData { source, points } = dependencies.get(&fname).unwrap();
    let mut lines = source.lines().enumerate();
    visited.insert(fname);

    for InsertionPoint {fname: subname, index} in points {
        loop {
            let (i, line) = lines.next().unwrap();  // no insertion point should have an index 
                                                    // greater than the nr of lines in a file
            if i == *index {
                if !visited.contains(subname) {
                    subbuild_file(subname.clone(), acc, dependencies, visited);
                }
                break;
            } else {
                acc.push(line)
            }
        }   
    }

    // append remaining lines
    lines.for_each(|(_, l)| acc.push(l));
}