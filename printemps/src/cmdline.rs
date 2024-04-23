//! The command line interface of the application.

use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(long)]
    pub listen_addr: String,

    #[arg(long)]
    pub listen_port: u16,

    #[arg(long)]
    pub wasm_path: PathBuf,
}
