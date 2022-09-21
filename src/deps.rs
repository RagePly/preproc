use std::collections::HashMap;
use crate::{process::{ParseLine, Source, IncludePoint}, filefetcher::{FileName, FetchedFile}, FileFetcher};

#[derive(Debug)]
/// An object specifying where- and with what- to insert a dependency.
pub struct InsertionPoint {
    /// The linenumber of the insertion point
    pub index: usize,
    /// The filename of the source that should be included
    pub fname: String,
}

impl InsertionPoint {
    /// Creates a new [`InsertionPoint`].
    pub fn new(index: usize, fname: String) -> InsertionPoint {
        InsertionPoint { index, fname }
    }
}
#[derive(Debug)]
/// The actual source and insertion-points (see [`InsertionPoint`]) beloning to a file.
pub struct FileData {
    /// The utf-8 encoded string of the source 
    pub source: String,
    /// A list of insertion points (see [`InsertionPoint`])
    pub points: Vec<InsertionPoint>,
}

/// The dependency tree is implemented as a [`HashMap`] where the key corresponds to
/// the filename and the data is the actual source corresponding to the file along with
/// insertion points of other files in the tree.
pub type DepTree = HashMap<String, FileData>;

/// Generate a dependency-tree, starting from `seed` and using `parser`, [`FileFetcher`], to retrieve
/// sources and a `parser`, [`ParseLine`], to work on the files.
pub fn generate_deptree<F, P>(seed: &str, fetcher: &mut F, parser: &P) -> Result<(String, DepTree), String> 
where
    F: FileFetcher,
    P: ParseLine,
{
    let start = FileName::LocalTo(seed.to_owned(), "./".to_owned());
    let fname = fetcher.resolve_name(&start).ok_or(format!("file not found {}", start))?;
    let mut deptree = DepTree::new();
    build_deptree(start.clone(), &mut deptree, fetcher, parser)?;
    Ok((fname, deptree))
}

fn build_deptree<F, P>(fname: FileName, deptree: &mut DepTree, fetcher: &mut F, parser: &P) -> Result<(), String> 
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

/// Join two [`DepTree`]s.
pub fn join_deptrees(mut dep1: DepTree, dep2: DepTree) -> DepTree {
    dep1.extend(dep2.into_iter());
    dep1
}

/// Creates the source for a dependency file: `<file>: [<dependency1> [<dependency2> ...]]`
pub fn create_depfile(filename: &str, root: Option<&str>, points: &DepTree) -> String {

    let fnames: Vec<_> = points.keys().map(|k| match root {
        Some(r) => k.strip_prefix(r).or_else(|| {println!("failed to strip prefix"); None}).unwrap_or(k).to_owned(),
        None => k.to_owned()
    }).collect();
    format!("{}: {}", filename, fnames.as_slice().join(" ")).replace("\\", "/") //TODO: fix this quickfix used to make `gnu-make` understand paths
}
