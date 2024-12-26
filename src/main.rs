use notify::{Event, RecursiveMode, Result, Watcher, PollWatcher};
use std::{fs::OpenOptions, path::Path, sync::mpsc};
use std::io::Write;

mod config;
mod convert;
mod email;

extern crate dirs;

fn main() -> Result<()> {
    let (tx, rx) = mpsc::channel::<Result<Event>>();

    println!("kiyomi - .cbz file watcher for kindle");
    if let Some(dir) = dirs::cache_dir() {
        println!("- your cache file is located here: {:?}", dir.join("kiyomi/cache"));
    }

    let kiyomi_config = match config::get_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("! config error: {:?}", e);
            return Ok(());
        }
    };

    match config::validate_config(&kiyomi_config) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("! config error: {:?}", e);
            return Ok(());
        }
    }

    let watcher_config = notify::Config::default()
        .with_poll_interval(std::time::Duration::from_secs(1))
        .with_compare_contents(true);

    let mut watcher = PollWatcher::new(tx, watcher_config)?;

    watcher.watch(Path::new(
        kiyomi_config
            .get("directories")
            .and_then(|d| d.get("manga"))
            .and_then(|m| m.as_str())
            .unwrap(),
    ), RecursiveMode::Recursive)?;

    println!("{}\n", format!("- watching for changes in {:?}", std::env::current_dir()?));

    for res in rx {
        match res {
            Ok(event) => {
                match event.kind {
                    notify::event::EventKind::Create(_) => {
                        created_files(event.paths);
                    }
                    _ => (),
                }
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    }

    
    Ok(())
}

/// Files were created! Let's check if they're .cbz files. If they are, we'll process them.
fn created_files(paths: Vec<std::path::PathBuf>) -> () {
    for path in paths {
        println!("+ created {:?}", path);
        if !path.is_file() {
            continue;
        }
        if path.extension().unwrap_or_default() != "cbz" {
            continue;
        }
        let filename = match path.to_str() {
            Some(f) => f,
            None => {
                continue;
            }
        };

        if let Some(_) = log_filename(filename) {
            println!("- created {:?}", filename);
            if let Err(e) = manga(filename) {
                eprintln!("manga error: {:?}", e);
            }
        }
    }
}

/// We found a cbz manga. Let's deal with it.
fn manga(file_path: &str) -> Result<()> {
    let path = Path::new(file_path); // .cbz absolute path
    
    // some sources don't provide nicely tagged files. In this case, we at least want the manga title
    let fallback_title = match path.parent() {
        Some(p) => {
            let dir_name = match p.file_name() {
                Some(name) => name,
                None => {
                    eprintln!("! couldn't get directory name");
                    return Ok(());
                }
            };
            dir_name.to_str().unwrap_or("Unknown manga")
        }
        None => {
            eprintln!("! couldn't get parent directory");
            return Ok(());
        }
    };

    println!("+ reading cbz file: {:?}", file_path);
    std::thread::sleep(std::time::Duration::from_secs(1)); // await fs writes

    let kiyomi_config = match config::get_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("! config error: {:?}", e);
            return Ok(());
        }
    };

    let output = kiyomi_config
        .get("directories")
        .and_then(|d| d.get("manga"))
        .and_then(|o| o.as_str())
        .expect("??? missing directories.output");

    let output_path = format!("{}/kiyomi_output", output);

    // if exists remove
    if std::fs::metadata(&output_path).is_ok() {
        match std::fs::remove_dir_all(&output_path) {
            Ok(_) => {
            }
            Err(e) => eprintln!("??? couldn't remove output directory: {:?}", e),
        }
    }
    match std::fs::create_dir_all(&output_path) {
        Ok(_) => (),
        Err(e) => eprintln!("??? couldn't create output directory: {:?}", e),
    }

    let manga = convert::extract_images_from_cbz(file_path)?;
    match convert::build_epub_from_images(manga, fallback_title, &output_path) {
        Ok(_) => {
            match email::send_epubs_via_email(
                kiyomi_config["smtp"]["server"].as_str().unwrap(),
                kiyomi_config["smtp"]["username"].as_str().unwrap(),
                kiyomi_config["smtp"]["password"].as_str().unwrap(),
                kiyomi_config["smtp"]["from_email"].as_str().unwrap(),
                kiyomi_config["smtp"]["to_email"].as_str().unwrap(),
                kiyomi_config["smtp"]["subject"].as_str().unwrap(),
                &output_path,
            ) {
                Ok(_) => println!("+ email sent successfully!"),
                Err(e) => eprintln!("email error: {:?}", e),
            }
        },
        Err(e) => eprintln!("epub error: {:?}", e),
    }
    Ok(())
}

/// We keep a log of the filenames we've already processed
fn log_filename(filename: &str) -> Option<()> {
    let dir = dirs::cache_dir()?;
    let mut path = dir.join("kiyomi");

    std::fs::create_dir_all(&path).ok()?;

    path.push("cache");

    // if this file already exists, we check if the filename is already in the file
    if path.exists() {
        let file = std::fs::read_to_string(&path).ok()?;
        if file.contains(filename) {
            return None;
        }
    }

    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(&path).ok()?;

    if let Err(e) = writeln!(file, "{}", filename) {
        eprintln!("! couldn't write to file: {}", e);
    }

    Some(())
}
