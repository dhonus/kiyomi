use serde::Deserialize;
use std::path::PathBuf;

extern crate dirs;

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub struct Config {
    pub smtp: SmtpConfig,
    pub directories: DirectoriesConfig,
    pub options: OptionsConfig,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub struct SmtpConfig {
    pub server: String,
    pub port: Option<u16>,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub to_email: String,
    pub subject: String,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub struct DirectoriesConfig {
    pub manga: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct OptionsConfig {
    pub delete: bool,
    #[serde(alias = "size_limit")]
    pub size_limit_mb: u64,
}

impl Default for OptionsConfig {
    fn default() -> Self {
        Self {
            delete: false,
            size_limit_mb: 25,
        }
    }
}

impl OptionsConfig {
    pub fn size_limit_bytes(&self) -> usize {
        self.size_limit_mb as usize * 1024 * 1024
    }
}

/// Where the (optional) TOML config file lives.
/// Overridable with KIYOMI_CONFIG_PATH so a whole file mount isn't required in Docker.
pub fn config_file_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("KIYOMI_CONFIG_PATH") {
        return Some(PathBuf::from(path));
    }
    dirs::config_dir().map(|dir| dir.join("kiyomi.toml"))
}

/// Where we keep track of filenames we've already processed.
/// Overridable with KIYOMI_CACHE_PATH.
pub fn cache_file_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("KIYOMI_CACHE_PATH") {
        return Some(PathBuf::from(path));
    }
    dirs::cache_dir().map(|dir| dir.join("kiyomi/cache"))
}

const DEFAULT_CONFIG_TEMPLATE: &str = r#"[smtp]
server = "smtp.example.com"
username = ""
password = ""
from_email = ""
to_email = ""
subject = "Manga"
# port = 587 # optional, defaults to 587 for TLS

[directories]
# manga = "/path/to/your/manga"

[options]
delete = false
# size_limit_mb = 25
"#;

/// Loads config from (optionally) a TOML file, then applies environment variable
/// overrides on top. Every setting can be supplied purely via env vars, which is
/// the intended path when running in Docker - no config file is required in that case.
pub fn get_config() -> Result<Config, Box<dyn std::error::Error>> {
    let config_path = config_file_path();
    let file_existed = config_path.as_deref().is_some_and(|p| p.exists());

    let mut config = match (&config_path, file_existed) {
        (Some(path), true) => {
            let config_str = std::fs::read_to_string(path)?;
            toml::from_str(&config_str)?
        }
        _ => Config::default(),
    };

    apply_env_overrides(&mut config);

    if let Err(e) = validate_config(&config) {
        // Only bother scaffolding a config file if env vars didn't already cover
        // everything - in a pure env-var (e.g. Docker) setup there's nothing to fix here.
        if let Some(path) = &config_path {
            if !file_existed {
                match std::fs::write(path, DEFAULT_CONFIG_TEMPLATE) {
                    Ok(_) => log::info!("created default config file at {:?}", path),
                    Err(write_err) => log::warn!("couldn't create config file at {:?}: {}", path, write_err),
                }
            }
        }

        let hint = match &config_path {
            Some(path) => format!(
                "{e} (edit the config file at {:?}, or set the corresponding KIYOMI_* environment variable)",
                path
            ),
            None => format!("{e} (set the corresponding KIYOMI_* environment variable)"),
        };
        return Err(hint.into());
    }

    Ok(config)
}

fn apply_env_overrides(config: &mut Config) {
    if let Ok(v) = std::env::var("KIYOMI_SMTP_SERVER") {
        config.smtp.server = v;
    }
    if let Ok(v) = std::env::var("KIYOMI_SMTP_PORT") {
        match v.parse() {
            Ok(port) => config.smtp.port = Some(port),
            Err(_) => log::warn!("KIYOMI_SMTP_PORT is not a valid port number: {:?}", v),
        }
    }
    if let Ok(v) = std::env::var("KIYOMI_SMTP_USERNAME") {
        config.smtp.username = v;
    }
    if let Ok(v) = std::env::var("KIYOMI_SMTP_PASSWORD") {
        config.smtp.password = v;
    }
    if let Ok(v) = std::env::var("KIYOMI_SMTP_FROM_EMAIL") {
        config.smtp.from_email = v;
    }
    if let Ok(v) = std::env::var("KIYOMI_SMTP_TO_EMAIL") {
        config.smtp.to_email = v;
    }
    if let Ok(v) = std::env::var("KIYOMI_SMTP_SUBJECT") {
        config.smtp.subject = v;
    }
    if let Ok(v) = std::env::var("KIYOMI_MANGA_DIR") {
        config.directories.manga = v;
    }
    if let Ok(v) = std::env::var("KIYOMI_DELETE") {
        match v.parse() {
            Ok(delete) => config.options.delete = delete,
            Err(_) => log::warn!("KIYOMI_DELETE must be true or false, got {:?}", v),
        }
    }
    if let Ok(v) = std::env::var("KIYOMI_SIZE_LIMIT_MB") {
        match v.parse() {
            Ok(size) => config.options.size_limit_mb = size,
            Err(_) => log::warn!("KIYOMI_SIZE_LIMIT_MB must be a positive integer, got {:?}", v),
        }
    }
}

pub fn validate_config(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    if config.smtp.server.is_empty() {
        return Err("smtp.server must not be empty".into());
    }
    if config.smtp.username.is_empty() {
        return Err("smtp.username must not be empty".into());
    }
    if config.smtp.password.is_empty() {
        return Err("smtp.password must not be empty".into());
    }
    if config.smtp.from_email.is_empty() {
        return Err("smtp.from_email must not be empty".into());
    }
    if config.smtp.to_email.is_empty() {
        return Err("smtp.to_email must not be empty".into());
    }
    if config.smtp.subject.is_empty() {
        return Err("smtp.subject must not be empty".into());
    }
    if config.directories.manga.is_empty() {
        return Err("directories.manga must not be empty".into());
    }
    if !std::path::Path::new(&config.directories.manga).exists() {
        return Err(format!(
            "directories.manga path does not exist: {:?}",
            config.directories.manga
        )
        .into());
    }
    if config.options.size_limit_mb < 1 {
        return Err("options.size_limit_mb must be greater than 0".into());
    }

    Ok(())
}
