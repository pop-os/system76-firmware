use buildchain::{Block, Downloader, Manifest, Sha384};
use serde_json;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use config;
use err_str;

pub struct Cache {
    path: PathBuf,
    downloader: Downloader,
}

impl Cache {
    pub fn new<P: AsRef<Path>>(path: P, downloader: Downloader) -> Result<Cache, String> {
        if ! path.as_ref().is_dir() {
            fs::create_dir(path.as_ref()).map_err(err_str)?;
        }

        Ok(Cache {
            path: path.as_ref().to_owned(),
            downloader
        })
    }

    pub fn object(&self, digest: &str) -> Result<Vec<u8>, String> {
        //TODO: Atomic, with permissions

        let path = self.path.join(digest);
        if path.is_file() {
            let mut data = Vec::new();
            {
                let mut file = File::open(&path).map_err(err_str)?;
                file.read_to_end(&mut data).map_err(err_str)?;
            }

            let sha = Sha384::new(data.as_slice()).map_err(err_str)?;
            if &sha.to_base32() == digest {
                eprintln!("Used cache");
                return Ok(data);
            } else {
                fs::remove_file(&path).map_err(err_str)?;
            }
        }

        let data = self.downloader.object(digest)?;
        {
            let mut file = File::create(&path).map_err(err_str)?;
            file.write_all(&data).map_err(err_str)?;
        }
        Ok(data)
    }

    pub fn tail(&self) -> Result<Block, String> {
        self.downloader.tail()
    }
}

pub fn download(file: &str) -> Result<Vec<u8>, String> {
    let cache = Cache::new("/var/cache/system76-firmware-daemon", Downloader::new(
        config::KEY,
        config::URL,
        config::PROJECT,
        config::BRANCH,
        Some(config::CERT)
    )?)?;

    let tail = cache.tail()?;

    let manifest_json = cache.object(&tail.digest)?;
    let manifest = serde_json::from_slice::<Manifest>(&manifest_json).map_err(|e| e.to_string())?;

    if let Some(digest) = manifest.files.get(file) {
        cache.object(digest)
    } else {
        Err(format!("{} not found", file))
    }
}
