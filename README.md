# kiyomi

When you download manga using suwayomi, kiyomi automatically sends it to your kindle. It does this by watching your
manga directory for new .cbz files, converting them to .epub and emailing them to your kindle. The generated manga is
right-to-left, correctly tagged, passes epubcheck and is actually full screen on my paperwhite (no text mode margins).

## What you need

- A kindle
- An email address whitelisted for your kindle (amazon website)
- A suwayomi server / client

> Set the download location absolute path (same as kiyomi)  
> *Settings -> Download -> **Download location***

> Suwayomi must be set to download manga **as .cbz files**  
> *Settings -> Download -> **Save as CBZ archive***

## How to use

Clone and run `cargo run --release` to start the program. Kiyomi will create a config file and print its location. Edit
this file to configure. Kiyomi will watch the specified directory for new .cbz files.

After kiyomi is running, download manga using suwayomi. Kiyomi will automagically send your manga to your kindle. Read
the logs for more information.

## Manga title format

1. If a split happened (too large to send in one email), the title will begin `N-M` where N is the current part and M is
   the total number of parts.
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

- There are no resend checks on purpose. If the send fails, you will get an email from amazon. Delete and download a
  manga again to resend.
- Manga that exists in the manga directory before kiyomi starts will not be sent. Only those that are downloaded while
  kiyomi is running will be sent.

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

## Docker

You can also run kiyomi in a docker container. There is an example docker-compose file in the repository. Make sure to
edit the environment variables and volume paths before running. No config file is required when running in Docker.

1. Create required directories on the host

kiyomi watches a directory for .cbz files and stores cache files.

```shell
mkdir -p /home/YOUR_USERNAME/manga
mkdir -p ./cache
```

The manga directory must exist before starting the container.

2. Example docker-compose.yml

```yaml
services:
  kiyomi:
    build: .
    container_name: kiyomi
    restart: unless-stopped
    environment:
      # SMTP
      KIYOMI_SMTP_SERVER: smtp.gmail.com
      KIYOMI_SMTP_USERNAME: danelnovak@gmail.com
      KIYOMI_SMTP_PASSWORD: "kiol qonh vuoq vrha"
      KIYOMI_SMTP_FROM: danelnovak@gmail.com
      KIYOMI_SMTP_TO: danielhonus_wkabpq@kindle.com
      KIYOMI_SMTP_SUBJECT: Manga

      # Manga directory inside container
      KIYOMI_MANGA_DIR: /manga

      # Options
      KIYOMI_DELETE: "true"
      KIYOMI_SIZE_LIMIT: "10"

      # Cache location inside container
      KIYOMI_CACHE: /cache

    volumes:
      # host path → container path
      - /home/daniel/manga:/manga

      # persistent cache
      - ./cache:/cache
```

3. Start kiyomi

```
docker compose up -d --build
```

Check logs:

```
docker compose logs -f
```

Expected output:

kiyomi - .cbz file watcher for kindle
Watching for new manga.cbz in "/manga"

4. Test it

Place a .cbz file into your manga folder:

cp test.cbz /home/YOUR_USERNAME/manga/

kiyomi will then detect the file, convert it to .epub, send it to your kindle and optionally delete the original .cbz
file.

> Gmail users: use an App Password
> Do NOT use your normal password.
>
> Use that value for `KIYOMI_SMTP_PASSWORD`

Environment variables reference

Required:

`KIYOMI_SMTP_SERVER`  
`KIYOMI_SMTP_USERNAME`  
`KIYOMI_SMTP_PASSWORD`  
`KIYOMI_SMTP_FROM`  
`KIYOMI_SMTP_TO`  
`KIYOMI_MANGA_DIR`

Optional:

`KIYOMI_SMTP_SUBJECT` default: Manga  
`KIYOMI_DELETE` default: false  
`KIYOMI_SIZE_LIMIT` default: 25 (MB)  
`KIYOMI_CACHE` default: ~/.cache/kiyomi
