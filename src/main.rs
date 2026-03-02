use crate::config::Settings;
use notify::{Event, PollWatcher, RecursiveMode, Result, Watcher};
use std::io::Write;
use std::{path::Path, sync::mpsc};

mod config;
mod convert;
mod email;
mod paths;

extern crate dirs;

fn main() -> Result<()> {
    let (tx, rx) = mpsc::channel::<Result<Event>>();

    println!("kiyomi - .cbz file watcher for kindle");

    match paths::config_file() {
        Ok(p) => println!("- config file: {:?}", p),
        Err(_) => println!("- config file: unknown"),
    }

    match paths::cache_file() {
        Ok(p) => println!("- log file: {:?}", p),
        Err(_) => println!("- log file: unknown"),
    }

    let settings = Settings::load().expect("Failed to load settings");

    let watcher_config =
        notify::Config::default().with_poll_interval(std::time::Duration::from_secs(1));

    let mut watcher = PollWatcher::new(tx, watcher_config)?;

    println!("Attempting to watch directory: {}", settings.manga_dir);

    watcher
        .watch(Path::new(&settings.manga_dir), RecursiveMode::Recursive)
        .expect("Failed to watch manga directory");

    println!("\n{}\n", format!("Watching for new manga.cbz in {:?}", &settings.manga_dir));

    for res in rx {
        match res {
            Ok(event) => match event.kind {
                notify::event::EventKind::Create(_) => {
                    process_new_manga(event.paths, &settings);
                }
                _ => (),
            },
            Err(e) => println!("watch error: {:?}", e),
        }
    }

    Ok(())
}

/// Files were created! Let's check if they're .cbz files. If they are, we'll process them.
fn process_new_manga(paths: Vec<std::path::PathBuf>, settings: &Settings) -> () {
    for path in paths {
        if !path.is_file() {
            continue;
        }
        if path.extension().unwrap_or_default() != "cbz" {
            continue;
        }
        let filename = match path.to_str() {
            Some(f) => f,
            None => {
                eprintln!("! couldn't get filename");
                continue;
            }
        };

        let _ = log_filename(filename);

        println!("+ found new file: {:?}", filename);
        if let Err(e) = manga(filename, &settings) {
            eprintln!("manga error: {:?}", e);
            return;
        }

        // delete if desired
        if settings.delete {
            match std::fs::remove_file(&path) {
                Ok(_) => println!("- deleted file: {:?}", filename),
                Err(e) => eprintln!("! couldn't delete file: {:?}", e),
            }
        }

        println!();
    }
}

/// We found a cbz manga. Let's deal with it.
fn manga(file_path: &str, settings: &Settings) -> Result<()> {
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

    wait_until_stable_size(Path::new(file_path), 30)?;

    let output_path = format!("{}/kiyomi_output", settings.manga_dir);

    // if exists remove
    if std::fs::metadata(&output_path).is_ok() {
        match std::fs::remove_dir_all(&output_path) {
            Ok(_) => {}
            Err(e) => eprintln!("??? couldn't remove output directory: {:?}", e),
        }
    }
    match std::fs::create_dir_all(&output_path) {
        Ok(_) => (),
        Err(e) => eprintln!("??? couldn't create output directory: {:?}", e),
    }

    let manga = convert::extract_images_from_cbz(file_path)?;

    // kiyomi sends email, which has a size limit. We need to stay below 20MB by splitting the manga
    // and bulding multiple epubs

    let cover_image = manga.0.first();

    let mut current_size = 0;
    let mut files = Vec::new();
    let mut current_epub = Vec::new();
    for image in manga.0.iter() {
        current_size += image.contents.len() / (1024 * 1024);
        if current_size > settings.size_limit_mb {
            files.push(current_epub);
            current_epub = Vec::new();
            current_size = image.contents.len();
        }
        current_epub.push(image);
    }
    if !current_epub.is_empty() {
        files.push(current_epub);
    }
    if files.len() > 1 {
        println!("- manga will be split into {} parts due to size constraints", files.len());
    }

    for (i, file) in files.iter().enumerate() {
        println!("- processing part {} of {}", i + 1, files.len());
        // now we have a vector of epubs. Let's build them
        match convert::build_epub_from_images(
            (file, manga.1.clone()),
            cover_image,
            &format!("{} - part {}", fallback_title, i + 1),
            &output_path,
            if files.len() > 1 { Some((i, files.len())) } else { None },
        ) {
            Ok(path) => {
                match email::send_epub(
                    &settings.smtp_port,
                    &settings.smtp_server,
                    &settings.smtp_username,
                    &settings.smtp_password,
                    &settings.smtp_from,
                    &settings.smtp_to,
                    &format!("{} - part {} of {}", i + 1, files.len(), &settings.smtp_subject),
                    &path,
                ) {
                    Ok(_) => println!("- email sent successfully!"),
                    Err(e) => eprintln!("email error: {:?}", e),
                }
            }
            Err(e) => eprintln!("epub error: {:?}", e),
        }
    }

    Ok(())
}

/// We keep a log of the filenames we've already processed
fn log_filename(filename: &str) -> Option<()> {
    let path = paths::cache_file().ok()?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok()?;
    }

    if path.exists() {
        let file = std::fs::read_to_string(&path).ok()?;
        if file.contains(filename) {
            return None;
        }
    }

    let mut file =
        std::fs::OpenOptions::new().write(true).append(true).create(true).open(&path).ok()?;

    writeln!(file, "{}", filename).ok()?;

    Some(())
}

fn wait_until_stable_size(path: &Path, timeout_secs: u64) -> std::io::Result<()> {
    use std::{fs, thread, time::Duration};

    let mut last_size = 0;
    let mut stable_count = 0;

    for _ in 0..timeout_secs {
        let metadata = fs::metadata(path)?;
        let size = metadata.len();

        if size == last_size {
            stable_count += 1;
        } else {
            stable_count = 0;
            last_size = size;
        }

        if stable_count >= 2 {
            return Ok(());
        }

        thread::sleep(Duration::from_secs(1));
    }

    Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "Couldn't stabilize"))
}
