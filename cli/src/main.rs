use std::{path::PathBuf, process::exit};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(
    name = "codeowners",
    about = r#"Github CODEOWNERS query CLI

🐙 For more information on Github CODEOWNERS see https://help.github.com/en/articles/about-code-owners
"#
)]
struct Options {
    #[structopt(
        short = "c",
        long = "codeowners",
        help = "Path to code owners file. An attempt will be made to locate one if this is not provided."
    )]
    codeowners: Option<PathBuf>,
    #[structopt(help = "Path of source file or directory, in gitignore format")]
    path: String,
}

fn main() {
    let Options { codeowners, path } = StructOpt::from_args();
    if let Some(owners) = codeowners.or_else(|| codeowners::locate(".")) {
        match codeowners::from_path(owners).of(&path) {
            None => println!("{} is up for adoption", path),
            Some(owners) => {
                for owner in owners {
                    println!("{}", owner);
                }
            }
        }
    } else {
        exit(1)
    }
}
