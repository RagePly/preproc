# preproc
This crate provides functionality to build a simple source-code preprocessor.

In the current state, the methods available are only capable of generating 
a single file from several source files through include-directives. The 
function `deps::generate_deptree()` traverses these include-directives recursively 
to create a dependency-tree, `deps::DepTree`. This dependency-tree can then be
used to build a source file through `build_file()`.
 
Included are simple implementations for `filefetcher::FileFetcher` and `process::ParseLine` 
(see `filefetcher::FilesystemFetcher`, `filefetcher::MemoryFetcher` and 
`process::CommentParser`) that can help build a very simple preprocessor.
 
## Example
The following code would be able to process the local file `main.file` and look for
any includes in the folder `include_folder`.
 
```rust
use preproc::deps::{generate_deptree, DepTree};
use preproc::filefetcher::FilesystemFetcher;
use preproc::process::CommentParser;
use preproc::build_file;

// This parser will treat all lines starting with `//&` as a preproc statement
let parser = CommentParser::from("//");
// This fetcher works on the local filesystem
let mut fetcher = FilesystemFetcher::new();
// The fetcher will look in `include_folder/` while searching for files
fetcher.add_path("include_folder");
 
// Traverse all files recursively and build a dependency-tree
let (_, deptree) = generate_deptree("main.file", &mut fetcher, &parser)?;
// Generate source that satisfies all dependencies listed in deptree
let generated_source = build_file(&deptree)?;
```