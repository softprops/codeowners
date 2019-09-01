//! Codeowners provides interfaces for resolving owners of paths within code
//! repositories using
//! Github [CODEOWNERS](https://help.github.com/articles/about-codeowners/)
//! files
//!
//! # Examples
//!
//! Typical use involves resolving a CODEOWNERS file, parsing it,
//! then querying target paths
//!
//! ```no_run
//! use std::env;
//!
//! fn main() {
//!   if let (Some(owners_file), Some(path)) =
//!      (env::args().nth(1), env::args().nth(2)) {
//!      let owners = codeowners::from_path(owners_file);
//!      match owners.of(&path) {
//!        None => println!("{} is up for adoption", path),
//!        Some(owners) => {
//!           for owner in owners {
//!             println!("{}", owner);
//!           }
//!        }
//!      }
//!   }
//! }
//! ```
#![allow(missing_docs)]

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

use glob::Pattern;
use lazy_static::lazy_static;
use regex::Regex;
use std::{
    fmt,
    fs::File,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    str::FromStr,
};

const CODEOWNERS: &str = "CODEOWNERS";

/// Various types of owners
///
/// Owners supports parsing from strings as well as displaying as strings
///
/// # Examples
///
/// ```rust
/// let raw = "@org/team";
/// assert_eq!(
///   raw.parse::<codeowners::Owner>().unwrap().to_string(),
///   raw
/// );
/// ```
#[derive(Debug, PartialEq)]
pub enum Owner {
    /// Owner in the form @username
    Username(String),
    /// Owner in the form @org/Team
    Team(String),
    /// Owner in the form user@domain.com
    Email(String),
}

impl fmt::Display for Owner {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        let inner = match *self {
            Owner::Username(ref u) => u,
            Owner::Team(ref t) => t,
            Owner::Email(ref e) => e,
        };
        f.write_str(inner.as_str())
    }
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

/// Pasrsed result of a GitHub `CODEOWNERS` file
#[derive(Debug, PartialEq)]
pub struct Owners {
    location: String,
    paths: Vec<(usize, Pattern, Vec<Owner>)>,
}

impl Owners {
    /// Path location of where this instance was loaded from.
    /// This may return `-` in cases where not loaded directly from a file
    pub fn location(&self) -> &str {
        self.location.as_str()
    }

    /// Resolve a list of owners matching a given path
    pub fn of<P>(
        &self,
        path: P,
    ) -> Option<&Vec<Owner>>
    where
        P: AsRef<Path>,
    {
        self.paths
            .iter()
            .filter_map(|mapping| {
                let &(_, ref pattern, ref owners) = mapping;
                let opts = glob::MatchOptions {
                    case_sensitive: false,
                    require_literal_separator: pattern.as_str().contains('/'),
                    require_literal_leading_dot: false,
                };
                if pattern.matches_path_with(path.as_ref(), opts) {
                    Some(owners)
                } else {
                    // this pattern is only meant to match
                    // direct children
                    if pattern.as_str().ends_with("/*") {
                        return None;
                    }
                    // case of implied owned children
                    // foo/bar @owner should indicate that foo/bar/baz.rs is
                    // owned by @owner
                    let mut p = path.as_ref();
                    while let Some(parent) = p.parent() {
                        if pattern.matches_path_with(parent, opts) {
                            return Some(owners);
                        } else {
                            p = parent;
                        }
                    }
                    None
                }
            })
            .next()
    }
}

/// Attempts to locate CODEOWNERS file based on common locations relative to
/// a given git repo
///
/// # Examples
///
/// ```rust
///  match codeowners::locate(".") {
///   Some(ownersfile)  => {
///     println!(
///      "{:#?}",
///      codeowners::from_path(ownersfile)
///    )
///  },
///   _ => println!("failed to find CODEOWNERS file")
/// }
/// ```
pub fn locate<P>(ctx: P) -> Option<PathBuf>
where
    P: AsRef<Path>,
{
    let root = ctx.as_ref().join(CODEOWNERS);
    let github = ctx.as_ref().join(".github").join(CODEOWNERS);
    let docs = ctx.as_ref().join("docs").join(CODEOWNERS);
    if root.exists() {
        Some(root)
    } else if github.exists() {
        Some(github)
    } else if docs.exists() {
        Some(docs)
    } else {
        None
    }
}

