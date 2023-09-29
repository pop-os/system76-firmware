use std::path::Path;
use std::{fs, process};

use crate::mount;
use crate::util;

pub fn set_next_boot(efi_dir: &str, modify_order: bool) -> Result<(), String> {
    let mounts = match mount::Mount::all() {
        Ok(ok) => ok,
        Err(err) => {
            return Err(format!("failed to read mounts: {}", err));
        }
    };

    let efi_mount = match mounts.iter().find(|mount| {
        if let (Some(source), Some(dest)) = (mount.source.to_str(), mount.dest.to_str()) {
            source.starts_with('/') && dest == efi_dir
        } else {
            false
        }
    }) {
        Some(some) => some,
        None => {
            return Err(format!("failed to find mount: {}", efi_dir));
        }
    };

    let efi_dev = Path::new(&efi_mount.source);

    let efi_name = match efi_dev.file_name() {
        Some(some) => some,
        None => {
            return Err(format!("failed to get filename: {}", efi_dev.display()));
        }
    };

    let efi_sys_block = Path::new("/sys/class/block").join(efi_name);
    let efi_sys = match fs::canonicalize(&efi_sys_block) {
        Ok(ok) => ok,
        Err(err) => {
            return Err(format!(
                "failed to canonicalize {}: {}",
                efi_sys_block.display(),
                err
            ));
        }
    };

    let efi_sys_part = efi_sys.join("partition");
    let efi_part = match util::read_string(&efi_sys_part) {
        Ok(ok) => ok.trim().to_string(),
        Err(err) => {
            return Err(format!(
                "failed to read {}: {}",
                efi_sys_part.display(),
                err
            ));
        }
    };

    let disk_sys = match efi_sys.parent() {
        Some(some) => some,
        None => {
            return Err(format!("failed to get parent: {}", efi_sys.display()));
        }
    };

    let disk_name = match disk_sys.file_name() {
        Some(some) => some,
        None => {
            return Err(format!("failed to get filename: {}", disk_sys.display()));
        }
    };

    let disk_dev = Path::new("/dev").join(disk_name);

    println!("{} {}", disk_dev.display(), efi_part);

    {
        let mut command = process::Command::new("efibootmgr");
        command
            .arg("--quiet")
            .arg(if modify_order {
                "--create"
            } else {
                "--create-only"
            })
            .arg("--bootnum")
            .arg("1776")
            .arg("--disk")
            .arg(disk_dev)
            .arg("--part")
            .arg(efi_part)
            .arg("--loader")
            .arg("\\system76-firmware-update\\boot.efi")
            .arg("--label")
            .arg("system76-firmware-update");

        eprintln!("{:?}", command);

        match command.status() {
            Ok(status) => {
                if !status.success() {
                    return Err(format!("failed to add boot entry: {}", status));
                }
            }
            Err(err) => {
                return Err(format!("failed to add boot entry: {}", err));
            }
        }
    }

    {
        let mut command = process::Command::new("efibootmgr");
        command.arg("--quiet").arg("--bootnext").arg("1776");

        eprintln!("{:?}", command);

        match command.status() {
            Ok(status) => {
                if !status.success() {
                    return Err(format!("failed to set next boot: {}", status));
                }
            }
            Err(err) => {
                return Err(format!("failed to set next boot: {}", err));
            }
        }
    }

    Ok(())
}

pub fn unset_next_boot() -> Result<(), String> {
    {
        let mut command = process::Command::new("efibootmgr");
        command.arg("--quiet").arg("--delete-bootnext");

        eprintln!("{:?}", command);

        match command.status() {
            Ok(_status) => (),
            Err(err) => {
                return Err(format!("failed to unset next boot: {}", err));
            }
        }
    }

    {
        let mut command = process::Command::new("efibootmgr");
        command
            .arg("--quiet")
            .arg("--delete-bootnum")
            .arg("--bootnum")
            .arg("1776");

        eprintln!("{:?}", command);

        match command.status() {
            Ok(_status) => (),
            Err(err) => {
                return Err(format!("failed to remove boot entry: {}", err));
            }
        }
    }

    Ok(())
}
