#[macro_use]
extern crate anyhow;

use anyhow::Context;
use buildchain::{Block, Downloader, Manifest};
use std::fs;
use std::path::Path;

pub mod config;
pub mod download;
pub mod util;

mod bios;
mod boot;
mod ec;
mod me;
mod mount;
mod sideband;
mod thelio_io;
mod transition;

pub use crate::bios::bios;
pub use crate::ec::{ec, ec_or_none};
pub use crate::me::me;
pub use crate::thelio_io::{
    thelio_io_download, thelio_io_list, thelio_io_update, ThelioIo, ThelioIoMetadata,
};
pub use crate::transition::TransitionKind;

const SECONDS_IN_DAY: u64 = 60 * 60 * 24;

const MODEL_WHITELIST: &[&str] = &[
    "addw1",
    "addw2",
    "addw3",
    "addw4",
    "addw5",
    "bonw11",
    "bonw12",
    "bonw13",
    "bonw14",
    "bonw15",
    "bonw15-b",
    "bonw16",
    "darp5",
    "darp6",
    "darp7",
    "darp8",
    "darp9",
    "darp10",
    "darp10-b",
    "darp11",
    "darp11-b",
    "galp2",
    "galp3",
    "galp3-b",
    "galp3-c",
    "galp4",
    "galp5",
    "galp6",
    "galp7",
    "gaze10",
    "gaze11",
    "gaze12",
    "gaze13",
    "gaze14",
    "gaze15",
    "gaze16-3050",
    "gaze16-3060",
    "gaze16-3060-b",
    "gaze17-3050",
    "gaze17-3060-b",
    "gaze18",
    "gaze19",
    "gaze20",
    "kudu2",
    "kudu3",
    "kudu4",
    "kudu5",
    "kudu6",
    "lemu6",
    "lemu7",
    "lemu8",
    "lemp9",
    "lemp10",
    "lemp11",
    "lemp12",
    "lemp13",
    "lemp13-b",
    "meer4",
    "meer5",
    "meer6",
    "meer7",
    "meer8",
    "meer8-b",
    "meer9",
    "meer10",
    "meer10-b",
    "orxp1",
    "oryp2",
    "oryp2-ess",
    "oryp3",
    "oryp3-b",
    "oryp3-ess",
    "oryp4",
    "oryp4-b",
    "oryp5",
    "oryp6",
    "oryp7",
    "oryp8",
    "oryp9",
    "oryp10",
    "oryp11",
    "oryp12",
    "pang10",
    "pang11",
    "pang12",
    "pang13",
    "pang14",
    "pang15",
    "serw9",
    "serw10",
    "serw11",
    "serw11-b",
    "serw12",
    "serw13",
    "serw14",
    "thelio-b1",
    "thelio-b2",
    "thelio-b3",
    "thelio-b4",
    "thelio-b5",
    "thelio-major-b1",
    "thelio-major-b1.1",
    "thelio-major-b2",
    "thelio-major-b3",
    "thelio-major-b4",
    "thelio-major-r1",
    "thelio-major-r2",
    "thelio-major-r2.1",
    "thelio-major-r3",
    "thelio-major-r4",
    "thelio-major-r5",
    "thelio-mega-b1",
    "thelio-mega-r1",
    "thelio-mega-r1.1",
    "thelio-mega-r2",
    "thelio-mega-r3",
    "thelio-mega-r4",
    "thelio-mira-b1",
    "thelio-mira-b2",
    "thelio-mira-b3",
    "thelio-mira-b4",
    "thelio-mira-b4.1",
    "thelio-mira-r1",
    "thelio-mira-r2",
    "thelio-mira-r3",
    "thelio-mira-r4",
    "thelio-r1",
    "thelio-r2",
    "thelio-r3",
    "thelio-r4",
    "thelio-r5",
    "thelio-spark-b1",
    "thelio-spark-r1",
    "thelio-spark-r2",
];

pub fn model_is_whitelisted(model: &str) -> bool {
    MODEL_WHITELIST.iter().any(|whitelist| model == *whitelist)
}

// Helper function for errors
pub fn err_str<E: ::std::fmt::Display>(err: E) -> String {
    format!("{}", err)
}

