# kiyomi

When you download manga using suwayomi, kiyomi automatically sends it to your kindle. It does this by watching your manga directory for new .cbz files, converting them to .epub and emailing them to your kindle. The generated manga is right-to-left, correctly tagged, passes epubcheck and is actually full screen on my paperwhite (no text mode margins).

## What you need
- A kindle
- An email address whitelisted for your kindle (amazon website)
- A suwayomi server / client

> Set the download location absolute path (same as kiyomi)  
> *Settings -> Download -> **Download location***

> Suwayomi must be set to download manga **as .cbz files**  
> *Settings -> Download -> **Save as CBZ archive***

## How to use

### Docker (recommended)
Copy `docker-compose.example.yml` to `docker-compose.yml`, fill in your SMTP details and manga directory, then:

```bash
docker compose up -d --build
```

Kiyomi watches the directory mounted to `/manga` in the container for new `.cbz` files, and keeps its "already sent" state in the directory mounted to `/cache`. No config file is needed - everything is set via environment variables. See [Environment variables](#environment-variables) below for the full list.

To follow the logs:
```bash
docker compose logs -f
```

### Running natively
Clone and run `cargo run --release` to start the program. Kiyomi will create a config file and print its location. Edit this file to configure. Kiyomi will watch the specified directory for new .cbz files. Any setting can alternatively be set via the environment variables below, which take precedence over the config file.

After kiyomi is running, download manga using suwayomi. Kiyomi will automagically send your manga to your kindle. Read the logs for more information. Log verbosity is controlled with `RUST_LOG` (e.g. `RUST_LOG=debug`), defaulting to `info`.

## Manga title format

1. If a split happened (too large to send in one email), the title will begin `N-M` where N is the current part and M is the total number of parts.
2. Following that, the title will be `Chapter name - Manga Title`.

## Configuration
Kiyomi will create a config file and print its location. Edit this file to configure.
### Example config
```toml
[smtp]
# SMTP server with port
server = "smtp.gmail.com"
username = "you@gmail.com"
password = "yourpassword"
from_email = "you@gmail.com"
to_email = "yourkindle_xxxxxx@kindle.com"
subject = "kiyomi"
# port = 587 # optional, defaults to 587 for TLS

[directories]
manga = "/home/you/manga"

[options]
# Set to true to delete the .cbz files after sending
delete = true
# Size in MB to split the manga into multiple emails if too large to send
# 25MB is the default if not set
size_limit_mb = 25
```

## Environment variables
Every setting can be provided as an environment variable instead of (or as an override for) the config file. This is the primary way to configure kiyomi in Docker.

| Variable                 | Config equivalent          | Default            |
|---------------------------|-----------------------------|--------------------|
| `KIYOMI_SMTP_SERVER`       | `smtp.server`                | -                   |
| `KIYOMI_SMTP_PORT`         | `smtp.port`                  | `587`               |
| `KIYOMI_SMTP_USERNAME`     | `smtp.username`              | -                   |
| `KIYOMI_SMTP_PASSWORD`     | `smtp.password`              | -                   |
| `KIYOMI_SMTP_FROM_EMAIL`   | `smtp.from_email`            | -                   |
| `KIYOMI_SMTP_TO_EMAIL`     | `smtp.to_email`              | -                   |
| `KIYOMI_SMTP_SUBJECT`      | `smtp.subject`                | -                   |
| `KIYOMI_MANGA_DIR`         | `directories.manga`          | -                   |
| `KIYOMI_DELETE`            | `options.delete`             | `false`             |
| `KIYOMI_SIZE_LIMIT_MB`     | `options.size_limit_mb`      | `25`                |
| `KIYOMI_CONFIG_PATH`       | (path to the TOML file itself) | OS config dir     |
| `KIYOMI_CACHE_PATH`        | (path to the "already sent" state file) | OS cache dir |
| `RUST_LOG`                 | (log verbosity: `error`/`warn`/`info`/`debug`/`trace`) | `info` |

If a config file exists, it's loaded first and environment variables are applied on top. If no config file exists and all required variables are set via the environment, kiyomi runs without ever needing one - this is the Docker setup.

## Notes
- There are no resend checks on purpose. If the send fails, you will get an email from amazon. Delete and download a manga again to resend.
- Manga that exists in the manga directory before kiyomi starts will not be sent. Only those that are downloaded while kiyomi is running will be sent.

## Showcase
[showcase.webm](https://github.com/user-attachments/assets/cf52818d-f6e3-490d-925d-6b2fb9afa4e8)

## Systemd service example
Create a file at `/etc/systemd/system/kiyomi.service` with the following content
```ini
[Unit]
Description=Kiyomi
After=network.target

[Service]
ExecStart=/home/YOUR_USERNAME/kiyomi/target/release/kiyomi
WorkingDirectory=/home/YOUR_USERNAME/kiyomi
Restart=always
RestartSec=5
User=YOUR_USERNAME
Group=YOUR_USERNAME
Environment=PATH=/usr/bin:/usr/local/bin
Environment=HOME=/home/YOUR_USERNAME

[Install]
WantedBy=multi-user.target
```
Then enable and start the service
```bash
sudo systemctl enable kiyomi
sudo systemctl start kiyomi
```