/// Parse a CODEOWNERS file existing at a given path
pub fn from_path<P>(path: P) -> Owners
where
    P: AsRef<Path>,
{
    crate::from_reader(
        path.as_ref().display().to_string(),
        File::open(path).unwrap(),
    )
}

/// Parse a CODEOWNERS file from some readable source
/// This format is defined in
/// [Github's documentation](https://help.github.com/articles/about-codeowners/)
/// The syntax is uses gitgnore
/// [patterns](https://www.kernel.org/pub/software/scm/git/docs/gitignore.html#_pattern_format)
/// followed by an identifier for an owner. More information can be found
/// [here](https://help.github.com/articles/about-codeowners/#codeowners-syntax)
pub fn from_reader<L, R>(
    location: L,
    read: R,
) -> Owners
where
    L: AsRef<str>,
    R: Read,
{
    let mut paths = BufReader::new(read)
        .lines()
        .filter_map(Result::ok)
        .enumerate()
        .filter(|(_, line)| !line.is_empty() && !line.starts_with('#'))
        .fold(Vec::new(), |mut paths, (idx, line)| {
            let mut elements = line.split_whitespace();
            if let Some(path) = elements.next() {
                let owners = elements.fold(Vec::new(), |mut result, owner| {
                    if let Ok(owner) = owner.parse() {
                        result.push(owner)
                    }
                    result
                });
                let line_number = idx + 1;
                paths.push((line_number, pattern(path), owners))
            }
            paths
        });
    // last match takes precedence
    paths.reverse();
    let location = location.as_ref().into();
    Owners { location, paths }
}

