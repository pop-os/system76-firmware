use clap::{App, AppSettings, Arg, SubCommand};
use std::{io, process};
use system76_firmware::*;

fn tool() -> Result<(), String> {
    if unsafe { libc::geteuid() } != 0 {
        return Err(format!("must be run as root"));
    }

    // Get I/O Permission
    if unsafe { libc::iopl(3) } < 0 {
        return Err(format!(
            "failed to get I/O permission: {}",
            io::Error::last_os_error()
        ));
    }

    let efi_dir = match util::get_efi_mnt() {
        Some(x) => x,
        None => return Err("EFI mount point not found".into())
    };

    let matches = App::new("system76-firmware-cli")
        .setting(AppSettings::SubcommandRequired)
        .setting(AppSettings::DisableVersion)
        .setting(AppSettings::VersionlessSubcommands)
        .subcommand(SubCommand::with_name("schedule")
                    .arg(Arg::with_name("open")
                         .long("open"))
                    .arg(Arg::with_name("--proprietary")
                         .long("proprietary")
                         .conflicts_with("open")))
        .subcommand(SubCommand::with_name("unschedule"))
        .subcommand(SubCommand::with_name("thelio-io"))
        .get_matches();

    match matches.subcommand() {
        ("schedule", Some(sub_m)) => {
            let transition_kind = if sub_m.is_present("open") {
                TransitionKind::Open
            } else if matches.is_present("proprietary") {
                TransitionKind::Proprietary
            } else {
                TransitionKind::Automatic
            };

            let (digest, _changelog) = match download(transition_kind) {
                Ok(ok) => ok,
                Err(err) => return Err(format!("failed to download: {}", err))
            };

            match schedule(&digest, &efi_dir, transition_kind) {
                Ok(()) => Ok(()),
                Err(err) => Err(format!("failed to schedule: {}", err))
            }
        }
        ("unschedule", Some(_)) => {
            match unschedule(&efi_dir) {
                Ok(()) => Ok(()),
                Err(err) => Err(format!("failed to unschedule: {}", err))
            }
        }
        ("thelio-io", Some(_)) => {
            let (digest, _revision) = match thelio_io_download() {
                Ok(ok) => ok,
                Err(err) => return Err(format!("failed to download: {}", err))
            };

            match thelio_io_update(&digest) {
                Ok(()) => Ok(()),
                Err(err) => Err(format!("failed to update: {}", err))
            }
        }
        _ => unreachable!()
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
