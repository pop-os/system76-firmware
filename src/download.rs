use buildchain::{Downloader, Sha384};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::err_str;

pub struct Cache {
    path: PathBuf,
    downloader: Option<Downloader>,
}

impl Cache {
    pub fn new<P: AsRef<Path>>(path: P, downloader: Option<Downloader>) -> Result<Cache, String> {
        if !path.as_ref().is_dir() {
            fs::create_dir(path.as_ref()).map_err(err_str)?;
        }

        Ok(Cache {
            path: path.as_ref().to_owned(),
            downloader,
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
            if sha.to_base32() == digest {
                return Ok(data);
            } else {
                fs::remove_file(&path).map_err(err_str)?;
            }
        }

        if let Some(ref downloader) = self.downloader {
            let data = downloader.object(digest)?;
            {
                let mut file = File::create(&path).map_err(err_str)?;
                file.write_all(&data).map_err(err_str)?;
            }
            Ok(data)
        } else {
            Err(format!("could not find digest in cache: {}", digest))
        }
    }
}
