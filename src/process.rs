type Lines<'a> = Vec<&'a str>;

pub struct Source<'a>
{
    lines: Lines<'a>,
    comment_type: String,
}

impl<'a> Source<'a>
{
    pub fn from_str(source: &'a str, comment_type: &str) -> Source<'a>
    {
        Source {
            lines: source.lines().collect(),
            comment_type: String::from(comment_type),
        }
    }

    pub fn process(&self) -> Result<PreprocessPoints<'a>, String>
    {
        let mut pp = PreprocessPoints::new();

        for (i, line) in self.lines.iter().enumerate() {
            if let Some(parsed_line) = parse_line(line, &self.comment_type) {
                match parsed_line {
                    Ok(com) => { pp.0.push((i, com)); },
                    Err(s) => { return Err(format!("line {}: {}", i, s)); }
                }
            }
        }

        Ok(pp)
    }
}

#[derive(Debug)]
pub enum PreprocCommand<'a> {
    Include(&'a str),
}

fn parse_line<'a>(line: &'a str, comment_type: &String) -> Option<Result<PreprocCommand<'a>, String>> 
{
    if let Some(rem) = line.strip_prefix(comment_type.as_str()).and_then(|r| r.strip_prefix("&")) {
        if let Some(com) = rem.strip_prefix("include") { 
            if let Some(filename) = com.trim().strip_prefix("<").and_then(|r| r.strip_suffix(">")) {
                Some(Ok(PreprocCommand::Include(filename)))
            } else {
                Some(Err(format!("invalid include statement `{}`", rem)))
            }
        } else {
            Some(Err(format!("invalid preproc statement `{}`", rem)))
        }
    } else {
        None
    }
}

#[derive(Debug)]
pub struct PreprocessPoints<'a>(Vec<(usize, PreprocCommand<'a>)>);

impl<'a> PreprocessPoints<'a> {
    pub fn new() -> PreprocessPoints<'a> {
        PreprocessPoints { 0: Vec::new() }
    }
    pub fn get_include_points(&self) -> Vec<(usize, &'a str)> {
        let mut include_points = Vec::new();
        for (linenr, command) in &self.0 {
            #[allow(irrefutable_let_patterns)]
            if let PreprocCommand::Include(filename) = command {
                include_points.push((*linenr, *filename));
            }
        }
        include_points
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fetch_filenames() {
        let filestr = "//&include <custom_file.c>
// This is a normal comment
#include <stdio.h>
//&include <myfile.txt>

int main() {
    printf(\"Hello World!\\n\");
    return 0;
}";
        let source = Source::from_str(filestr, "//");
        let pp = source.process().expect("file is correct");
        let files = pp.get_include_points();

        assert_eq!(files.len(), 2);
        assert_eq!(files[0], (0, "custom_file.c"));
        assert_eq!(files[1], (3, "myfile.txt"));


        let other_file = "# This is a python file
from sys import argv

# the below file imports some file
#&include <other_file.py>

if __name__ == \"__main__\":
    print(*argv)
    other_function()
    raise SystemExit()
";
        let pp2 = Source::from_str(other_file, "#").process().expect("file is correct");
        let files = pp2.get_include_points();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0], (4, "other_file.py"));
    }

    #[test]
    fn expect_error() {
        let wrong_file = "//&wrong <not read>";

        let source1 = Source::from_str(wrong_file, "//");
        let pp1_error = source1.process().expect_err("file shouldn't parse");
        assert_eq!(pp1_error, "line 0: invalid preproc statement `wrong <not read>`");

    }
}
