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

    if let Some(dir) = dirs::config_dir() {
        println!("- config file: {:?}", dir.join("kiyomi.toml"));
    }
    // we log all the filenames we've downloaded
    if let Some(dir) = dirs::cache_dir() {
        println!("- log file: {:?}", dir.join("kiyomi/cache"));
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

    let delete_automatically = kiyomi_config
        .get("options")
        .and_then(|o| o.get("delete"))
        .and_then(|d| d.as_bool())
        .unwrap_or(false);

    let watcher_config = notify::Config::default()
        .with_poll_interval(std::time::Duration::from_secs(1));

    let mut watcher = PollWatcher::new(tx, watcher_config)?;

    let watch_dir = kiyomi_config
        .get("directories")
        .and_then(|d| d.get("manga"))
        .and_then(|m| m.as_str())
        .unwrap_or_else(|| {
            eprintln!("! missing directories.manga in config");
            std::process::exit(1);
        });

    watcher.watch(Path::new(
        watch_dir,
    ), RecursiveMode::Recursive)?;

    println!("\n{}\n", format!("Watching for new manga.cbz in {:?}", watch_dir));

    for res in rx {
        match res {
            Ok(event) => {
                match event.kind {
                    notify::event::EventKind::Create(_) => {
                        process_new_manga(event.paths, delete_automatically);
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
fn process_new_manga(paths: Vec<std::path::PathBuf>, delete_automatically: bool) -> () {
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
        if let Err(e) = manga(filename) {
            eprintln!("manga error: {:?}", e);
            return;
        }

        // delete if desired
        if delete_automatically {
            match std::fs::remove_file(&path) {
                Ok(_) => println!("- deleted file: {:?}", filename),
                Err(e) => eprintln!("! couldn't delete file: {:?}", e),
            }
        }

        println!();
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

    wait_until_stable_size(Path::new(file_path), 30)?;

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

    // kiyomi sends email, which has a size limit. We need to stay below 20MB by splitting the manga
    // and bulding multiple epubs

    let cover_image = manga.0.first();


    // let user choose size to slip over
    // 25MB is the default size for email attachments
    let size_limit = match kiyomi_config
        .get("options")
        .and_then(|o| o.get("size_limit"))
        .and_then(|s| s.as_integer()) {
        Some(s) => {
            println!("- using size limit of {}MB", s);
            if s < 1 {
                eprintln!("! size limit must be greater than 0");
                return Ok(());
            }
            s as usize * 1024 * 1024
        }
        None => {
            25 * 1024 * 1024
        }
    };

    let mut current_size = 0;
    let mut files = Vec::new();
    let mut current_epub = Vec::new();
    for image in manga.0.iter() {
        current_size += image.contents.len();
        if current_size > size_limit {
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
                    kiyomi_config["smtp"]["server"].as_str().unwrap(),
                    kiyomi_config["smtp"]["username"].as_str().unwrap(),
                    kiyomi_config["smtp"]["password"].as_str().unwrap(),
                    kiyomi_config["smtp"]["from_email"].as_str().unwrap(),
                    kiyomi_config["smtp"]["to_email"].as_str().unwrap(),
                    &format!("{}-{} {}", i, files.len(), kiyomi_config["smtp"]["subject"].as_str().unwrap()),
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

    Err(std::io::Error::new(
        std::io::ErrorKind::TimedOut, "Couldn't stabilize",
    ))
}
