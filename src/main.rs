extern crate buildchain;
extern crate ecflash;
extern crate serde_json;
extern crate sha2;
extern crate tar;
extern crate tempdir;
extern crate xz2;

use std::{fs, io, path, process};

mod config;
mod ec;
mod download;
mod util;

fn remove_dir<P: AsRef<path::Path>>(path: P) -> Result<(), String> {
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

fn download_and_extract<P: AsRef<path::Path>>(file: &str, path: P) -> Result<(), String> {
    eprintln!("downloading {}", file);
    let data = match download::download(file) {
        Ok(ok) => ok,
        Err(err) => {
            return Err(format!("failed to download {}: {}", file, err));
        }
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

fn update() -> Result<(), String> {
    extern {
        fn geteuid() -> isize;
        fn iopl(level: isize) -> isize;
    }

    if unsafe { geteuid() } != 0 {
        return Err(format!("must be run as root"));
    }

    // Get I/O Permission
    if unsafe { iopl(3) } < 0 {
        return Err(format!(
            "failed to get I/O permission: {}",
            io::Error::last_os_error()
        ));
    }

    let bios_model = match util::read_string("/sys/class/dmi/id/product_version") {
        Ok(ok) => ok.trim().to_string(),
        Err(err) => {
            return Err(format!("failed to read BIOS model: {}", err));
        }
    };

    let bios_version = match util::read_string("/sys/class/dmi/id/bios_version") {
        Ok(ok) => ok.trim().to_string(),
        Err(err) => {
            return Err(format!("failed to read BIOS version: {}", err));
        }
    };

    let (ec_project, ec_version) = match ec::ec(true) {
        Ok(ok) => ok,
        Err(err) => {
            eprintln!("system76-firmware-daemon: failed to read EC: {}", err);
            ("none".to_string(), "none".to_string())
        }
    };

    eprintln!("BIOS Model: {}", bios_model);
    eprintln!("BIOS Version: {}", bios_version);
    eprintln!("EC Project: {}", ec_project);
    eprintln!("EC Version: {}", ec_version);

    let ec_hash = util::sha256(ec_project.as_bytes());
    let firmware_id = format!("{}_{}", bios_model, ec_hash);
    eprintln!("Firmware ID: {}", firmware_id);

    let updater_file = "system76-firmware-update.tar.xz";
    let firmware_file = format!("{}.tar.xz", firmware_id);
    let updater_dir = path::Path::new("/boot/efi/system76-firmware-update");

    remove_dir(&updater_dir)?;

    let updater_tmp = match tempdir::TempDir::new_in("/boot/efi", "system76-firmware-update") {
        Ok(ok) => ok,
        Err(err) => {
            return Err(format!("failed to create temporary directory: {}", err));
        }
    };

    download_and_extract(updater_file, updater_tmp.path())?;

    download_and_extract(&firmware_file, &updater_tmp.path().join("firmware"))?;

    let updater_tmp_dir = updater_tmp.into_path();
    eprintln!("moving {} to {}", updater_tmp_dir.display(), updater_dir.display());
    match fs::rename(&updater_tmp_dir, &updater_dir) {
        Ok(()) => (),
        Err(err) => {
            let _ = remove_dir(&updater_tmp_dir);
            return Err(format!("failed to move {} to {}: {}", updater_tmp_dir.display(), updater_dir.display(), err));
        }
    }

    Ok(())
}

fn main() {
    match update() {
        Ok(()) => {
            eprintln!("Firmware update prepared. Reboot your machine to install.");
            process::exit(0)
        },
        Err(err) => {
            eprintln!("system76-firmware-daemon: {}", err);
            process::exit(1)
        }
    }
}
