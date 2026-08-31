use clap::Parser;
use std::{io, process};
use system76_firmware::*;

#[derive(Parser)]
#[clap(
    name = "system76-firmware-cli",
    about = "Download and install updates of System76 firmware"
)]
enum Args {
    #[clap(about = "Check if firmware updates are available")]
    Check,
    #[clap(about = "Schedule installation of firmware for next boot")]
    #[group(multiple = false)]
    Schedule {
        #[clap(help = "Schedule install of open firmware", long = "open")]
        open: bool,
        #[clap(
            help = "Schedule install of proprietary firmware",
            long = "proprietary"
        )]
        proprietary: bool,
    },
    #[clap(about = "Cancel scheduled firmware installation")]
    Unschedule,
    #[clap(about = "Update Thelio IO firmware")]
    ThelioIo,
}

// Local deserialization structs for changelog.json.
// The canonical types live in the daemon crate, which we can't import here
// without creating a circular dependency.
#[derive(serde::Deserialize)]
struct Changelog {
    versions: Vec<ChangelogVersion>,
}

#[derive(serde::Deserialize)]
struct ChangelogVersion {
    bios: String,
    me: Option<String>,
}

fn efi_dir() -> Result<String, String> {
    util::get_efi_mnt().ok_or_else(|| "EFI mount point not found".into())
}

fn check() -> Result<bool, String> {
    let (model, current_bios) = bios()?;

    let current_me = me()?;

    let (_digest, changelog_json) = download(TransitionKind::Automatic)
        .map_err(|err| format!("failed to download: {}", err))?;

    let changelog: Changelog = serde_json::from_str(&changelog_json)
        .map_err(|err| format!("failed to parse changelog: {}", err))?;

    let latest = changelog
        .versions
        .first()
        .ok_or_else(|| "changelog has no versions".to_string())?;

    let bios_outdated = current_bios != latest.bios;
    println!("BIOS:");
    println!("  Model:     {}", model);
    println!("  Current:   {}", current_bios);
    println!("  Available: {}", latest.bios);
    println!("  {}", if bios_outdated { "Update available" } else { "Up to date" });

    let me_outdated = if let Some(ref current_me) = current_me {
        println!();
        println!("ME:");
        println!("  Current:   {}", current_me);
        if let Some(ref available_me) = latest.me {
            let outdated = current_me != available_me;
            println!("  Available: {}", available_me);
            println!("  {}", if outdated { "Update available" } else { "Up to date" });
            outdated
        } else {
            println!("  No ME update in changelog");
            false
        }
    } else {
        false
    };

    let update_available = bios_outdated || me_outdated;

    if update_available {
        println!();
        println!("Firmware update available. Run 'system76-firmware-cli schedule' to install.");
    }

    Ok(update_available)
}

fn tool() -> Result<(), String> {
    if unsafe { libc::geteuid() } != 0 {
        return Err("must be run as root".to_string());
    }

    // Get I/O Permission
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    if unsafe { libc::iopl(3) } < 0 {
        return Err(format!(
            "failed to get I/O permission: {}",
            io::Error::last_os_error()
        ));
    }

    match Args::parse() {
        Args::Check => {
            let update_available = check()?;
            if update_available {
                process::exit(2);
            }
            Ok(())
        }
        Args::Schedule { open, proprietary } => {
            let efi_dir = efi_dir()?;
            let transition_kind = if open {
                TransitionKind::Open
            } else if proprietary {
                TransitionKind::Proprietary
            } else {
                TransitionKind::Automatic
            };

            let (digest, _changelog) = match download(transition_kind) {
                Ok(ok) => ok,
                Err(err) => return Err(format!("failed to download: {}", err)),
            };

            match schedule(&digest, &efi_dir, transition_kind) {
                Ok(()) => Ok(()),
                Err(err) => Err(format!("failed to schedule: {}", err)),
            }
        }
        Args::Unschedule => {
            let efi_dir = efi_dir()?;
            match unschedule(&efi_dir) {
                Ok(()) => Ok(()),
                Err(err) => Err(format!("failed to unschedule: {}", err)),
            }
        }
        Args::ThelioIo => {
            let (digest, _revision) = match thelio_io_download() {
                Ok(ok) => ok,
                Err(err) => return Err(format!("failed to download: {}", err)),
            };

            match thelio_io_update(&digest) {
                Ok(()) => Ok(()),
                Err(err) => Err(format!("failed to update: {}", err)),
            }
        }
    }
}

fn main() {
    match tool() {
        Ok(()) => (),
        Err(err) => {
            eprintln!("system76-firmware: {}", err);
            process::exit(1);
        }
    }
}
