type Lines<'a> = Vec<&'a str>;

pub struct Source<'a>(Lines<'a>);

impl<'a> Source<'a>
{
    pub fn from_str(source: &'a str) -> Source<'a>
    {
        Source (source.lines().collect())
    }

    pub fn process<T>(&self, parser: &T) -> Result<PreprocessPoints<'a>, String>
    where
        T: ParseLine
    {
        let mut pp = PreprocessPoints::new();
        for (i, line) in self.0.iter().enumerate() {
            if let Some(parsed_line) = parser.parse_line(line) {
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
    IncludeLocal(&'a str),
}

pub trait ParseLine {
    fn parse_line<'a>(&self, line: &'a str) -> Option<Result<PreprocCommand<'a>, String>>;
}

pub struct CommentParser(String);

impl From<&str> for CommentParser {
    fn from(s: &str) -> Self {
        CommentParser(s.to_owned())
    }
}

impl From<String> for CommentParser {
    fn from(s: String) -> Self {
        CommentParser(s)
    }
}

impl ParseLine for CommentParser {
    fn parse_line<'a>(&self, line: &'a str) -> Option<Result<PreprocCommand<'a>, String>> 
    {
        if let Some(rem) = line.strip_prefix(self.0.as_str()).and_then(|r| r.strip_prefix("&")) {
            if let Some(com) = rem.strip_prefix("include") { 
                if let Some(filename) = com.trim().strip_prefix("<").and_then(|r| r.strip_suffix(">")) {
                    Some(Ok(PreprocCommand::Include(filename)))
                } else if let Some(filename) = com.trim().strip_prefix("\"").and_then(|r| r.strip_suffix("\"")) {
                    Some(Ok(PreprocCommand::IncludeLocal(filename)))
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
}

#[derive(Debug, PartialEq, Eq)]
pub enum IncludePoint<'a> {
    Local(usize, &'a str),
    Global(usize, &'a str),
}

#[derive(Debug)]
pub struct PreprocessPoints<'a>(Vec<(usize, PreprocCommand<'a>)>);

impl<'a> PreprocessPoints<'a> {
    pub fn new() -> PreprocessPoints<'a> {
        PreprocessPoints { 0: Vec::new() }
    }
    pub fn get_include_points(&self) -> Vec<IncludePoint> {
        let mut include_points = Vec::new();
        for (linenr, command) in &self.0 {
            include_points.push(
                match command {
                    PreprocCommand::Include(filename) => IncludePoint::Global(*linenr, *filename),
                    PreprocCommand::IncludeLocal(filename) => IncludePoint::Local(*linenr, *filename),
                }
            );
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
        let source = Source::from_str(filestr);
        let pp = source.process::<CommentParser>(&"//".into()).expect("file is correct");
        let files = pp.get_include_points();

        assert_eq!(files.len(), 2);
        assert_eq!(files[0], IncludePoint::Global(0, "custom_file.c"));
        assert_eq!(files[1], IncludePoint::Global(3, "myfile.txt"));


        let other_file = "# This is a python file
from sys import argv

# the below file imports some file
#&include <other_file.py>

if __name__ == \"__main__\":
    print(*argv)
    other_function()
    raise SystemExit()
";
        let pp2 = Source::from_str(other_file).process::<CommentParser>(&"#".into()).expect("file is correct");
        let files = pp2.get_include_points();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0], IncludePoint::Global(4, "other_file.py"));
    }

    #[test]
    fn expect_error() {
        let wrong_file = "//&wrong <not read>";

        let source1 = Source::from_str(wrong_file);
        let pp1_error = source1.process::<CommentParser>(&"//".into()).expect_err("file shouldn't parse");
        assert_eq!(pp1_error, "line 0: invalid preproc statement `wrong <not read>`");

    }
}