pub fn model_variant(model: &str) -> Result<u8, String> {
    let pins: &[(u8, u8)] = match model {
        "gaze15" => &[
            // BOARD_ID1 = GPP_G0
            (0x6D, 0x60),
            // BOARD_ID2 = GPP_G1
            (0x6D, 0x62),
        ],
        _ => &[],
    };

    let mut variant = 0;
    if !pins.is_empty() {
        let sideband = unsafe { sideband::Sideband::new(0xFD00_0000)? };
        for (i, pin) in pins.iter().enumerate() {
            let data = unsafe { sideband.gpio(pin.0, pin.1) };
            if data & (1 << 1) > 0 {
                variant |= 1 << i;
            }
        }
    }

    Ok(variant)
}

pub fn generate_firmware_id(model: &str, project: &str) -> String {
    let project_hash = util::sha256(project.as_bytes());
    format!("{}_{}", model, project_hash)
}

pub fn firmware_id(transition_kind: TransitionKind) -> Result<String, String> {
    let (bios_model, _bios_version) = bios::bios()?;
    let variant = model_variant(&bios_model)?;
    let (ec_project, _ec_version) = ec_or_none(true);
    let (transition_model, transition_ec) =
        transition_kind.transition(&bios_model, variant, &ec_project)?;
    Ok(generate_firmware_id(&transition_model, &transition_ec))
}

fn remove_dir<P: AsRef<Path>>(path: P) -> Result<(), String> {
    if path.as_ref().exists() {
        eprintln!("removing {}", path.as_ref().display());
        match fs::remove_dir_all(&path) {
            Ok(()) => (),
            Err(err) => {
                return Err(format!(
                    "failed to remove {}: {}",
                    path.as_ref().display(),
                    err
                ));
            }
        }
    }

    Ok(())
}

pub fn download(transition_kind: TransitionKind) -> Result<(String, String), String> {
    download_firmware_id(&firmware_id(transition_kind)?)
}

pub fn download_firmware_id(firmware_id: &str) -> Result<(String, String), String> {
    let tail_path = Path::new(config::CACHE).join("tail");

    util::retry(
        || download_firmware_id_(&tail_path, firmware_id),
        || {
            fs::remove_file(&tail_path)
                .context("failed to remove tail cache")
                .map_err(err_str)
        },
    )
}

fn download_firmware_id_(tail_cache: &Path, firmware_id: &str) -> Result<(String, String), String> {
    let dl = Downloader::new(
        config::KEY,
        config::URL,
        config::PROJECT,
        config::BRANCH,
        Some(config::CERT),
    )?;

    if !Path::new(config::CACHE).is_dir() {
        eprintln!("creating cache directory {}", config::CACHE);
        fs::create_dir(config::CACHE).map_err(err_str)?;
    }

    eprintln!("downloading tail");

    let fetch_tail = || dl.tail().map_err(|why| anyhow!(why));
    let tail = cached_block(tail_cache, fetch_tail).map_err(err_str)?;

    eprintln!("opening download cache");
    let cache = download::Cache::new(config::CACHE, Some(dl))?;

    eprintln!("downloading manifest.json");
    let manifest_json = cache.object(&tail.digest)?;
    let manifest = serde_json::from_slice::<Manifest>(&manifest_json).map_err(|e| e.to_string())?;

    let _updater_data = {
        let file = "system76-firmware-update.tar.xz";
        eprintln!("downloading {}", file);
        let digest = manifest
            .files
            .get(file)
            .ok_or(format!("{} not found", file))?;
        cache.object(digest)?
    };

    let firmware_data = {
        let file = format!("{}.tar.xz", firmware_id);
        eprintln!("downloading {}", file);
        let digest = manifest
            .files
            .get(&file)
            .ok_or(format!("{} not found", file))?;
        cache.object(digest)?
    };

    eprintln!("loading changelog.json");
    let changelog = util::extract_file(&firmware_data, "./changelog.json").map_err(err_str)?;

    Ok((tail.digest, changelog))
}

/// Retrieves a `Block` from the cached path if it exists and the modified time is recent.
///
/// - If the modified time is older than one day, the cache will be updated.
/// - The most recent `Block` from cache will be returned after the cache is updated.
/// - If the cache does not require an update, it will be returned after being deserialized.
fn cached_block<F: FnMut() -> anyhow::Result<Block>>(
    path: &Path,
    mut func: F,
) -> anyhow::Result<Block> {
    let result: anyhow::Result<Block> = (|| {
        let modified =
            timestamp::modified_since_unix(path).context("could not get modified time")?;

        let now = timestamp::current();

        if timestamp::exceeded(modified, now, SECONDS_IN_DAY) {
            return Err(anyhow::anyhow!("timestamp exceeded"));
        }

        let file = fs::File::open(path).context("failed to read cached block")?;

        bincode::deserialize_from(file).context("failed to deserialize cached block")
    })();

    if result.is_err() {
        let block = func().context("failed to fetch block")?;

        let file = fs::File::create(path).context("failed to create file for cached block")?;

        bincode::serialize_into(file, &block).context("failed to cache block")?;

        Ok(block)
    } else {
        result
    }
}

