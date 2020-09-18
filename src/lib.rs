use buildchain::{Downloader, Manifest};
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
    ThelioIo, ThelioIoMetadata,
    thelio_io_download, thelio_io_list, thelio_io_update
};
pub use crate::transition::TransitionKind;

const MODEL_WHITELIST: &[&str] = &[
    "addw1",
    "addw2",
    "bonw11",
    "bonw12",
    "bonw13",
    "bonw14",
    "darp5",
    "darp6",
    "galp2",
    "galp3",
    "galp3-b",
    "galp3-c",
    "galp4",
    "gaze10",
    "gaze11",
    "gaze12",
    "gaze13",
    "gaze14",
    "gaze15",
    "kudu2",
    "kudu3",
    "kudu4",
    "kudu5",
    "lemu6",
    "lemu7",
    "lemu8",
    "lemp9",
    "meer4",
    "meer5",
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
    "serw9",
    "serw10",
    "serw11",
    "serw11-b",
    "serw12",
    "thelio-b1",
    "thelio-b2",
    "thelio-major-b1",
    "thelio-major-b1.1",
    "thelio-major-b2",
    "thelio-major-r1",
    "thelio-major-r2",
    "thelio-major-r2.1",
    "thelio-mega-b1",
    "thelio-mega-r1",
    "thelio-mega-r1.1",
    "thelio-r1",
    "thelio-r2",
];

pub fn model_is_whitelisted(model: &str) -> bool {
    MODEL_WHITELIST
        .into_iter()
        .find(|whitelist| model == **whitelist)
        .is_some()
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
    if ! pins.is_empty() {
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
    let (transition_model, transition_ec) = transition_kind.transition(&bios_model, variant, &ec_project);
    Ok(generate_firmware_id(&transition_model, &transition_ec))
}

fn remove_dir<P: AsRef<Path>>(path: P) -> Result<(), String> {
    if path.as_ref().exists() {
        eprintln!("removing {}", path.as_ref().display());
        match fs::remove_dir_all(&path) {
            Ok(()) => (),
            Err(err) => {
                return Err(format!("failed to remove {}: {}", path.as_ref().display(), err));
            }
        }
    }

    Ok(())
}

pub fn download(transition_kind: TransitionKind) -> Result<(String, String), String> {
    download_firmware_id(&firmware_id(transition_kind)?)
}

pub fn download_firmware_id(firmware_id: &str) -> Result<(String, String), String> {
    let dl = Downloader::new(
        config::KEY,
        config::URL,
        config::PROJECT,
        config::BRANCH,
        Some(config::CERT)
    )?;

    eprintln!("downloading tail");
    let tail = dl.tail()?;

    eprintln!("opening download cache");
    let cache = download::Cache::new(config::CACHE, Some(dl))?;

    eprintln!("downloading manifest.json");
    let manifest_json = cache.object(&tail.digest)?;
    let manifest = serde_json::from_slice::<Manifest>(&manifest_json).map_err(|e| e.to_string())?;

    let _updater_data = {
        let file = "system76-firmware-update.tar.xz";
        eprintln!("downloading {}", file);
        let digest = manifest.files.get(file).ok_or(format!("{} not found", file))?;
        cache.object(&digest)?
    };

    let firmware_data = {
        let file = format!("{}.tar.xz", firmware_id);
        eprintln!("downloading {}", file);
        let digest = manifest.files.get(&file).ok_or(format!("{} not found", file))?;
        cache.object(&digest)?
    };

    eprintln!("loading changelog.json");
    let changelog = util::extract_file(&firmware_data, "./changelog.json").map_err(err_str)?;

    Ok((tail.digest.to_string(), changelog))
}

fn extract<P: AsRef<Path>>(digest: &str, file: &str, path: P) -> Result<(), String> {
    let cache = download::Cache::new(config::CACHE, None)?;

    let manifest_json = cache.object(&digest)?;
    let manifest = serde_json::from_slice::<Manifest>(&manifest_json).map_err(|e| e.to_string())?;

    let data = {
        let digest = manifest.files.get(file).ok_or(format!("{} not found", file))?;
        cache.object(&digest)?
    };

    eprintln!("extracting {} to {}", file, path.as_ref().display());
    match util::extract(&data, &path) {
        Ok(()) => (),
        Err(err) => {
            return Err(format!("failed to extract {} to {}: {}", file, path.as_ref().display(), err));
        }
    }

    Ok(())
}

pub fn schedule(digest: &str, efi_dir: &str, transition_kind: TransitionKind) -> Result<(), String> {
    schedule_firmware_id(digest, efi_dir, &firmware_id(transition_kind)?)
}

pub fn schedule_firmware_id(digest: &str, efi_dir: &str, firmware_id: &str) -> Result<(), String> {
    if ! Path::new("/sys/firmware/efi").exists() {
        return Err(format!("must be run using UEFI boot"));
    }

    let updater_file = "system76-firmware-update.tar.xz";
    let firmware_file = format!("{}.tar.xz", firmware_id);
    let updater_dir = Path::new(efi_dir).join("system76-firmware-update");

    boot::unset_next_boot()?;

    remove_dir(&updater_dir)?;

    let updater_tmp = match tempdir::TempDir::new_in(efi_dir, "system76-firmware-update") {
        Ok(ok) => ok,
        Err(err) => {
            return Err(format!("failed to create temporary directory: {}", err));
        }
    };

    extract(digest, updater_file, updater_tmp.path())?;

    extract(digest, &firmware_file, &updater_tmp.path().join("firmware"))?;

    let updater_tmp_dir = updater_tmp.into_path();
    eprintln!("moving {} to {}", updater_tmp_dir.display(), updater_dir.display());
    match fs::rename(&updater_tmp_dir, &updater_dir) {
        Ok(()) => (),
        Err(err) => {
            let _ = remove_dir(&updater_tmp_dir);
            return Err(format!("failed to move {} to {}: {}", updater_tmp_dir.display(), updater_dir.display(), err));
        }
    }

    boot::set_next_boot(efi_dir)?;

    eprintln!("Firmware update scheduled. Reboot your machine to install.");

    Ok(())
}

pub fn unschedule(efi_dir: &str) -> Result<(), String> {
    let updater_dir = Path::new(efi_dir).join("system76-firmware-update");

    boot::unset_next_boot()?;

    remove_dir(&updater_dir)?;

    eprintln!("Firmware update cancelled.");

    Ok(())
}
