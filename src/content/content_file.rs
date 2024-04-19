use std::{fs, io};
use std::io::ErrorKind;
use std::path::PathBuf;

use crate::content::content_format::ContentFormat;

pub struct ContentFile {
    pub link: String,
    pub file_path: PathBuf,
    pub format: ContentFormat,
    pub raw_content: String,
}

impl ContentFile {
    pub fn from_file(link: String, file_path: PathBuf) -> io::Result<ContentFile> {
        let format = match Self::guess_type(&file_path) {
            None => return Err(io::Error::new(ErrorKind::Unsupported, format!("Could not guess the type of the file {}", &file_path.to_str().unwrap()))),
            Some(format) => format,
        };

        let raw_content = fs::read_to_string(&file_path)?;

        Ok(ContentFile {
            link,
            file_path,
            format,
            raw_content,
        })
    }

    fn guess_type(file_name: &PathBuf) -> Option<ContentFormat> {
        match file_name.to_str().unwrap() {
            x if x.ends_with(".md") => Some(ContentFormat::Texted),
            x if x.ends_with(".html") || x.ends_with(".htm") => Some(ContentFormat::Html),
            _ => return None,
        }
    }
}