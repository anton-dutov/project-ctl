mod cargo;
mod git;

use std::fs::File;

use anyhow::{Result as Fallible, format_err};

use walkdir::WalkDir;

use clap::Parser as Clap;
use serde::Deserialize;

static CFG_FILE: &str = "project-ctl.yaml";

#[derive(Debug, Deserialize)]
struct ProjectPath {
    pub path: String,
}

#[derive(Debug, Deserialize)]
struct Target {
    path: String,
    name: String,
}

/// Project control helper
#[derive(Clap)]
#[clap(name = "Project control", version = env!("CARGO_PKG_VERSION"), author = "Anton Dutov <anton.dutov@gmail.com>")]
struct Opts {
    #[clap(subcommand)]
    cmd: Command,
}

#[derive(Clap)]
enum Command {
    #[clap(name = "cargo")]
    Cargo {
        #[clap(short = 'D', long)]
        debug: bool,

        #[clap(subcommand)]
        cmd: cargo::Command,
    },

    #[clap(name = "git")]
    Git {
        #[clap(short = 'D', long)]
        debug: bool,

        #[clap(subcommand)]
        cmd: git::Command,
    },
}

/// Show git state of all repos
#[derive(Clap)]
struct GitState {
    /// Show all checked repos
    #[clap(short, long)]
    all: bool,
    /// print debug information verbosely
    #[clap(short = 'D', long)]
    debug: bool,
}

fn main() {
    if let Err(e) = run() {
        println!("{:?}", e);
    }
}

fn run() -> Fallible<()> {
    let mut cfg = dirs::config_dir().ok_or_else(|| format_err!("CONFIG DIR NOT FOUND"))?;

    cfg.push(CFG_FILE);

    let config = File::open(cfg)?;

    let paths: Vec<ProjectPath> = serde_yaml::from_reader(config)?;

    let repos: Vec<String> = paths.into_iter().map(|i| i.path).collect();

    if repos.is_empty() {
        return Err(format_err!("{}: Path not defined", CFG_FILE));
    }

    let opts: Opts = Opts::parse();

    match opts.cmd {
        Command::Cargo {
            debug,
            cmd: cargo::Command::Audit,
        } => {
            if let Err(e) = cargo::cmd(&repos, Some("audit"), debug) {
                println!("FAIL: {}", e);
            }
        }
        Command::Cargo {
            debug,
            cmd: cargo::Command::Clean,
        } => {
            if let Err(e) = cargo::cmd(&repos, Some("clean"), debug) {
                println!("FAIL: {}", e);
            }
        }
        Command::Cargo {
            debug,
            cmd: cargo::Command::Update,
        } => {
            if let Err(e) = cargo::cmd(&repos, Some("update"), debug) {
                println!("FAIL: {}", e);
            }
        }
        Command::Cargo {
            debug,
            cmd: cargo::Command::Info,
        } => {
            if let Err(e) = cargo::cmd(&repos, None, debug) {
                println!("FAIL: {}", e);
            }
        }
        Command::Cargo {
            debug,
            cmd: cargo::Command::BuildWin(params),
        } => {
            if let Err(e) = cargo::build_windows(debug, params) {
                println!("FAIL: {}", e);
            }
        }
        Command::Git {
            debug,
            cmd: git::Command::State(o),
        } => {
            if let Err(e) = git::cmd_state(&repos, o.all, debug) {
                println!("FAIL: {}", e);
            }
        }
    }

    Ok(())
}
