use std::{
    fs::File,
    io::{self, BufReader, Read},
    path::Path,
};
use epub_builder::{EpubBuilder, EpubContent, EpubVersion, ZipLibrary};
use zip::ZipArchive;
use infer;

pub struct ImageFile {
    pub file_name: String, // original file name from the ZIP
    pub contents: Vec<u8>, // raw bytes of the file
    pub mime_type: String, // detected MIME type
}

/// represents a 'ComicInfo.xml' file
#[derive(Debug, Default)]
pub struct ComicInfo {
    pub title: Option<String>,
    pub writer: Option<String>,
    pub series: Option<String>,
}

pub fn extract_images_from_cbz<P: AsRef<Path>>(cbz_path: P) -> io::Result<(Vec<ImageFile>, Option<ComicInfo>)> {
    println!("- extracting images from cbz");
    let file = File::open(cbz_path)?;
    let mut comic_info = None;

    println!("- reading cbz file");
    let mut archive = ZipArchive::new(BufReader::new(file))?;
    println!("+ cbz file opened");

    let mut image_files = Vec::new();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();

        // Read file into memory
        let mut contents = Vec::new();
        entry.read_to_end(&mut contents)?;

        // Use the `infer` crate to detect MIME type
        let mime_type = match infer::get(&contents) {
            Some(kind) => kind.mime_type().to_string(),
            None => "application/octet-stream".to_string(),
        };

        if !mime_type.starts_with("image/") && entry.name() != "ComicInfo.xml" {
            println!("! skipping non-image file: {}", name);
            continue;
        }

        if entry.name() == "ComicInfo.xml" {
            comic_info = Some(parse_comicinfo(&contents));
            continue;
        }

        image_files.push(ImageFile {
            file_name: name,
            contents,
            mime_type,
        });
    }

    image_files.sort_by(|a, b| a.file_name.cmp(&b.file_name));

    Ok((image_files, comic_info))
}

pub fn build_epub_from_images(
    manga: (Vec<ImageFile>, Option<ComicInfo>),
    fallback_title: &str,
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {

    let (images, comic_info) = manga;

    let mut epub = EpubBuilder::new(ZipLibrary::new()?)?;
    epub.epub_version(EpubVersion::V30);
    
    // the first image is always the cover
    if let Some(cover) = images.first() {
        epub.add_cover_image(&cover.file_name, &cover.contents[..], &cover.mime_type)?;
    }

    let mut title = String::from(comic_info.as_ref().and_then(|ci| ci.title.as_deref()).unwrap_or(fallback_title));
    if let Some(series) = comic_info.as_ref().and_then(|ci| ci.series.as_deref()) {
        title = format!("{} - {}", series, title);
    }
    epub.metadata("title", &title)?;
    epub.metadata("author", comic_info.as_ref().and_then(|ci| ci.writer.as_deref()).unwrap_or("Unknown"))?;

    // output file! In the future there may be more than 1. We need to be under 50MB
    let mut output = File::create(format!("{}/{}.epub", output_path, title))?;

    for (i, image_file) in images.iter().enumerate() {
        let image_path = format!("images/{}", image_file.file_name);

        // 2) Create a simple XHTML page referencing that image
        let chapter_xhtml = format!("
        <!DOCTYPE html>
            <html xmlns='http://www.w3.org/1999/xhtml' xml:lang='en' lang='en'>
            <head>
                <meta charset='utf-8'/>
                <title>Manga</title>
            </head>
            <body>
                <div id='s{}'>
                    <img src=\"{image_path}\" alt='Image' />
                </div>
            </body>
            </html>
            ", i + 1
        );

        if i == 0 {
            epub.add_content(
                EpubContent::new(format!("s{}.xhtml", i + 1), chapter_xhtml.as_bytes())
                    .title("Cover")
                    .reftype(epub_builder::ReferenceType::Cover)
            )?;
            continue;
        }

        epub.add_content(
            EpubContent::new(format!("s{}.xhtml", i + 1), chapter_xhtml.as_bytes())
                .title(format!("Page {}", i + 1))
        )?;
    }

     for (_, image_file) in images.iter().enumerate() {
        println!("+ adding image: {}", image_file.file_name);
        let image_path = format!("images/{}", image_file.file_name);
        epub.add_resource(&image_path, &image_file.contents[..], &image_file.mime_type)?;

     }

    // Write out the EPUB
    epub.generate(&mut output)?;

    println!("- epub created: {}", output_path);
    Ok(())
}

pub fn parse_comicinfo(xml_bytes: &[u8]) -> ComicInfo {
    let mut comic_info = ComicInfo::default();

    let mut reader = quick_xml::Reader::from_reader(xml_bytes);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(ref e)) => {
                match e.name().as_ref() {
                    b"Series" => {
                        match reader.read_text(e.name()) {
                            Ok(t) => {
                                comic_info.series = Some(t.to_string());
                            }
                            _ => {}
                        }
                    }
                    b"Title" => {
                        match reader.read_text(e.name()) {
                            Ok(t) => {
                                comic_info.title = Some(t.to_string());
                            }
                            _ => {}
                        }
                    }
                    b"Writer" => {
                        match reader.read_text(e.name()) {
                            Ok(t) => {
                                comic_info.writer = Some(t.to_string());
                                println!("Writer: {:?}", t);
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(e) => {
                eprintln!("! couldn't parse ComicInfo.xml: {:?} Basic metadata will be used", e);
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    comic_info
}
