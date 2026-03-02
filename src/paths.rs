use std::path::PathBuf;

/// Returns config file path.
///
/// Priority:
/// 1. KIYOMI_CONFIG env var
/// 2. ~/.config/kiyomi.toml
pub fn config_file() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Ok(path) = std::env::var("KIYOMI_CONFIG") {
        return Ok(PathBuf::from(path));
    }

    let dir = dirs::config_dir().ok_or("could not determine config directory")?;

    Ok(dir.join("kiyomi.toml"))
}

/// Returns cache directory path.
///
/// Priority:
/// 1. KIYOMI_CACHE env var
/// 2. ~/.cache/kiyomi
pub fn cache_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Ok(path) = std::env::var("KIYOMI_CACHE") {
        return Ok(PathBuf::from(path));
    }

    let dir = dirs::cache_dir().ok_or("could not determine cache directory")?;

    Ok(dir.join("kiyomi"))
}

/// Returns cache log file path.
pub fn cache_file() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(cache_dir()?.join("cache"))
}
