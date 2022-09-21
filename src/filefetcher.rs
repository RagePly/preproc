extern crate normpath;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::fs::read_to_string;
use std::iter;

use normpath::{PathExt, BasePath};
/// The resolved filename and source of a file.
pub struct FetchedFile {
    /// Filename
    pub name: String,
    /// Source as utf-8
    pub content: String,
}

impl FetchedFile {
    /// Creates a new [`FetchedFile`]
    pub fn new(name: String, content: String) -> FetchedFile {
        FetchedFile {name, content}
    }
}

#[derive(Debug, Clone)]
/// A filename that includes the directive of where to find the file. See module [process](crate::process) for an explination for the directive.
pub enum FileName {
    /// The file should be searched for with a global context.
    Global(String),
    /// The file (first argument) should be local to the second argument.
    LocalTo(String, String),
}

#[doc(hidden)]
impl Display for FileName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileName::Global(gname) => write!(f, "<{}>", gname),
            FileName::LocalTo(lname, toname) => write!(f, "\"{}\" (local to {})", lname, toname)
        }
    }
}

/// Trait for resolving filenames and fetching file-sources.
pub trait FileFetcher {
    /// Returns a source as well as the resolved name
    fn fetch(&mut self, name: &FileName) -> Option<FetchedFile>;

    /// Tries to find the file and if it does, resolve an unique name
    fn resolve_name(&mut self, name: &FileName) -> Option<String>;
}

/// A fetcher for working with files stored in memory.
pub struct MemoryFetcher(HashMap<String, String>);

impl MemoryFetcher {
    /// Creates an empty [`MemoryFetcher`]
    pub fn new() -> MemoryFetcher {
        MemoryFetcher(HashMap::new())
    }

    /// Adds a file with name `name` and utf-8 data `data` to the [`MemoryFetcher`].
    pub fn add_file(&mut self, name: &str, data: &str) {
        self.0.insert(name.to_owned(), data.to_owned());
    }
}

impl FileFetcher for MemoryFetcher {
    fn fetch(&mut self, name: &FileName) -> Option<FetchedFile> {
        if let FileName::Global(name) = name {
            if let Some(source) = self.0.get(name) {
                Some(FetchedFile::new(name.clone(), source.clone()))
            } else {
                None
            } 
        } else {
            todo!("implement local-to for MemoryFetcher.fetch()")
        }
    }

    fn resolve_name(&mut self, name: &FileName) -> Option<String> {
        if let FileName::Global(name) = name {
            if self.0.contains_key(name) {
                Some(name.to_owned())
            } else {
                None
            }
        } else {
            todo!("implement local-to for MemoryFetcher.resolve_name()")
        }
    }
}

#[derive(Debug)]
/// A fetcher for working with files stored on the local filesystem.
pub struct FilesystemFetcher {
    search_order: Vec<PathBuf>,
    default: PathBuf,
}

impl FilesystemFetcher {
    /// Initializes an instance of [`FilesystemFetcher`]
    pub fn new() -> FilesystemFetcher {
        FilesystemFetcher {
            search_order: vec![],
            default: PathBuf::from("./"),
        }
    }

    /// Appends the path `p` to the list of include-directories. The directories are searched in the order of insertion.
    pub fn add_path(&mut self, p: &str) {
        self.search_order.push(PathBuf::from(p)); 
    }
}

impl FileFetcher for FilesystemFetcher {
    fn fetch(&mut self, name: &FileName) -> Option<FetchedFile> {
        if let Some(fname) = self.resolve_name(name) {
            let source = read_to_string(&fname).expect("file exists");
            Some(FetchedFile::new(fname, source))
        } else {
            None
        }
    }

    fn resolve_name(&mut self, name: &FileName) -> Option<String> {
        // TODO: too many unchecked "unwraps", implement solution if parsing fails
        // TODO: implement a Error type that more clearly explains the failure
        match name {
            FileName::Global(name) => {
                let path = Path::new(name);
                
                if path.is_absolute() {
                    // path is absolute, return wether the file exists
                    if path.is_file() {
                        Some(path.to_str().unwrap().to_owned())
                    } else {
                        None
                    }
                } else if path.starts_with("./") {
                    // the file has a forced relative path, normalize according to CWD
                    path.normalize()
                        .ok()
                        .map(|norm_str| norm_str.as_path().to_str().unwrap().to_owned())
                } else {
                    // the file has a flat type, perform search
                    for search_path in self.search_order
                                    .iter()
                                    .chain(iter::once(&self.default)) 
                    {
                        // Below is a likely fail, as it might be the first IO operation.
                        // TODO: this shouldn't be alowed to panic as it is a common problem
                        let spath = BasePath::new(search_path.as_path()).unwrap();
                        let joined_path = spath.join(path);

                        if let Some(cp) = joined_path.normalize().ok() {
                            let cp_str = cp.as_path().to_str().unwrap();
                            return Some(cp_str.to_owned());
                        }
                    }
                    None
                }
            }
            FileName::LocalTo(name, local) => {
                let path = Path::new(name);
                let local_path = BasePath::new(Path::new(local)).ok()?;
                let local_parent = if local_path.is_file() {
                    local_path.parent().ok()??
                } else {
                    &local_path
                };
                let joined_path = local_parent.join(path);
                joined_path.normalize().ok().map(|cp| cp.as_path().to_str().unwrap().to_owned())
            }
        }
    }
}
