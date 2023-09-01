mod mono;
use std::{
    error::Error,
    fs,
    os::unix::process::CommandExt,
    process::{Command, Stdio},
    thread::spawn,
};

use clap::{Parser, ValueEnum};
use mono::setup_mono;

static REPOS: [(&str, &str); 9] = [
    (
        "cns-app-runtime",
        "https://github.com/nmshd/cns-app-runtime",
    ),
    ("cns-app-web", "https://github.com/nmshd/cns-app-web"),
    ("cns-connector", "https://github.com/nmshd/cns-connector"),
    (
        "cns-consumption",
        "https://github.com/nmshd/cns-consumption",
    ),
    ("cns-content", "https://github.com/nmshd/cns-content"),
    ("cns-crypto", "https://github.com/nmshd/cns-crypto"),
    ("cns-transport", "https://github.com/nmshd/cns-transport"),
    ("connector-tui", "https://github.com/nmshd/connector-tui"),
    ("cns-runtime", "https://github.com/nmshd/cns-runtime"),
];

// static DEPNAMES: [&'static str; 7] = [
//     "@nmshd/app-runtime",
//     "@nmshd/connector",
//     "@nmshd/consumption",
//     "@nmshd/content",
//     "@nmshd/crypto",
//     "@nmshd/runtime",
//     "@nmshd/transport",
// ];

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// What command to execute
    #[arg(value_enum, short, long)]
    cmd: CliCommand,
}

#[derive(Copy, Clone, PartialEq, Debug, Eq, PartialOrd, Ord, ValueEnum)]
enum CliCommand {
    /// Initialize the monorepo by cloning all repos
    Init,
    /// Install dependencies of all repos
    Install,
    /// Clean project
    Clean,
    /// Build all repos
    Build,
}

fn main() {
    let cli = Cli::parse();

    match cli.cmd {
        CliCommand::Init => {
            if let Err(e) = initialize() {
                println!("{e}");
            }
        }
        CliCommand::Install => {
            if let Err(e) = install() {
                println!("{e}");
            }
        }
        CliCommand::Clean => {
            if let Err(e) = clean() {
                println!("{e}");
            }
        }
        CliCommand::Build => {
            if let Err(e) = build() {
                println!("{e}");
            }
        }
    }
}

fn build() -> Result<(), Box<dyn Error>> {
    let dir = fs::canonicalize("../")?;
    if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", "npx nx run-many -t build:node"])
            .current_dir(dir)
            .stdout(Stdio::inherit())
            .exec();
    } else {
        Command::new("sh")
            .args(["-c", "npx nx run-many -t build:node"])
            .current_dir(dir)
            .stdout(Stdio::inherit())
            .exec();
    };
    Ok(())
}

fn install() -> Result<(), Box<dyn Error>> {
    if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", "pnpm i"])
            .current_dir(std::fs::canonicalize("../")?)
            .output()
            .expect("no npm?")
    } else {
        Command::new("sh")
            .args(["-c", "pnpm i"])
            .current_dir(std::fs::canonicalize("../")?)
            .output()
            .expect("no npm?")
    };
    Ok(())
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
    // Now that we initialized the repos we update the package.json to use the local repos for
    // building
    // let mut file = OpenOptions::new()
    //     .read(true)
    //     .write(true)
    //     .open(fs::canonicalize(format!(
    //         "../packages/{}/package.json",
    //         dir
    //     ))?)?;
    // let mut byte_positions = Vec::new();
    // let mut byte = 0;
    // let reader = BufReader::new(&file);
    // for line in reader.lines() {
    //     let Ok(line) = line else {
    //         panic!("Error reading the package.json of {}", dir);
    //     };
    //     if !line.contains("name") && DEPNAMES.iter().any(|dep| line.contains(dep)) {
    //         let indeces: Vec<_> = line.match_indices('"').collect();
    //         byte_positions.push((byte + indeces[2].0, indeces[3].0 - indeces[2].0));
    //     }
    //     byte += line.len() + 1;
    // }
    // file.rewind()?;
    // let mut buffer = Vec::with_capacity(500);
    // for (start, len) in byte_positions.into_iter().rev() {
    //     file.seek(std::io::SeekFrom::Start(start as u64 + 1 + len as u64))?;
    //     file.read_to_end(&mut buffer)?;
    //     file.rewind()?;
    //     file.seek(std::io::SeekFrom::Start(start as u64))?;
    //     write!(file, "\"workspace:*\"")?;
    //     file.write_all(&buffer)?;
    //     file.rewind()?;
    //     buffer.clear();
    // }
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
            .args(["/C", "rm -rf pnpm-lock.yaml"])
            .current_dir(&current_dir)
            .stdout(Stdio::inherit())
            .spawn()
            .expect("Failed to run clean");
        Command::new("cmd")
            .args(["/C", "rm -rf package-lock.json"])
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
            .args(["-c", "rm -rf pnpm-lock.yaml"])
            .current_dir(&current_dir)
            .stdout(Stdio::inherit())
            .spawn()
            .expect("Failed to run clean");
        Command::new("sh")
            .args(["-c", "rm -rf package-lock.json"])
            .current_dir(&current_dir)
            .stdout(Stdio::inherit())
            .spawn()
            .expect("Failed to run clean");
    };
    Ok(())
}