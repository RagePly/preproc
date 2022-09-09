extern crate indextree;
use indextree::{Arena, NodeId};
use std::collections::HashSet;
use std::fmt;

mod process;
mod filefetcher;

use process::Source;

pub use filefetcher::{FileFetcher, FilesystemFetcher, MemoryFetcher};

const JOIN_SEPARATOR: &'static str = "\n";

pub struct FileOptions<'a> {
    pub comment_str: &'a str,
}

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

fn subprocess_file<T>(filename: String, 
    prev_files: &mut FilenameSet, 
    arena: &mut FileArena,
    fetcher: &mut T,
    foptions: &FileOptions,
    insertion_point: usize,
) -> Result<NodeId, PreprocessError>
where
    T: FileFetcher
{
    // fetch file
    let file_and_source = fetcher
        .fetch(filename.as_str())
        .ok_or(PreprocessError::FetchError(format!("file not found \"{}\"", &filename)))?;
    
    let resolved_filename = file_and_source.name;
    let source = file_and_source.content;

    // process file
    let processed = Source::from_str(source.as_str(), foptions.comment_str)
        .process()
        .or_else(|e| Err(PreprocessError::ParseError(e)))?;

    // fetch includes
    let includes: Vec<_> = processed
        .get_include_points()
        .into_iter() 
        .map(|(i, f)| (i, f.to_owned())) 
        .collect();

    // append file to visited files
    prev_files.insert(resolved_filename);

    // add file to arena
    let this_file = arena.new_node(FileData::new(insertion_point, source));
    
    // loop through files and process unvisited ones
    for (linenr, next_filename) in includes.into_iter() {
        let resolved_next_filename = fetcher.resolve_name(&next_filename)
            .ok_or(PreprocessError::FetchError(format!("file not found \"{}\"", &next_filename)))?;

        if !prev_files.contains(&resolved_next_filename) { // here is the error
            this_file.append(
                subprocess_file(next_filename,
                    prev_files,
                    arena,
                    fetcher,
                    foptions,
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

pub fn process_file<T>(filename: String, fetcher: T, foptions: &FileOptions) -> Result<ProcessResult, PreprocessError>
where
    T: FileFetcher
{
    let mut prev_files = FilenameSet::new();
    let mut arena = Arena::new();
    let mut fetcher = fetcher;
    
    // top node
    let nodeid = subprocess_file(filename, &mut prev_files, &mut arena, &mut fetcher, foptions, 0)?;
    
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

        let file_options = FileOptions { comment_str: "//" };
        let new_file = process_file(String::from("a.txt"), mf, &file_options).unwrap();

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

        let file_options = FileOptions { comment_str: "//" };
        let new_file = process_file(String::from("a.txt"), fetcher, &file_options).unwrap();

        assert_eq!(new_file,
"File a.txt begin
File b.txt begin
File c.txt begin
File c.txt end
File b.txt end
File a.txt end");


        let mut fetcher = FilesystemFetcher::new();
        fetcher.add_path("./test");
        let new_file = process_file(String::from("c.txt"), fetcher, &file_options).unwrap();

        assert_eq!(new_file,
"File c.txt begin
File a.txt begin
File b.txt begin
File b.txt end
File a.txt end
File c.txt end");
    }
}
