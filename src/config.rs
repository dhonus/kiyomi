extern crate dirs;

use std::error::Error;
use std::path::Path;

pub struct Settings {
    pub smtp_server: String,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_from: String,
    pub smtp_to: String,
    pub smtp_subject: String,
    pub smtp_port: Option<i64>,

    pub manga_dir: String,

    pub delete: bool,
    // let user choose size to slip over
    // 25MB is the default size for email attachments
    pub size_limit_mb: usize,
}

impl Settings {
    pub fn from_env() -> Option<Self> {
        let smtp_server = std::env::var("KIYOMI_SMTP_SERVER").ok()?;
        let smtp_username = std::env::var("KIYOMI_SMTP_USERNAME").ok()?;
        let smtp_password = std::env::var("KIYOMI_SMTP_PASSWORD").ok()?;
        let smtp_from = std::env::var("KIYOMI_SMTP_FROM").ok()?;
        let smtp_to = std::env::var("KIYOMI_SMTP_TO").ok()?;

        let manga_dir = std::env::var("KIYOMI_MANGA_DIR").ok()?;

        let smtp_subject =
            std::env::var("KIYOMI_SMTP_SUBJECT").unwrap_or_else(|_| "Manga".to_string());

        let smtp_port = std::env::var("KIYOMI_SMTP_PORT").ok().and_then(|v| v.parse().ok());

        let delete = std::env::var("KIYOMI_DELETE").map(|v| v == "true").unwrap_or(false);

        let size_limit_mb =
            std::env::var("KIYOMI_SIZE_LIMIT").ok().and_then(|v| v.parse().ok()).unwrap_or(25);

        Some(Self {
            smtp_server,
            smtp_username,
            smtp_password,
            smtp_from,
            smtp_to,
            smtp_subject,
            smtp_port,
            manga_dir,
            delete,
            size_limit_mb,
        })
    }

    pub fn from_config_file(path: &Path) -> Result<Self, Box<dyn Error>> {
        let config_str = std::fs::read_to_string(path)?;
        let config: toml::Value = toml::from_str(&config_str)?;

        let get_str = |section: &str, key: &str| -> Result<String, Box<dyn Error>> {
            config
                .get(section)
                .and_then(|s| s.get(key))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| format!("missing {}.{}", section, key).into())
        };

        let get_int_optional = |section: &str, key: &str| -> Option<i64> {
            config.get(section).and_then(|s| s.get(key)).and_then(|v| v.as_integer())
        };

        let get_bool_optional = |section: &str, key: &str| -> Option<bool> {
            config.get(section).and_then(|s| s.get(key)).and_then(|v| v.as_bool())
        };

        Ok(Self {
            smtp_server: get_str("smtp", "server")?,
            smtp_username: get_str("smtp", "username")?,
            smtp_password: get_str("smtp", "password")?,
            smtp_from: get_str("smtp", "from_email")?,
            smtp_to: get_str("smtp", "to_email")?,
            smtp_subject: get_str("smtp", "subject")?,

            smtp_port: get_int_optional("smtp", "port"),

            manga_dir: get_str("directories", "manga")?,

            delete: get_bool_optional("options", "delete").unwrap_or(false),

            size_limit_mb: get_int_optional("options", "size_limit").unwrap_or(25) as usize,
        })
    }

    pub fn load() -> Result<Self, Box<dyn Error>> {
        if let Some(settings) = Self::from_env() {
            println!("- using environment configuration");
            return Ok(settings);
        }

        let config_path = crate::paths::config_file()?;

        if config_path.exists() {
            println!("- using config file {:?}", config_path);
            return Self::from_config_file(&config_path);
        }

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let default_config = r#"[smtp]
server = "smtp.example.com"
username = ""
password = ""
from_email = ""
to_email = ""
subject = "Manga"

[directories]
# manga = "/path/to/manga"

[options]
delete = false
"#;

        std::fs::write(&config_path, default_config)?;

        println!("- created default config at {:?}", config_path);

        Err(format!("please edit config at {:?}", config_path).into())
    }
}
