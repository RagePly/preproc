#![warn(missing_docs)]
//! This crate provides functionality to build a simple source-code preprocessor.
//! 
//! In the current state, the methods available are only capable of generating 
//! a single file from several source files through include-directives. The 
//! function [`deps::generate_deptree()`] traverses these include-directives recursively 
//! to create a dependency-tree, [`deps::DepTree`]. This dependency-tree can then be
//! used to build a source file through [`build_file()`].
//! 
//! Included are simple implementations for [`filefetcher::FileFetcher`] and [`process::ParseLine`] 
//! (see [`filefetcher::FilesystemFetcher`], [`filefetcher::MemoryFetcher`] and 
//! [`process::CommentParser`]) that can help build a very simple preprocessor.
//! 
//! # Example
//! The following code would be able to process the local file `main.file` and look for
//! any includes in the folder `include_folder`.
//! 
//! ```no_run
//! # fn main() -> Result<(), String> {
//! use preproc::deps::{generate_deptree, DepTree};
//! use preproc::filefetcher::FilesystemFetcher;
//! use preproc::process::CommentParser;
//! use preproc::build_file;
//! 
//! // This parser will treat all lines starting with `//&` as a preproc statement
//! let parser = CommentParser::from("//");
//! // This fetcher works on the local filesystem
//! let mut fetcher = FilesystemFetcher::new();
//! // The fetcher will look in `include_folder/` while searching for files
//! fetcher.add_path("include_folder");
//! 
//! // Traverse all files recursively and build a dependency-tree
//! let (_, deptree) = generate_deptree("main.file", &mut fetcher, &parser)?;
//! // Generate source that satisfies all dependencies listed in deptree
//! let generated_source = build_file(&deptree)?;
//! # Ok(())
//! # }
//! ```

use std::collections::HashSet;

pub mod process;
pub mod filefetcher;
pub mod deps;

use deps::InsertionPoint;
use deps::DepTree;
use filefetcher::FileFetcher;

const JOIN_SEPARATOR: &'static str = "\n";

/// Generates a new file from a dependency tree (see [`deps::DepTree`]) by satisfying
/// all dependencies.
/// # Error
/// Fails if `deptree` is empty.
pub fn build_file(deptree: &DepTree) -> Result<String, String> {
    if deptree.is_empty() {
        return Err("empty dependency tree".into());
    }
    // figure out top scope
    let mentioned: HashSet<_> = deptree
        .values()
        .map(|d| d.points.iter().map(|p| &p.fname))
        .flatten()
        .collect();
    let sources: HashSet<_> = deptree.keys().collect();

    let not_mentioned: Vec<_> = sources.difference(&mentioned).map(|s| *s).collect();
    
    let roots: Vec<_> = if not_mentioned.is_empty() {
        deptree.keys().take(1).collect()
    } else {
        not_mentioned
    };

    let mut acc = Vec::new();
    let mut visited = HashSet::new();

    for root in roots {
        subbuild_file(root.clone(), &mut acc, deptree, &mut visited);
    }

    Ok(acc.as_slice().join(JOIN_SEPARATOR))
}

fn subbuild_file<'a>(fname: String, acc: &mut Vec<&'a str>, deptree: &'a DepTree, visited: &mut HashSet<String>) {
    // get lines and insert-points
    let deps::FileData { source, points } = deptree.get(&fname).unwrap();
    let mut lines = source.lines().enumerate();
    visited.insert(fname);

    for InsertionPoint {fname: subname, index} in points {
        loop {
            let (i, line) = lines.next().unwrap();  // no insertion point should have an index 
                                                    // greater than the nr of lines in a file
            if i == *index {
                if !visited.contains(subname) {
                    subbuild_file(subname.clone(), acc, deptree, visited);
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