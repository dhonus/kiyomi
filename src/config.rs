extern crate dirs;

// Get the config file. If it doesn't exist, create a default one
pub fn get_config() -> Result<toml::Value, Box<dyn std::error::Error>> {

    let config_dir = match dirs::config_dir() {
        Some(dir) => dir,
        None => {
            return Err("! the config directory could not be created".into());
        }
    };

    let config_path = config_dir.join("kiyomi.toml");

    if !config_path.exists() {
        let default_config = r#"[smtp]
server = "smtp.example.com"
username = ""
password = ""
from_email = ""
to_email = ""
subject = "Manga"
[directories]
# manga = "/path/to/your/manga"
        "#;

        match std::fs::write(&config_path, default_config) {
            Ok(_) => println!("- created default config file at {:?}", config_path),
            Err(e) => eprintln!("! couldn't create config file: {}", e),
        }

        return Err(format!("! please edit the config file at {:?}", config_path).into());
    }

    let config_str = std::fs::read_to_string(&config_path)?;
    let config: toml::Value = toml::from_str(&config_str)?;

    Ok(config)
}

pub fn validate_config(config: &toml::Value) -> Result<(), Box<dyn std::error::Error>> {
    let smtp = config.get("smtp").ok_or("missing [smtp] section")?;
    let directories = config.get("directories").ok_or("missing [directories] section")?;

    let server = smtp.get("server").ok_or("missing smtp.server")?;
    let username = smtp.get("username").ok_or("missing smtp.username")?;
    let password = smtp.get("password").ok_or("missing smtp.password")?;
    let from_email = smtp.get("from_email").ok_or("missing smtp.from_email")?;
    let to_email = smtp.get("to_email").ok_or("missing smtp.to_email")?;
    let subject = smtp.get("subject").ok_or("missing smtp.subject")?;

    let manga = directories.get("manga").ok_or("missing directories.manga")?;

    if server.as_str().is_none() {
        return Err("smtp.server must be a string".into());
    }

    if username.as_str().is_none() {
        return Err("smtp.username must be a string".into());
    }

    if password.as_str().is_none() {
        return Err("smtp.password must be a string".into());
    }

    if from_email.as_str().is_none() {
        return Err("smtp.from_email must be a string".into());
    }

    if to_email.as_str().is_none() {
        return Err("smtp.to_email must be a string".into());
    }

    if subject.as_str().is_none() {
        return Err("smtp.subject must be a string".into());
    }

    if manga.as_str().is_none() {
        return Err("directories.manga must be a string".into());
    }

    if !std::path::Path::new(manga.as_str().unwrap()).exists() {
        return Err("directories.manga path does not exist".into());
    }

    // check length
    if username.as_str().unwrap().len() == 0 {
        return Err("smtp.username must not be empty".into());
    }

    if password.as_str().unwrap().len() == 0 {
        return Err("smtp.password must not be empty".into());
    }

    if from_email.as_str().unwrap().len() == 0 {
        return Err("smtp.from_email must not be empty".into());
    }

    if to_email.as_str().unwrap().len() == 0 {
        return Err("smtp.to_email must not be empty".into());
    }

    if subject.as_str().unwrap().len() == 0 {
        return Err("smtp.subject must not be empty".into());
    }

    Ok(())
}