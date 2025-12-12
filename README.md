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
Clone and run `cargo run --release` to start the program. Kiyomi will create a config file and print its location. Edit this file to configure. Kiyomi will watch the specified directory for new .cbz files.

After kiyomi is running, download manga using suwayomi. Kiyomi will automagically send your manga to your kindle. Read the logs for more information.

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
size_limit = 25
```

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