use std::{path::PathBuf, process::exit};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "codeowners", about = "github codeowners query cli")]
struct Options {
    #[structopt(
        short = "c",
        long = "codeowners",
        help = "Path to code owners file. An attempt will be made to locate one if not provided"
    )]
    codeowners: Option<PathBuf>,
    #[structopt(help = "Path to query")]
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