fn extract<P: AsRef<Path>>(digest: &str, file: &str, path: P) -> Result<(), String> {
    let cache = download::Cache::new(config::CACHE, None)?;

    let manifest_json = cache.object(digest)?;
    let manifest = serde_json::from_slice::<Manifest>(&manifest_json).map_err(|e| e.to_string())?;

    let data = {
        let digest = manifest
            .files
            .get(file)
            .ok_or(format!("{} not found", file))?;
        cache.object(digest)?
    };

    eprintln!("extracting {} to {}", file, path.as_ref().display());
    match util::extract(&data, &path) {
        Ok(()) => (),
        Err(err) => {
            return Err(format!(
                "failed to extract {} to {}: {}",
                file,
                path.as_ref().display(),
                err
            ));
        }
    }

    Ok(())
}

pub fn schedule(
    digest: &str,
    efi_dir: &str,
    transition_kind: TransitionKind,
) -> Result<(), String> {
    schedule_firmware_id(digest, efi_dir, &firmware_id(transition_kind)?)
}

pub fn schedule_firmware_id(digest: &str, efi_dir: &str, firmware_id: &str) -> Result<(), String> {
    if !Path::new("/sys/firmware/efi").exists() {
        return Err("must be run using UEFI boot".to_string());
    }

    let updater_file = "system76-firmware-update.tar.xz";
    let firmware_file = format!("{}.tar.xz", firmware_id);
    let updater_dir = Path::new(efi_dir).join("system76-firmware-update");

    boot::unset_next_boot()?;

    remove_dir(&updater_dir)?;

    let updater_tmp = match tempfile::TempDir::with_prefix_in("system76-firmware-update.", efi_dir) {
        Ok(ok) => ok,
        Err(err) => {
            return Err(format!("failed to create temporary directory: {}", err));
        }
    };

    extract(digest, updater_file, updater_tmp.path())?;

    // tar will not create a directory if it does not exist in the archive.
    fs::create_dir(&updater_tmp.path().join("firmware")).map_err(err_str)?;
    extract(digest, &firmware_file, &updater_tmp.path().join("firmware"))?;

    let updater_tmp_dir = updater_tmp.into_path();
    eprintln!(
        "moving {} to {}",
        updater_tmp_dir.display(),
        updater_dir.display()
    );
    match fs::rename(&updater_tmp_dir, &updater_dir) {
        Ok(()) => (),
        Err(err) => {
            let _ = remove_dir(&updater_tmp_dir);
            return Err(format!(
                "failed to move {} to {}: {}",
                updater_tmp_dir.display(),
                updater_dir.display(),
                err
            ));
        }
    }

    // thelio-mira-r1/r2 will not boot to firmware updater unless it is added to BootOrder
    let modify_order =
        firmware_id.starts_with("thelio-mira-r1_") || firmware_id.starts_with("thelio-mira-r2_");
    boot::set_next_boot(efi_dir, modify_order)?;

    eprintln!("Firmware update scheduled. Reboot your machine to install.");

    Ok(())
}

pub fn unschedule(efi_dir: &str) -> Result<(), String> {
    let updater_dir = Path::new(efi_dir).join("system76-firmware-update");

    boot::unset_next_boot()?;

    remove_dir(updater_dir)?;

    eprintln!("Firmware update cancelled.");

    Ok(())
}

mod timestamp {
    use std::{
        io,
        path::Path,
        time::{Duration, SystemTime},
    };

    /// Convenience function for fetching the current time in seconds since the UNIX Epoch.
    pub fn current() -> u64 {
        seconds_since_unix(SystemTime::now())
    }

    pub fn modified_since_unix(path: &Path) -> io::Result<u64> {
        path.metadata()
            .and_then(|md| md.modified())
            .map(seconds_since_unix)
    }

    pub fn seconds_since_unix(time: SystemTime) -> u64 {
        time.duration_since(SystemTime::UNIX_EPOCH)
            .as_ref()
            .map(Duration::as_secs)
            .unwrap_or(0)
    }

    pub fn exceeded(last: u64, current: u64, limit: u64) -> bool {
        current == 0 || last > current || current - last > limit
    }
}
