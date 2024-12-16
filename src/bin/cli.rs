use clap::{AppSettings, Parser};
use std::{io, process};
use system76_firmware::*;

#[derive(Parser)]
#[clap(
    name = "system76-firmware-cli",
    about = "Download and install updates of System76 firmware",
    setting = AppSettings::SubcommandRequired
)]
enum Args {
    #[clap(about = "Schedule installation of firmware for next boot")]
    Schedule {
        #[clap(help = "Schedule install of open firmware", long = "open")]
        open: bool,
        #[clap(
            help = "Schedule install of proprietary firmware",
            long = "proprietary",
            conflicts_with = "open"
        )]
        proprietary: bool,
    },
    #[clap(about = "Cancel scheduled firmware installation")]
    Unschedule,
    #[clap(about = "Update Thelio IO firmware")]
    ThelioIo,
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

    let efi_dir = match util::get_efi_mnt() {
        Some(x) => x,
        None => return Err("EFI mount point not found".into()),
    };

    match Args::parse() {
        Args::Schedule { open, proprietary } => {
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
        Args::Unschedule => match unschedule(&efi_dir) {
            Ok(()) => Ok(()),
            Err(err) => Err(format!("failed to unschedule: {}", err)),
        },
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
