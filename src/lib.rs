extern crate indextree;
use indextree::{Arena, NodeId};
use std::collections::HashSet;
use std::fmt;

mod process;
mod filefetcher;
pub mod deps;

use process::{Source, IncludePoint};
use filefetcher::FileName;
use deps::InsertionPoint;

pub use deps::{Dependencies, generate_dependencies};
pub use process::{ParseLine, CommentParser};
pub use filefetcher::{FileFetcher, FilesystemFetcher, MemoryFetcher};

const JOIN_SEPARATOR: &'static str = "\n";

#[derive(Debug)]
struct FileData {
    ln: usize,
    source: Option<String>
}

impl FileData {
    fn new(ln: usize, source: String) -> FileData {
        FileData {ln, source: Some(source)}
    }

    fn new_empty(ln: usize) -> FileData {
        FileData {ln, source: None}
    }
}

type FileArena = Arena<FileData>;
type FilenameSet = HashSet<String>;

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

pub fn subbuild_file<'a>(fname: String, acc: &mut Vec<&'a str>, dependencies: &'a Dependencies, visited: &mut HashSet<String>) {
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

fn subprocess_file<T, P>(filename: &FileName, 
    prev_files: &mut FilenameSet, 
    arena: &mut FileArena,
    fetcher: &mut T,
    parser: &P,
    insertion_point: usize,
) -> Result<NodeId, PreprocessError>
where
    T: FileFetcher,
    P: ParseLine,
{
    // fetch file
    let file_and_source = fetcher
        .fetch(filename)
        .ok_or(PreprocessError::FetchError(format!("file not found {:?}", filename)))?;
    
    let resolved_filename = file_and_source.name;
    let source = file_and_source.content;

    // process file
    let processed = Source::from_str(source.as_str())
        .process(parser)
        .or_else(|e| Err(PreprocessError::ParseError(e)))?;

    // fetch includes
    let includes: Vec<_> = processed
        .get_include_points()
        .into_iter() 
        .map(|ip| match ip {
            IncludePoint::Global(i, f) => (i, FileName::Global(f.to_owned())),
            IncludePoint::Local(i, f) => (i, FileName::LocalTo(f.to_owned(), resolved_filename.clone()))
        }) 
        .collect();

    // append file to visited files
    prev_files.insert(resolved_filename);

    // add file to arena
    let this_file = arena.new_node(FileData::new(insertion_point, source));
    
    // loop through files and process unvisited ones
    for (linenr, next_filename) in includes.into_iter() {
        let resolved_next_filename = fetcher.resolve_name(&next_filename)
            .ok_or(PreprocessError::FetchError(format!("file not found \"{:?}\"", next_filename)))?;

        if !prev_files.contains(&resolved_next_filename) { // here is the error
            this_file.append(
                subprocess_file(&next_filename,
                    prev_files,
                    arena,
                    fetcher,
                    parser,
                    linenr)?,
                arena);
        } else {
            this_file.append(arena.new_node(FileData::new_empty(linenr)), arena);
        }
    }

    return Ok(this_file);
}


pub struct ProcessResult {
    pub file: String,
    pub included_files: FilenameSet,
}

pub fn process_file<T, P>(filename: String, fetcher: &mut T, parser: &P) -> Result<ProcessResult, PreprocessError>
where
    T: FileFetcher,
    P: ParseLine,
{
    let mut prev_files = FilenameSet::new();
    let mut arena = Arena::new();
    
    // top node
    let nodeid = subprocess_file(&FileName::Global(filename.to_owned()), &mut prev_files, &mut arena, fetcher, parser, 0)?;
    
    // assemble
    let lines = assemble(nodeid, &arena);
    Ok(ProcessResult {
        file: lines.as_slice().join(JOIN_SEPARATOR),
        included_files: prev_files,
    })
}

fn assemble<'a>(root: NodeId, arena: &'a FileArena) -> Vec<&'a str> {
    // this data
    let data = arena.get(root).expect("node exists").get();

    let lines: Vec<_> = match &data.source {
        Some(s) => s.as_str().lines().collect(),
        None => { return Vec::new(); }
    };

    let mut remaining = lines.as_slice();
    let mut collector = Vec::new();
    let mut i = 0;

    for child in root.children(arena) {
        assert!(remaining.len() > 0);

        let child_data = arena.get(child).expect("exists").get();
        let (prev, more) = remaining.split_at(child_data.ln - i);

        i += prev.len() + 1;

        // append prev
        collector.extend(prev);
        collector.append(&mut assemble(child, arena));
        
        remaining = &more[1..];
    }

    collector.extend(remaining);

    collector
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assemble_abc() {
        let mut mf = MemoryFetcher::new();
        mf.add_file("a.txt", "File a.txt begin
//&include <b.txt>
File a.txt end");
        mf.add_file("b.txt", "File b.txt begin
//&include <c.txt>
File b.txt end");
        mf.add_file("c.txt", "File c.txt begin
//&include <a.txt>
File c.txt end");

        let ProcessResult{ file: new_file, .. }= process_file::<_, CommentParser>(String::from("a.txt"), &mut mf, &"//".into()).unwrap();

        assert_eq!(new_file,
"File a.txt begin
File b.txt begin
File c.txt begin
File c.txt end
File b.txt end
File a.txt end");
    }

    #[test]
    fn fetch_files() {
        let mut fetcher = FilesystemFetcher::new();
        fetcher.add_path("./test");

        let ProcessResult{ file: new_file, .. } = process_file::<_, CommentParser>(String::from("a.txt"), &mut fetcher, &"//".into()).unwrap();

        assert_eq!(new_file,
"File a.txt begin
File b.txt begin
File c.txt begin
File c.txt end
File b.txt end
File a.txt end");


        let mut fetcher = FilesystemFetcher::new();
        fetcher.add_path("./test");
        let ProcessResult { file: new_file, .. } = process_file::<_, CommentParser>(String::from("c.txt"), &mut fetcher, &"//".into()).unwrap();

        assert_eq!(new_file,
"File c.txt begin
File a.txt begin
File b.txt begin
File b.txt end
File a.txt end
File c.txt end");
    }
}
