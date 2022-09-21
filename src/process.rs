/// Alias for a list of views into each line of a source.
pub type Lines<'a> = Vec<&'a str>;

/// Representation of a source as a list of lines.
pub struct Source<'a>(Lines<'a>);

impl<'a> Source<'a>
{
    /// Split the `source` into lines.
    pub fn from_str(source: &'a str) -> Source<'a>
    {
        Source (source.lines().collect())
    }

    /// Process the [`Source`] using `parser`, see [`ParseLine`], to parse each line.
    pub fn process<T>(&self, parser: &T) -> Result<PreprocessPoints<'a>, String>
    where
        T: ParseLine
    {
        let mut pp = PreprocessPoints::new();
        for (i, line) in self.0.iter().enumerate() {
            if let Some(parsed_line) = parser.parse_line(line) {
                match parsed_line {
                    Ok(com) => { pp.push(i, com); },
                    Err(s) => { return Err(format!("line {}: {}", i, s)); }
                }
            }
        }

        Ok(pp)
    }
}



#[derive(Debug)]
/// A preprocessing-command.
pub enum PreprocCommand<'a> {
    /// (Global)-include directive.
    Include(&'a str),
    /// Local-include directive.
    IncludeLocal(&'a str),
}

/// A trait for parsing a single line of a source. 
pub trait ParseLine {
    /// Parse a single line. Returns `Some` if the line represents a preprocessing-command, with the contained value
    /// being either an `Ok(command)` if the command parsed sucessfully, otherwise an `Err(text)` with `text` explaining
    /// the cause of the failure. Returns `None` if the line was not a preprocessing-command.
    fn parse_line<'a>(&self, line: &'a str) -> Option<Result<PreprocCommand<'a>, String>>;
}

/// A parser that will explore the comments of a source, looking for the character `&` appended after the 
/// comment-string as a start of a preprocessing-command.
/// 
/// # Syntax
/// ```bnf
///     <comment-str> "&" <ws> "include" <ws> ("<" <global-filename> ">" | "\"" <local-filename> "\"")
/// ```
/// 
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
            if let Some(com) = rem.trim_start().strip_prefix("include").map(|s| s.trim()) { 
                if let Some(filename) = com.strip_prefix("<").and_then(|r| r.strip_suffix(">")) {
                    Some(Ok(PreprocCommand::Include(filename)))
                } else if let Some(filename) = com.strip_prefix("\"").and_then(|r| r.strip_suffix("\"")) {
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
/// Parsed include point
pub enum IncludePoint<'a> {
    /// A local include directive, with linenumber and filename to be included
    Local(usize, &'a str),
    /// A global include directive, with linenumber and filename to be included
    Global(usize, &'a str),
}

#[derive(Debug)]
/// A wrapper around a vector containing preprocess-commands and at what linenumber the command was called from
pub struct PreprocessPoints<'a>(Vec<(usize, PreprocCommand<'a>)>);

impl<'a> PreprocessPoints<'a> {
    /// Initializes the struct
    pub fn new() -> PreprocessPoints<'a> {
        PreprocessPoints { 0: Vec::new() }
    }

    /// Add a command and at what linenumber it was called
    pub fn push(&mut self, i: usize, com: PreprocCommand<'a>) {
        self.0.push((i, com))
    }

    /// Extract only the [`IncludePoint`]s.
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
