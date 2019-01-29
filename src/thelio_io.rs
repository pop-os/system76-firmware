use buildchain::{Downloader, Manifest};
use serde::{Deserialize, Serialize};
use std::{fs, io, process, thread, time};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use {config, download, err_str};

fn read_file<P: AsRef<Path>>(path: P) -> io::Result<String> {
    fs::read_to_string(path).map(|x| x.trim().to_string())
}

fn check_file<P: AsRef<Path>>(path: P, value: &str) -> bool {
    read_file(path).ok().map_or(false, |x| x == value)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ThelioIoMetadata {
    pub device: String,
    pub revision: String,
}

#[derive(Debug)]
pub struct ThelioIoBootloader(PathBuf);

impl ThelioIoBootloader {
    fn dfu_programmer<F: FnMut(process::Command) -> io::Result<process::ExitStatus>>(
        &self, mut f: F
    ) -> io::Result<()> {
        let bus = read_file(self.0.join("busnum"))?;
        let dev = read_file(self.0.join("devnum"))?;
        let target = format!("atmega32u4:{},{}", bus, dev);
        let mut command = process::Command::new("dfu-programmer");
        command.arg(target);
        let status = f(command)?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                format!("dfu-programmer exited with {}", status)
            ))
        }
    }


    pub fn flash(&self, data: &[u8]) -> io::Result<()> {
        self.dfu_programmer(|mut command| {
            command.arg("flash");
            command.arg("--quiet");
            command.arg("STDIN");
            command.stdin(process::Stdio::piped());

            let mut process = command.spawn()?;
            process.stdin.take().unwrap().write_all(data)?;
            process.wait()
        })
    }

    pub fn reset(self) -> io::Result<()> {
        self.dfu_programmer(|mut command| {
            command.arg("reset");
            command.arg("--quiet");
            command.status()
        })
    }
}

#[derive(Debug)]
pub struct ThelioIoNormal(PathBuf);

impl ThelioIoNormal {
    pub fn revision(&self) -> io::Result<String> {
        let name = self.0.file_name().ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "ThelioIoNormal file name terminates in .."
        ))?.to_str().ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "ThelioIoNormal file name is not valid UTF-8"
        ))?;

        let iface_name = format!("{}:1.1", name);
        let path = self.0.join(&iface_name).join("revision");
        read_file(&path)
    }

    pub fn bootloader(self) -> io::Result<()> {
        let name = self.0.file_name().ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "ThelioIoNormal file name terminates in .."
        ))?.to_str().ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "ThelioIoNormal file name is not valid UTF-8"
        ))?;

        let iface_name = format!("{}:1.1", name);
        let path = self.0.join(&iface_name).join("bootloader");
        fs::write(&path, "1")
    }
}

#[derive(Debug)]
pub enum ThelioIo {
    Bootloader(ThelioIoBootloader),
    Normal(ThelioIoNormal),
}

impl ThelioIo {
    pub fn all() -> io::Result<Vec<Self>> {
        let mut all = Vec::new();

        for entry_res in fs::read_dir("/sys/bus/usb/devices")? {
            let entry = entry_res?;
            let path = entry.path();

            if let Some(item) = Self::new(path) {
                all.push(item);
            }
        }

        Ok(all)
    }

    pub fn new<P: AsRef<Path>>(path: P) -> Option<Self> {
        let path = path.as_ref();

        if ! check_file(path.join("manufacturer"), "System76") {
            return None;
        }

        if ! check_file(path.join("product"), "Io") {
            return None;
        }

        if check_file(path.join("idProduct"), "1776") {
            Some(ThelioIo::Normal(
                ThelioIoNormal(path.to_owned())
            ))
        } else {
            Some(ThelioIo::Bootloader(
                ThelioIoBootloader(path.to_owned())
            ))
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            ThelioIo::Bootloader(bootloader) => &bootloader.0,
            ThelioIo::Normal(normal) => &normal.0,
        }
    }
}

