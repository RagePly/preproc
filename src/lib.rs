use std::collections::HashSet;

pub mod process;
pub mod filefetcher;
pub mod deps;

use deps::InsertionPoint;
use deps::DepTree;
use filefetcher::FileFetcher;

const JOIN_SEPARATOR: &'static str = "\n";

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