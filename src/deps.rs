use std::collections::HashMap;
use crate::{process::{ParseLine, Source, IncludePoint}, filefetcher::{FileName, FetchedFile}, FileFetcher};

#[derive(Debug)]
pub struct InsertionPoint {
    pub index: usize,
    pub fname: String,
}

impl InsertionPoint {
    pub fn new(index: usize, fname: String) -> InsertionPoint {
        InsertionPoint { index, fname }
    }
}
#[derive(Debug)]
pub struct FileData {
    pub source: String,
    pub points: Vec<InsertionPoint>,
}

pub type Dependencies = HashMap<String, FileData>;

pub fn generate_dependencies<F, P>(seed: &str, fetcher: &mut F, parser: &P) -> Result<(String, Dependencies), String> 
where
    F: FileFetcher,
    P: ParseLine,
{
    let start = FileName::LocalTo(seed.to_owned(), "./".to_owned());
    let fname = fetcher.resolve_name(&start).ok_or(format!("file not found {}", start))?;
    let mut deptree = Dependencies::new();
    build_deptree(start.clone(), &mut deptree, fetcher, parser)?;
    Ok((fname, deptree))
}

fn build_deptree<F, P>(fname: FileName, deptree: &mut Dependencies, fetcher: &mut F, parser: &P) -> Result<(), String> 
where
    F: FileFetcher,
    P: ParseLine,
{
    // resolve name via fetcher
    let FetchedFile { name, content } = fetcher.fetch(&fname).ok_or(format!("file not found {}", fname))?;
    let mut fdata = FileData { source: content, points: Vec::new() };
    let source = Source::from_str(&fdata.source);
    
    // add this file to deptree, with placeholder file-data
    deptree.insert(name.clone(), FileData { source: String::new(), points: Vec::new()});

    // Process source and parse include points into insertion points
    for include_point in source.process(parser)?.get_include_points() {
        // parse type of include and point of insertion
        let (i, subname) = match include_point {
            IncludePoint::Global(i, f) => (i, FileName::Global(f.to_owned())),
            IncludePoint::Local(i, f) => (i, FileName::LocalTo(f.to_owned(), name.clone()))
        };

        // get resolved name
        let rname = fetcher.resolve_name(&subname).ok_or(format!("file not found {}", subname))?;

        // add to insertion-points if not yet present in file.
        if fdata.points.iter().all(|InsertionPoint{index: _, fname}| fname != &rname)
        {
            // also subprocess this tree if not yet done
            if !deptree.contains_key(&rname) {
                build_deptree(subname, deptree, fetcher, parser)?;
            }
            fdata.points.push(InsertionPoint {index: i, fname: rname});
        }
    };

    // update placeholder in deptree
    deptree.insert(name, fdata);

    Ok(())
}

/// Join two dependencytrees
pub fn join_dependencies(mut dep1: Dependencies, dep2: Dependencies) -> Dependencies {
    dep1.extend(dep2.into_iter());
    dep1
}

/// Creates the source for a dependency file: `<file>: [<dependency1> [<dependency2> ...]]`
pub fn create_depfile(filename: &str, root: Option<&str>, points: &Dependencies) -> String {

    let fnames: Vec<_> = points.keys().map(|k| match root {
        Some(r) => k.strip_prefix(r).or_else(|| {println!("failed to strip prefix"); None}).unwrap_or(k).to_owned(),
        None => k.to_owned()
    }).collect();
    format!("{}: {}", filename, fnames.as_slice().join(" ")).replace("\\", "/") //TODO: fix this quickfix used to make `gnu-make` understand paths
}
