
extern crate regex;
extern crate glob;
#[macro_use]
extern crate lazy_static;
use glob::Pattern;
use std::path::Path;
use std::fs::File;
use std::io::BufReader;
use std::io::{BufRead, Read};
use std::str::FromStr;

use regex::Regex;

#[derive(Debug, PartialEq)]
pub enum Owner {
    Username(String),
    Team(String),
    Email(String),
}

impl FromStr for Owner {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref TEAM: Regex = Regex::new(r"^@\S+/\S+").unwrap();
            static ref USERNAME: Regex = Regex::new(r"^@\S+").unwrap();
            static ref EMAIL: Regex = Regex::new(r"^\S+@\S+").unwrap();
        }
        if TEAM.is_match(s) {
            Ok(Owner::Team(s.into()))
        } else if USERNAME.is_match(s) {
            Ok(Owner::Username(s.into()))
        } else if EMAIL.is_match(s) {
            Ok(Owner::Email(s.into()))
        } else {
            Err(String::from("not an owner"))
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Owners {
    paths: Vec<(Pattern, Vec<Owner>)>,
}

impl Owners {
    pub fn owners<P>(&self, path: P) -> Option<&Vec<Owner>>
    where
        P: AsRef<Path>,
    {
        self.paths
            .iter()
            .filter_map(|mapping| {
                let &(ref pattern, ref owners) = mapping;
                if pattern.matches_path(path.as_ref()) {
                    Some(owners)
                } else {
                    None
                }
            })
            .next()
    }

    pub fn from_path<P>(path: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self::from_reader(File::open(path).unwrap())
    }

    pub fn from_reader<R>(read: R) -> Self
    where
        R: Read,
    {
        let mut paths = BufReader::new(read)
            .lines()
            .filter_map(Result::ok)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .fold(Vec::new(), |mut paths, line| {
                let mut elements = line.split_whitespace();
                if let Some(path) = elements.next() {
                    let owners = elements.fold(Vec::new(), |mut result, owner| {
                        if let Ok(owner) = owner.parse() {
                            result.push(owner)
                        }
                        result
                    });
                    paths.push((Pattern::new(path).unwrap(), owners))
                }
                paths
            });
        // last match takes precedence
        paths.reverse();
        Owners { paths: paths }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    const EXAMPLE: &str = r"# Lines starting with '#' are comments.
# Each line is a file pattern followed by one or more owners.

# These owners will be the default owners for everything in the repo.
*       @defunkt

# Order is important. The last matching pattern has the most precedence.
# So if a pull request only touches javascript files, only these owners
# will be requested to review.
*.js    @octocat @github/js

# You can also use email addresses if you prefer.
docs/*  docs@example.com
";
    #[test]
    fn from_reader_parses() {
        let owners = Owners::from_reader(EXAMPLE.as_bytes());
        assert_eq!(
            owners,
            Owners {
                paths: vec![
                    (
                        Pattern::new("docs/*").unwrap(),
                        vec![Owner::Email("docs@example.com".into())]
                    ),
                    (
                        Pattern::new("*.js").unwrap(),
                        vec![
                            Owner::Username("@octocat".into()),
                            Owner::Team("@github/js".into()),
                        ]
                    ),
                    (
                        Pattern::new("*").unwrap(),
                        vec![Owner::Username("@defunkt".into())]
                    ),
                ],
            }
        )
    }

    #[test]
    fn owners_owns_wildcard() {
        let owners = Owners::from_reader(EXAMPLE.as_bytes());
        assert_eq!(
            owners.owners("foo/bar.txt"),
            Some(&vec![Owner::Username("@defunkt".into())])
        )
    }

    #[test]
    fn owners_owns_last_match_wins() {
        let owners = Owners::from_reader(EXAMPLE.as_bytes());
        assert_eq!(
            owners.owners("docs/foo.js"),
            Some(&vec![Owner::Email("docs@example.com".into())])
        )
    }
}
