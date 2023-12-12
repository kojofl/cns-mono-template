mod mono;
use std::{
    error::Error,
    process::{Command, Stdio},
    sync::OnceLock,
    thread::spawn,
};

use clap::{Parser, Subcommand};
use mono::setup_mono;

static MERGE_DEPS: OnceLock<bool> = OnceLock::new();

static REPOS: [(&str, &str); 5] = [
    (
        "cns-app-runtime",
        "https://github.com/nmshd/cns-app-runtime",
    ),
    (
        "cns-consumption",
        "https://github.com/nmshd/cns-consumption",
    ),
    ("cns-content", "https://github.com/nmshd/cns-content"),
    ("cns-transport", "https://github.com/nmshd/cns-transport"),
    ("cns-runtime", "https://github.com/nmshd/cns-runtime"),
];

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Initialize the monorepo by cloning all repos
    Init {
        #[arg(short, long)]
        merge: bool,
    },
    /// Clean project
    Clean,
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init { merge } => {
            MERGE_DEPS.set(*merge).unwrap();
            if let Err(e) = initialize() {
                println!("{e}");
            }
        }
        Commands::Clean => {
            if let Err(e) = clean() {
                println!("{e}");
            }
        }
    }
}

fn initialize() -> Result<(), Box<dyn Error>> {
    let mut handles = Vec::new();
    for repo in REPOS {
        handles.push(spawn(move || {
            if let Err(e) = init_repo(repo) {
                println!("{e}");
            }
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }
    setup_mono()?;
    Ok(())
}

fn init_repo((dir, url): (&'static str, &'static str)) -> Result<(), Box<dyn Error>> {
    if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", &format!("git clone {url}")])
            .current_dir(std::fs::canonicalize("../packages")?)
            .spawn()
            .expect("No git?")
            .wait()?;
        Command::new("cmd")
            .args(["/C", "rm -rf ./.git"])
            .current_dir(std::fs::canonicalize(format!("../packages/{dir}"))?)
            .spawn()
            .expect("Failed to run delete .git")
            .wait()?;
    } else {
        Command::new("sh")
            .args(["-c", &format!("git clone {url}")])
            .current_dir(std::fs::canonicalize("../packages")?)
            .spawn()
            .expect("No git?")
            .wait()?;
        Command::new("sh")
            .args([
                "-c",
                &format!(
                    "rm -rf {}",
                    std::fs::canonicalize(format!("../packages/{}/.git", dir))?
                        .to_str()
                        .unwrap()
                ),
            ])
            .stdout(Stdio::inherit())
            .spawn()
            .expect("Failed to run delete .git")
            .wait()?;
    };
    Ok(())
}

fn clean() -> Result<(), Box<dyn Error>> {
    if cfg!(target_os = "windows") {
        let current_dir = std::fs::canonicalize("../")?;
        Command::new("cmd")
            .args(["/C", "rm -rf ./packages/*"])
            .current_dir(&current_dir)
            .stdout(Stdio::inherit())
            .spawn()
            .expect("Failed to run clean");
        Command::new("cmd")
            .args(["/C", "rm -rf node_modules"])
            .current_dir(&current_dir)
            .stdout(Stdio::inherit())
            .spawn()
            .expect("Failed to run clean");
        Command::new("cmd")
            .args(["/C", "rm -rf package.json"])
            .current_dir(&current_dir)
            .stdout(Stdio::inherit())
            .spawn()
            .expect("Failed to run clean");
        Command::new("cmd")
            .args(["/C", "rm -rf yarn.lock"])
            .current_dir(&current_dir)
            .stdout(Stdio::inherit())
            .spawn()
            .expect("Failed to run clean");
    } else {
        let current_dir = std::fs::canonicalize("../")?;
        Command::new("sh")
            .args(["-c", "rm -rf ./packages/*"])
            .current_dir(&current_dir)
            .stdout(Stdio::inherit())
            .spawn()
            .expect("Failed to run clean");
        Command::new("sh")
            .args(["-c", "rm -rf node_modules"])
            .current_dir(&current_dir)
            .stdout(Stdio::inherit())
            .spawn()
            .expect("Failed to run clean");
        Command::new("sh")
            .args(["-c", "rm -rf package.json"])
            .current_dir(&current_dir)
            .stdout(Stdio::inherit())
            .spawn()
            .expect("Failed to run clean");
        Command::new("sh")
            .args(["-c", "rm -rf yarn.lock"])
            .current_dir(&current_dir)
            .stdout(Stdio::inherit())
            .spawn()
            .expect("Failed to run clean");
    };
    Ok(())
}
