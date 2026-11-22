use std::path::PathBuf;

use flate2::read::GzDecoder;
use tar::Archive;

pub fn decompress_files(output: &PathBuf) -> Result<(), std::io::Error> {
    let tar_gz = include_bytes!(concat!(env!("OUT_DIR"), "/res.tar.gz"));
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
        let out_path =
            std::env::temp_dir().join(format!("texted-bootstrap-{}", uuid::Uuid::new_v4()));
        decompress_files(&out_path).unwrap();
        for file in [
            "template/page.tpl",
            "template/content.tpl",
            "template/contentlist.tpl",
            "docs/introduction.md",
        ] {
            assert!(
                out_path.join(file).is_file(),
                "Missing bootstrap resource: {file}"
            );
        }
        std::fs::remove_dir_all(out_path).unwrap();
    }
}
