// run  := cargo run --
// dir  := .
// kid  :=

mod load_config;
mod plain_text;
mod system_stats;
mod utils;

use clap::{Arg, ArgAction, Command};
use load_config::LoadConfig;
use std::path::PathBuf;
use system_stats::SystemStats;

fn main() {
    let matches = Command::new("System Info")
        .arg(
            Arg::new("text")
                .short('t')
                .long("text")
                .help("Display system information as plain text")
                .required(false)
                .action(ArgAction::SetFalse),
        )
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .num_args(1)
                .required(false),
        )
        .get_matches();

    let config_path = matches.get_one::<String>("config").map(PathBuf::from);
    let plain_text_flag = matches.get_flag("text");

    let config_loader = LoadConfig::new(config_path);
    let config = config_loader.get_config();
    let system_stats = SystemStats::new(config);
    if plain_text_flag {
        let s = plain_text::generate_text(config, &system_stats);
        print!("{}", s);
    }
}

