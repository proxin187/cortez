mod processor;
mod memory;
mod bus;
mod loader;
mod tui;

use tui::Tui;

use clap::{Parser, Subcommand};

use std::fs;


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,

    #[arg(long, short, action)]
    debug: bool,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// a interactive emulator interface
    Interactive {
        path: String
    },
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    match args.command {
        Command::Interactive { path } => {
            let rom = fs::read(path)?;

            let mut tui = Tui::new()?;

            tui.flash(&rom)?;

            tui.run()?;
        },
    }

    Ok(())
}


