use lettre::message::header;
use lettre::message::{Attachment, Mailbox, Message, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{SmtpTransport, Transport};
use std::error::Error;
use std::fs;

/// Sends an EPUB file as an email attachment.
pub fn send_epubs_via_email(
    smtp_server: &str,
    smtp_username: &str,
    smtp_password: &str,
    from_email: &str,
    to_email: &str,
    subject: &str,
    epubs_path: &str, // all the epubs are in this directory
) -> Result<(), Box<dyn Error>> {

    // get all files in epub_path
    let epub_files = fs::read_dir(epubs_path)?
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                if e.path().is_file() {
                    Some(e.path())
                } else {
                    None
                }
            })
        })
        .collect::<Vec<_>>();

    for epub_file in epub_files {
        let epub_file = epub_file.as_path();
        let path = match epub_file.to_str() {
            Some(p) => p,
            None => {
                eprintln!("Could not convert path to string: {:?}", epub_file);
                continue;
            }
        };

        send_epub(
            smtp_server, smtp_username, smtp_password, from_email, to_email, subject, path
        )?;
    }
    Ok(())
}

/// Sends an EPUB file as an email attachment.
fn send_epub(
    smtp_server: &str,
    smtp_username: &str,
    smtp_password: &str,
    from_email: &str,
    to_email: &str,
    subject: &str,
    epub_path: &str, // all the epubs are in this directory
) -> Result<(), Box<dyn Error>> {   
     
    // get the file
    let epub_bytes = fs::read(epub_path)?;
    let epub_filename = epub_path
        .rsplit('/')
        .next()
        .unwrap_or("attachment.epub"); // fallback name

    // build the email message
    // "multipart/mixed" content type allows us to include attachments
    let email = Message::builder()
        .from(from_email.parse::<Mailbox>()?)
        .to(to_email.parse::<Mailbox>()?)
        .subject(subject)
        .multipart(
            MultiPart::mixed()
                .singlepart(
                    SinglePart::builder()
                        .header(header::ContentType::TEXT_PLAIN)
                        .body(String::from("Here's your manga!"))
                )
                .singlepart(
                    Attachment::new(String::from(epub_filename))
                        .body(epub_bytes, "application/epub+zip".parse()?)
                )
        )?;

    // create the smtp transporter
    //  - for gmail and other servers that require TLS port 587 is usually used
    //  - for local/insecure servers adapt accordingly ?
    let creds = Credentials::new(smtp_username.to_string(), smtp_password.to_string());

    let mailer = SmtpTransport::relay(smtp_server)?
        .credentials(creds)
        .build();

    match mailer.send(&email) {
        Ok(_) => {
            Ok(())
        }
        Err(e) => {
            eprintln!("Could not send email: {:?}", e);
            Err(Box::new(e))
        }
    }
}