fn pattern(path: &str) -> Pattern {
    // if pattern starts with anchor or explicit wild card, it should
    // match any prefix
    let prefixed = if path.starts_with('*') || path.starts_with('/') {
        path.to_owned()
    } else {
        format!("**/{}", path)
    };
    // if pattern starts with anchor it should only match paths
    // relative to root
    let mut normalized = prefixed.trim_start_matches('/').to_string();
    // if pattern ends with /, it should match children of that directory
    if normalized.ends_with('/') {
        normalized.push_str("**");
    }
    Pattern::new(&normalized).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    const EXAMPLE: &str = include_str!("../tests/data/CODEOWNERS");

    #[test]
    fn owner_parses() {
        assert!("@user".parse() == Ok(Owner::Username("@user".into())));
        assert!("@org/team".parse() == Ok(Owner::Team("@org/team".into())));
        assert!("user@domain.com".parse() == Ok(Owner::Email("user@domain.com".into())));
        assert!("bogus".parse::<Owner>() == Err("not an owner".into()));
    }

    #[test]
    fn owner_displays() {
        assert!(Owner::Username("@user".into()).to_string() == "@user");
        assert!(Owner::Team("@org/team".into()).to_string() == "@org/team");
        assert!(Owner::Email("user@domain.com".into()).to_string() == "user@domain.com");
    }

    #[test]
    fn from_reader_parses() {
        let owners = from_reader("-", EXAMPLE.as_bytes());
        assert_eq!(
            owners,
            Owners {
                location: "-".into(),
                paths: vec![
                    (
                        37,
                        Pattern::new("docs/**").unwrap(),
                        vec![Owner::Username("@doctocat".into())]
                    ),
                    (
                        33,
                        Pattern::new("**/apps/**").unwrap(),
                        vec![Owner::Username("@octocat".into())]
                    ),
                    (
                        29,
                        Pattern::new("**/docs/*").unwrap(),
                        vec![Owner::Email("docs@example.com".into())]
                    ),
                    (
                        24,
                        Pattern::new("build/logs/**").unwrap(),
                        vec![Owner::Username("@doctocat".into())]
                    ),
                    (
                        19,
                        Pattern::new("*.go").unwrap(),
                        vec![Owner::Email("docs@example.com".into())]
                    ),
                    (
                        14,
                        Pattern::new("*.js").unwrap(),
                        vec![Owner::Username("@js-owner".into())]
                    ),
                    (
                        8,
                        Pattern::new("*").unwrap(),
                        vec![
                            Owner::Username("@global-owner1".into()),
                            Owner::Username("@global-owner2".into()),
                        ]
                    ),
                ],
            }
        )
    }

    #[test]
    fn owners_owns_wildcard() {
        let owners = from_reader("-", EXAMPLE.as_bytes());
        assert_eq!(
            owners.of("foo.txt"),
            Some(&vec![
                Owner::Username("@global-owner1".into()),
                Owner::Username("@global-owner2".into()),
            ])
        );
        assert_eq!(
            owners.of("foo/bar.txt"),
            Some(&vec![
                Owner::Username("@global-owner1".into()),
                Owner::Username("@global-owner2".into()),
            ])
        )
    }

    #[test]
    fn owners_owns_js_extention() {
        let owners = from_reader("-", EXAMPLE.as_bytes());
        assert_eq!(
            owners.of("foo.js"),
            Some(&vec![Owner::Username("@js-owner".into())])
        );
        assert_eq!(
            owners.of("foo/bar.js"),
            Some(&vec![Owner::Username("@js-owner".into())])
        )
    }

    #[test]
    fn owners_owns_go_extention() {
        let owners = from_reader("-", EXAMPLE.as_bytes());
        assert_eq!(
            owners.of("foo.go"),
            Some(&vec![Owner::Email("docs@example.com".into())])
        );
        assert_eq!(
            owners.of("foo/bar.go"),
            Some(&vec![Owner::Email("docs@example.com".into())])
        )
    }

    #[test]
    fn owners_owns_anchored_build_logs() {
        let owners = from_reader("-", EXAMPLE.as_bytes());
        // relative to root
        assert_eq!(
            owners.of("build/logs/foo.go"),
            Some(&vec![Owner::Username("@doctocat".into())])
        );
        assert_eq!(
            owners.of("build/logs/foo/bar.go"),
            Some(&vec![Owner::Username("@doctocat".into())])
        );
        // not relative to root
        assert_eq!(
            owners.of("foo/build/logs/foo.go"),
            Some(&vec![Owner::Email("docs@example.com".into())])
        )
    }

    #[test]
    fn owners_owns_unanchored_docs() {
        let owners = from_reader("-", EXAMPLE.as_bytes());
        // docs anywhere
        assert_eq!(
            owners.of("foo/docs/foo.js"),
            Some(&vec![Owner::Email("docs@example.com".into())])
        );
        assert_eq!(
            owners.of("foo/bar/docs/foo.js"),
            Some(&vec![Owner::Email("docs@example.com".into())])
        );
        // but not nested
        assert_eq!(
            owners.of("foo/bar/docs/foo/foo.js"),
            Some(&vec![Owner::Username("@js-owner".into())])
        )
    }

    #[test]
    fn owners_owns_unanchored_apps() {
        let owners = from_reader("-", EXAMPLE.as_bytes());
        assert_eq!(
            owners.of("foo/apps/foo.js"),
            Some(&vec![Owner::Username("@octocat".into())])
        )
    }

    #[test]
    fn owners_owns_anchored_docs() {
        let owners = from_reader("-", EXAMPLE.as_bytes());
        // relative to root
        assert_eq!(
            owners.of("docs/foo.js"),
            Some(&vec![Owner::Username("@doctocat".into())])
        )
    }

    #[test]
    fn implied_children_owners() {
        let owners = from_reader("-", "foo/bar @doug".as_bytes());
        assert_eq!(
            owners.of("foo/bar/baz.rs"),
            Some(&vec![Owner::Username("@doug".into())])
        )
    }
}
