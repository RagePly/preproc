use std::env::args;
use std::fs::write;
use std::path::{Path, PathBuf};
use preproc::{filefetcher::FilesystemFetcher, deps::{generate_deptree, create_depfile}, build_file, process::CommentParser};
use normpath::PathExt;

enum NextIs {
    OutputFile,
    Comment,
    IncludePath,
    MakeOutput,
}

fn main() {
    use NextIs::*;

    let mut fetcher = FilesystemFetcher::new();
    let mut file = None;
    let mut output_file = None;
    let mut comment = None;
    let mut next_is = None;
    let mut makefile = false;
    let mut makeoutput = None;
    let mut verbose = false;


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
                MakeOutput => {
                    if makeoutput.is_none() {
                        makeoutput = Some(PathBuf::from(arg))
                    } else {
                        println!("can't specify multiple dependency file outputs")
                    }
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
            } else if let Some(make_opt) = option.strip_prefix("M") {
                makefile = true;
                if make_opt == "F" {
                    next_is = Some(MakeOutput);
                } else if make_opt != "D" {
                    println!("unknown option -M{}", make_opt);
                    return;
                }
            } else if option == "v" {
                verbose = true;
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
            MakeOutput => println!("dependency file not specified"),
        }
        return;
    }

    let root = Path::new("./").normalize().map_err(|e| {println!("error while normalizing path to output-file: {e}"); e}).ok();
    let root_repr = root.as_ref().and_then(|r| r.as_path().to_str());

    let file = if let Some(f) = file {
        f
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

    let out_file_rep = root
        .as_ref()
        .and_then(|r| {
            let file_norm = r.join(output_file.as_path());

            file_norm
            .as_path()
            .strip_prefix(r).ok()
            .and_then(|f| f.to_str())
            .map(|f| f.to_owned())
        })
        .unwrap_or(file.clone());

    let makeoutput = if makefile {
        if let Some(o) = makeoutput {
            o
        } else {
            let mut fpath = output_file.clone();
            fpath.set_extension("d");
            fpath
        }
    } else {
        PathBuf::new()
    };

    

    let comment: CommentParser = comment.unwrap_or(String::from("//")).into();

    match generate_deptree(&file, &mut fetcher, &comment) {
        Ok((_, deps)) => match build_file(&deps) {
            Ok(new_source) => match write(&output_file, new_source) {
                Ok(_) => {
                    if makefile {        
                        let makesource = create_depfile(&out_file_rep, root_repr, &deps);
                        if let Err(e) = write(makeoutput, makesource) {
                            println!("failed to write file: {:?}", e);
                        }
                    }
                    if verbose {
                        for subfile in deps.keys() {
                            println!("processed {}",
                                root
                                .as_ref()
                                .and_then(|r| Path::new(subfile).strip_prefix(r).ok())
                                .and_then(|p| p.to_str())
                                .unwrap_or(subfile)
                            )
                        }
                        println!("wrote to {}", out_file_rep);
                    }
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
}
