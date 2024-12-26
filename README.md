# kiyomi

When you download manga using suwayomi, kiyomi automatically sends it to your kindle. It does this by watching your manga directory for new .cbz files, converting them to .epub and emailing them to your kindle.

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

## Configuration
Kiyomi will create a config file and print its location. Edit this file to configure.

## Notes
- There are no resend checks on purpose. If the send fails, you will get an email from amazon. Delete and download a manga again to resend.
- Kiyomi will not delete the .cbz files after sending them. You can delete them manually or configure suwayomi to delete them after downloading.
- Manga that exists in the manga directory before kiyomi starts will not be sent. Only those that are downloaded while kiyomi is running will be sent.