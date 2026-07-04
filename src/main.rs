use notify::{Event, RecursiveMode, Result, Watcher, PollWatcher};
use std::{fs::OpenOptions, path::Path, sync::mpsc};
use std::io::Write;

mod config;
mod convert;
mod email;

extern crate dirs;

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let (tx, rx) = mpsc::channel::<Result<Event>>();

    log::info!("kiyomi - .cbz file watcher for kindle");

    if let Some(path) = config::config_file_path() {
        log::info!("config file: {:?}", path);
    }
    if let Some(path) = config::cache_file_path() {
        log::info!("cache file: {:?}", path);
    }

    let kiyomi_config = match config::get_config() {
        Ok(c) => c,
        Err(e) => {
            log::error!("config error: {}", e);
            return Ok(());
        }
    };

    let delete_automatically = kiyomi_config.options.delete;

    let watcher_config = notify::Config::default()
        .with_poll_interval(std::time::Duration::from_secs(1));

    let mut watcher = PollWatcher::new(tx, watcher_config)?;

    let watch_dir = kiyomi_config.directories.manga.clone();

    watcher.watch(Path::new(&watch_dir), RecursiveMode::Recursive)?;

    log::info!("watching for new manga .cbz files in {:?}", watch_dir);

    for res in rx {
        match res {
            Ok(event) => {
                if let notify::event::EventKind::Create(_) = event.kind {
                    process_new_manga(event.paths, delete_automatically);
                }
            }
            Err(e) => log::error!("watch error: {:?}", e),
        }
    }

    Ok(())
}

/// Files were created! Let's check if they're .cbz files. If they are, we'll process them.
fn process_new_manga(paths: Vec<std::path::PathBuf>, delete_automatically: bool) {
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
                log::error!("couldn't get filename for {:?}", path);
                continue;
            }
        };

        let _ = log_filename(filename);

        log::info!("found new file: {:?}", filename);
        if let Err(e) = manga(filename) {
            log::error!("manga error: {:?}", e);
            return;
        }

        // delete if desired
        if delete_automatically {
            match std::fs::remove_file(&path) {
                Ok(_) => log::info!("deleted file: {:?}", filename),
                Err(e) => log::error!("couldn't delete file: {:?}", e),
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
                    log::error!("couldn't get directory name for {:?}", path);
                    return Ok(());
                }
            };
            dir_name.to_str().unwrap_or("Unknown manga")
        }
        None => {
            log::error!("couldn't get parent directory for {:?}", path);
            return Ok(());
        }
    };

    log::info!("reading cbz file: {:?}", file_path);

    wait_until_stable_size(Path::new(file_path), 30)?;

    let kiyomi_config = match config::get_config() {
        Ok(c) => c,
        Err(e) => {
            log::error!("config error: {}", e);
            return Ok(());
        }
    };

    let output_path = format!("{}/kiyomi_output", kiyomi_config.directories.manga);

    // if exists remove
    if std::fs::metadata(&output_path).is_ok() {
        if let Err(e) = std::fs::remove_dir_all(&output_path) {
            log::warn!("couldn't remove output directory: {:?}", e);
        }
    }
    if let Err(e) = std::fs::create_dir_all(&output_path) {
        log::warn!("couldn't create output directory: {:?}", e);
    }

    let manga = convert::extract_images_from_cbz(file_path)?;

    // kiyomi sends email, which has a size limit. We need to stay below the limit by splitting
    // the manga and building multiple epubs
    let cover_image = manga.0.first();

    let size_limit = kiyomi_config.options.size_limit_bytes();
    log::info!("using size limit of {}MB", kiyomi_config.options.size_limit_mb);

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
        log::info!("manga will be split into {} parts due to size constraints", files.len());
    }

    for (i, file) in files.iter().enumerate() {
        log::info!("processing part {} of {}", i + 1, files.len());
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
                    kiyomi_config.smtp.port,
                    &kiyomi_config.smtp.server,
                    &kiyomi_config.smtp.username,
                    &kiyomi_config.smtp.password,
                    &kiyomi_config.smtp.from_email,
                    &kiyomi_config.smtp.to_email,
                    &format!("{}-{} {}", i, files.len(), kiyomi_config.smtp.subject),
                    &path,
                ) {
                    Ok(_) => log::info!("email sent successfully!"),
                    Err(e) => log::error!("email error: {:?}", e),
                }
            }
            Err(e) => log::error!("epub error: {:?}", e),
        }
    }

    Ok(())
}

/// We keep a log of the filenames we've already processed
fn log_filename(filename: &str) -> Option<()> {
    let path = config::cache_file_path()?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok()?;
    }

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
        log::error!("couldn't write to cache file: {}", e);
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
