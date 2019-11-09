# codeowners [![Build Status](https://travis-ci.org/softprops/codeowners.svg?branch=master)](https://travis-ci.org/softprops/codeowners) [![Coverage Status](https://coveralls.io/repos/github/softprops/codeowners/badge.svg?branch=master)](https://coveralls.io/github/softprops/codeowners?branch=master) [![Software License](https://img.shields.io/badge/license-MIT-brightgreen.svg)](LICENSE) [![crates.io](https://img.shields.io/crates/v/codeowners.svg)](https://crates.io/crates/codeowners) [![](https://github.com/softprops/codeowners/workflows/Main/badge.svg)](https://github.com/softprops/codeowners/actions)

> A Github [CODEOWNERS](https://help.github.com/articles/about-codeowners/) answer sheet

[Documentation](https://docs.rs/codeowners)

## installation

Add the following to your `Cargo.toml` filter

```toml
[dependencies]
codeowners = "0.1"
```

## Usage

Typical use involves resolving a CODEOWNERS file, parsing it, then querying target paths

```rust
use std::env;

fn main() {
  if let (Some(owners_file), Some(path)) =
     (env::args().nth(1), env::args().nth(2)) {
     let owners = codeowners::from_path(owners_file);
     match owners.of(&path) {
       None => println!("{} is up for adoption", path),
       Some(owners) => {
          for owner in owners {
            println!("{}", owner);
          }
       }
     }
  }
}
```

Doug Tangren (softprops) 2017-2019
