extern crate normpath;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::fs::read_to_string;
use std::iter;

use normpath::{PathExt, BasePath};

pub struct FetchedFile {
    pub name: String,
    pub content: String,
}

impl FetchedFile {
    pub fn new(name: String, content: String) -> FetchedFile {
        FetchedFile {name, content}
    }
}

#[derive(Debug, Clone)]
pub enum FileName {
    Global(String),
    LocalTo(String, String),
}

impl Display for FileName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileName::Global(gname) => write!(f, "<{}>", gname),
            FileName::LocalTo(lname, toname) => write!(f, "\"{}\" (local to {})", lname, toname)
        }
    }
}

pub trait FileFetcher {
    /// Returns a source as well as the resolved name
    fn fetch(&mut self, name: &FileName) -> Option<FetchedFile>;

    /// Tries to find the file and if it does, resolve an unique name
    fn resolve_name(&mut self, name: &FileName) -> Option<String>;
}

pub struct MemoryFetcher(HashMap<String, String>);

impl MemoryFetcher {
    pub fn new() -> MemoryFetcher {
        MemoryFetcher(HashMap::new())
    }

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
struct SearchPath(PathBuf);

impl SearchPath {
    fn new(path: &str) -> SearchPath {
        let pb = PathBuf::from(path);
        SearchPath(pb)
    }

    fn get_path(&self) -> &PathBuf {
        &self.0
    }
}

#[derive(Debug)]
pub struct FilesystemFetcher {
    search_order: Vec<SearchPath>,
    default: SearchPath,
}

impl FilesystemFetcher {
    pub fn new() -> FilesystemFetcher {
        FilesystemFetcher {
            search_order: vec![],
            default: SearchPath::new("./"),
        }
    }

    pub fn add_path(&mut self, p: &str) {
        self.search_order.push(SearchPath::new(p)); 
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
        match name {
            FileName::Global(name) => {
                let path = Path::new(&name);
                
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
                        let spath = BasePath::new(search_path.get_path().as_path()).unwrap();
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