pub fn thelio_io_download() -> Result<(String, String), String> {
    let dl = Downloader::new(
        config::KEY,
        config::URL,
        config::THELIO_IO_PROJECT,
        config::BRANCH,
        Some(config::CERT)
    )?;

    let tail = dl.tail()?;

    let cache = download::Cache::new(config::CACHE, Some(dl))?;

    eprintln!("downloading manifest.json");
    let manifest_json = cache.object(&tail.digest)?;
    let manifest = serde_json::from_slice::<Manifest>(&manifest_json).map_err(|e| e.to_string())?;

    let metadata_json = {
        let file = "metadata.json";
        eprintln!("downloading {}", file);
        let digest = manifest.files.get(file).ok_or(format!("{} not found", file))?;
        cache.object(&digest)?
    };
    let metadata = serde_json::from_slice::<ThelioIoMetadata>(
        &metadata_json
    ).map_err(|e| e.to_string())?;

    let _firmware_data = {
        let file = "main.hex";
        eprintln!("downloading {}", file);
        let digest = manifest.files.get(file).ok_or(format!("{} not found", file))?;
        cache.object(&digest)?
    };

    Ok((tail.digest.to_string(), metadata.revision))
}

pub fn thelio_io_list() -> Result<HashMap<String, String>, String> {
    let mut map = HashMap::new();
    for item in ThelioIo::all().map_err(err_str)? {
        let path_str = {
            let path = item.path();
            path.to_str().ok_or(
                format!("invalid path: {:?}", path)
            )?.to_owned()
        };
        let revision = match item {
            ThelioIo::Bootloader(_bootloader) => String::new(),
            ThelioIo::Normal(normal) => normal.revision().unwrap_or(String::new()),
        };
        map.insert(path_str, revision);
    }
    Ok(map)
}

pub fn thelio_io_update(digest: &str) -> Result<(), String> {
    let cache = download::Cache::new(config::CACHE, None)?;

    let manifest_json = cache.object(&digest)?;
    let manifest = serde_json::from_slice::<Manifest>(&manifest_json).map_err(err_str)?;

    let metadata_json = {
        let file = "metadata.json";
        let digest = manifest.files.get(file).ok_or(format!("{} not found", file))?;
        cache.object(&digest)?
    };
    let metadata = serde_json::from_slice::<ThelioIoMetadata>(
        &metadata_json
    ).map_err(|e| e.to_string())?;

    let firmware_data = {
        let file = "main.hex";
        let digest = manifest.files.get(file).ok_or(format!("{} not found", file))?;
        cache.object(&digest)?
    };

    eprintln!("Switching devices to bootloader");
    let mut sleep = false;
    for thelio_io in ThelioIo::all().map_err(err_str)? {
        eprintln!(" {:?}", thelio_io.path());
        match thelio_io {
            ThelioIo::Bootloader(_) => {
                eprintln!("  already in bootloader");
            },
            ThelioIo::Normal(normal) => {
                let revision = normal.revision().unwrap_or(String::new());
                eprintln!("  revision: {:?}", revision);
                if revision != metadata.revision {
                    eprintln!("  switching to bootloader");
                    normal.bootloader().map_err(err_str)?;
                    sleep = true;
                } else {
                    eprintln!("  already up to date");
                }
            },
        }
    }

    if sleep {
        eprintln!("Waiting 5 seconds");
        thread::sleep(time::Duration::new(5, 0));
    }

    eprintln!("Flashing devices");
    sleep = false;
    for thelio_io in ThelioIo::all().map_err(err_str)? {
        eprintln!(" {:?}", thelio_io.path());
        match thelio_io {
            ThelioIo::Bootloader(bootloader) => {
                eprintln!("  flashing: {}", metadata.revision);
                bootloader.flash(&firmware_data).map_err(err_str)?;
                bootloader.reset().map_err(err_str)?;
                sleep = true;
            },
            ThelioIo::Normal(_) => {
                eprintln!("  not in bootloader!");
            }
        }
    }

    if sleep {
        eprintln!("Waiting 5 seconds");
        thread::sleep(time::Duration::new(5, 0));
    }

    eprintln!("Enumerating devices");
    for thelio_io in ThelioIo::all().map_err(err_str)? {
        eprintln!(" {:?}", thelio_io.path());
        match thelio_io {
            ThelioIo::Bootloader(_) => {
                eprintln!("  still in bootloader!");
            },
            ThelioIo::Normal(normal) => {
                let revision = normal.revision().unwrap_or(String::new());
                eprintln!("  revision: {:?}", revision);
            }
        }
    }

    Ok(())
}
