use buildchain::{Downloader, Manifest};
use serde_json;

use config;

pub fn download(file: &str) -> Result<Vec<u8>, String> {
    let dl = Downloader::new(
        config::KEY,
        config::URL,
        config::PROJECT,
        config::BRANCH,
        Some(config::CERT)
    )?;

    let tail = dl.tail()?;

    let manifest_json = dl.object(&tail.digest)?;
    let manifest = serde_json::from_slice::<Manifest>(&manifest_json).map_err(|e| e.to_string())?;

    if let Some(digest) = manifest.files.get(file) {
        dl.object(digest)
    } else {
        Err(format!("{} not found", file))
    }
}
