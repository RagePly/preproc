use std::env::args;
use std::fs::write;
use std::path::PathBuf;
use preproc::{FilesystemFetcher, generate_dependencies, build_file, CommentParser};

enum NextIs {
    OutputFile,
    Comment,
    IncludePath,
}

fn main() {
    use NextIs::*;

    let mut fetcher = FilesystemFetcher::new();
    let mut file = None;
    let mut output_file = None;
    let mut comment = None;
    let mut next_is = None;


    for arg in args().skip(1) {
        if let Some(n) = &next_is {
            match n {
                OutputFile => { 
                    if output_file.is_none() {
                        output_file = Some(PathBuf::from(arg));
                    } else {
                        println!("can't specify multiple output-files");
                        return;
                    }
                }
                Comment => {
                    if comment.is_none() {
                        comment = Some(arg); 
                    } else {
                        println!("can't specify multiple output-files");
                        return;
                    }
                }
                IncludePath => {
                    fetcher.add_path(arg.as_str());
                }
            }
            next_is = None;
            continue;
        }

        let arg_str = arg.as_str();
        if let Some(option) = arg_str.strip_prefix("-") {
            if let Some(new_dir) = option.strip_prefix("I") {
                if new_dir.is_empty() {
                    next_is = Some(IncludePath);
                } else {
                    fetcher.add_path(new_dir);
                }
            } else if let Some(comment_str) = option.strip_prefix("c") {
                if comment_str.is_empty() {
                    next_is = Some(Comment);
                } else {
                    if comment.is_none() {
                        comment = Some(comment_str.to_owned());
                    } else {
                        println!("comment can't be specified twice");
                        return;
                    }
                }
            } else if option == "o" {
                next_is = Some(OutputFile);
            } else {
                println!("unknown option -{}", option);
                return;
            }
        } else if file.is_none() {
            file = Some(arg); 
        } else {
            println!("invalid argument {}", arg);
            return;
        }
    }

    if let Some(next) = next_is {
        print!("unfinished argument: ");
        match next {
            OutputFile => println!("output file not specified"),
            Comment => println!("comment not supplied"),
            IncludePath => println!("include path not specified"),
        }
        return;
    }

    let file = if let Some(f) = file {
        f
        // let root_path = BasePath::new(Path::new("./")).unwrap();
        // if let Ok(file_path) = root_path.join(&f).normalize() {
        //     file_path.as_path().to_str().unwrap().to_owned()
        // } else {
        //     println!("file {} does not exist", f);
        //     return; 
        // }
    } else {
        println!("please supply a file");
        return;
    };

    let output_file = match output_file {
        Some(of) => of,
        None => {
            let mut fpath = PathBuf::from(&file);
            fpath.set_extension("i");
            fpath
        }
    };

    let comment: CommentParser = comment.unwrap_or(String::from("//")).into();

    match generate_dependencies(&file, &mut fetcher, &comment) {
        Ok((_, deps)) => match build_file(&deps) {
            Ok(new_source) => match write(&output_file, new_source) {
                Ok(_) => {
                    for subfile in deps.keys() {
                        println!("processed {}", &subfile)
                    }
                    println!("wrote to {}", output_file.to_str().unwrap())
                }
                Err(e) => {
                    println!("failed to write file: {:?}", e)
                }
            }
            Err(e) => {
                println!("error while building file: {}", e)
            }
        }
        Err(e) => {
            println!("error while generating/processing dependencies: {}", e)
        }
    }

    // match process_file(file, &mut fetcher, &comment ) {
    //     Ok(ProcessResult { file: new_file, included_files}) => match write(&output_file, new_file) {
    //         Ok(_) => { 
    //             for subfile in included_files {
    //                 println!("processed {}", subfile);
    //             }
    //             println!("wrote to {}", output_file.to_str().unwrap()); 
    //         }
    //         Err(e) => { println!("failed to write file: {:?}", e); }
    //     }
    //     Err(e) => {
    //         println!("error while processing file: {:?}", e);
    //     }
    // }
}
