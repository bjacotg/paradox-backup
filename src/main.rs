extern crate chrono;
extern crate clap;
extern crate env_logger;
extern crate log;
extern crate notify;
extern crate regex;

use chrono::offset::Local;
use clap::{App, Arg};
use env_logger::{Builder, Target};
use log::{debug, error, warn, LevelFilter};
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use regex::Regex;
use std::fs::create_dir;
use std::io::Error;
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::Duration;

static BACKUP_FOLDER: &str = "backup";
static EU4_EXTENSION: &str = "eu4";

fn is_backup(p: &PathBuf) -> bool {
    let file_name = p.file_name().unwrap().to_str().unwrap();
    let re = Regex::new(".*_Backup.eu4").unwrap();

    return re.is_match(file_name);
}

fn create_dir_if_none(p: &Path) -> Result<(), Error> {
    if p.exists() && !p.is_dir() {
        return Err(Error::new(
            std::io::ErrorKind::InvalidInput,
            "Path already exists, but is not dir",
        ));
    }
    if p.exists() {
        return Ok(());
    }
    match create_dir(p) {
        Ok(_) => {
            debug!("Created dir {}", p.display());
            return Ok(());
        }

        Err(error) => {
            return Err(error);
        }
    }
}

fn watch(p: &Path) -> notify::Result<()> {
    // Create a channel to receive the events.
    let (tx, rx) = channel();

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(2))?;

    watcher.watch(p, RecursiveMode::NonRecursive)?;

    // This is a simple loop, but you may want to use more complex logic here,
    // for example to handle I/O.
    loop {
        match rx.recv() {
            Ok(DebouncedEvent::NoticeRemove(ref backup_path)) if is_backup(backup_path) => {
                debug!("Removed backup");
            }
            Ok(DebouncedEvent::NoticeRemove(ref backup_path)) if !is_backup(backup_path) => {
                debug!("Removed savegame");
            }
            Ok(DebouncedEvent::Rename(ref _src, ref dst)) if is_backup(dst) => {
                debug!("Backup save")
            }
            Ok(DebouncedEvent::Create(ref path)) if !is_backup(path) => {
                debug!("Backing up {:?}", path);
                save(path)?;
            }
            Ok(event) => warn!("{:?}", event),
            Err(e) => error!("watch error: {:?}", e),
        }
    }
}

fn save(save_file_path: &Path) -> Result<(), Error> {
    let save_folder = save_file_path.parent().unwrap();
    let save_file_name = save_file_path.file_stem().unwrap();
    let backup_dir = save_folder.join(BACKUP_FOLDER).join(save_file_name);

    create_dir_if_none(backup_dir.as_path())?;

    let backup_file_name = Local::now().to_rfc3339();

    let backup_path = backup_dir
        .with_file_name(backup_file_name)
        .with_extension(EU4_EXTENSION);

    debug!("Backing up {:?} to {:?}", save_file_path, backup_path);
    std::fs::copy(save_file_path, backup_path)?;

    return Ok(());
}

fn main() {
    Builder::new()
        .target(Target::Stdout)
        .filter_level(LevelFilter::Trace)
        .init();

    let matches = App::new("Save your ironman saves")
        .version("0.1")
        .author("bjacotg@gmail.com")
        .arg(
            Arg::with_name("game")
                .short("g")
                .long("game")
                .required(true)
                .takes_value(true),
        )
        .get_matches();

    let path = Path::new(match matches.value_of("game").unwrap() {
        "eu4" => "/home/bastien/.local/share/Paradox Interactive/Europa Universalis IV/save games/",
        "ck2" => panic!("DEUS VULT"),
        x => panic!("Cannot handle flag {}", x),
    });

    let backup_path = path.join(BACKUP_FOLDER);

    match create_dir_if_none(backup_path.as_path()) {
        Ok(_) => debug!("Created backup path"),
        Err(error) => panic!("Error creating backup directory: {}", error),
    }

    if let Err(e) = watch(path) {
        println!("error: {:?}", e)
    }
}
