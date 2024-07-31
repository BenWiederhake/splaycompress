use splaycompress::{compress, decompress, Flavor};
use std::io::{stdin, stdout};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Whether to decompress instead of compress.
    #[arg(short, long)]
    decompress: bool,

    /// Flavor of the algorithm to use. Defaults to 8bit which is many times faster but slightly worse at compressing.
    #[clap(value_enum)]
    #[arg(short, long, default_value = "bit8")]
    flavor: CLIFlavor,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum CLIFlavor {
    Bit8,
    Bit16BE,
    Bit16LE,
}

fn main() {
    let r = stdin().lock();
    let w = stdout().lock();
    let args = Args::parse();
    let flavor = match args.flavor {
        CLIFlavor::Bit8 => Flavor::Symbol8,
        CLIFlavor::Bit16BE => Flavor::Symbol16BE,
        CLIFlavor::Bit16LE => Flavor::Symbol16LE,
    };
    if args.decompress {
        compress(flavor, r, w).unwrap()
    } else {
        decompress(flavor, r, w).unwrap()
    }
}
