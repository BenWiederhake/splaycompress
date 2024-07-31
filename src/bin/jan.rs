use splaycompress::{compress8, decompress8};
use std::io::{stdin, stdout};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Whether to decompress instead of compress.
    #[arg(short, long)]
    decompress: bool,
}

fn main() {
    let r = stdin().lock();
    let w = stdout().lock();
    let args = Args::parse();
    if args.decompress {
        decompress8(r, w).unwrap();
    } else {
        compress8(r, w).unwrap();
    }
}
