use std::path::PathBuf;

use flate2::read::GzDecoder;
use tar::Archive;

pub fn decompress_files(output: &PathBuf) -> Result<(), std::io::Error> {
    let tar_gz = include_bytes!("../../../res.tar.gz");
    let tar = GzDecoder::new(tar_gz.as_ref());
    let mut archive = Archive::new(tar);
    archive.unpack(output)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uncompress() {
        let out_path = PathBuf::from("/Users/thiago/src/texted/z");
        decompress_files(&out_path).unwrap()
    }
}