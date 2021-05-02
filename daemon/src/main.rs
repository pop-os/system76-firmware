use dbus::blocking::Connection;
use dbus_crossroads::{Crossroads, Context, MethodErr};
use std::{io, process};

use system76_firmware::*;
use system76_firmware_daemon::*;

fn dmi_vendor() -> Result<String, String> {
    match util::read_string("/sys/class/dmi/id/sys_vendor") {
        Ok(ok) => Ok(ok.trim().to_string()),
        Err(err) => Err(format!("failed to read DMI system vendor: {}", err)),
    }
}

fn daemon() -> Result<(), String> {
    if unsafe { libc::geteuid() } != 0 {
        return Err("must be run as root".into());
    }

    // Get I/O Permission
    if unsafe { libc::iopl(3) } < 0 {
        return Err(format!(
            "failed to get I/O permission: {}",
            io::Error::last_os_error()
        ));
    }

    /// State shared across DBus calls
    struct State {
        efi_dir: String,
        in_whitelist: bool,
        transition_kind: TransitionKind
    }

    let state = State {
        efi_dir: match util::get_efi_mnt() {
            Some(x) => x,
            None => return Err("EFI mount point not found".into())
        },

        in_whitelist:
            dmi_vendor().ok().map_or(false, |vendor| vendor.contains("System76")) &&
            bios().ok().map_or(false, |(model, _)| model_is_whitelisted(&*model)),

        transition_kind: TransitionKind::Automatic
    };

    let c = Connection::new_system().map_err(err_str)?;

    c.request_name(DBUS_DEST, false, true, false).map_err(err_str)?;

    let mut cr = Crossroads::new();

    let iface_token = cr.register(DBUS_IFACE, |b| {
        b.method(
            METHOD_BIOS,
            (),
            ("model", "version"),
            move |_ctx: &mut Context, state: &mut State, _inputs: ()| {
                eprintln!("Bios");
                if !state.in_whitelist {
                    return Err(MethodErr::failed(&"product is not in whitelist"));
                }

                bios().map_err(|err| {
                    eprintln!("{}", err);
                    MethodErr::failed(&err)
                })
            }
        );

        b.method(
            METHOD_EC,
            ("primary",),
            ("project", "version"),
            |_ctx: &mut Context, state: &mut State, (primary,): (bool,)| {
                eprintln!("EmbeddedController({})", primary);
                if !state.in_whitelist {
                    return Err(MethodErr::failed(&"product is not in whitelist"));
                }

                ec(primary).map_err(|err| {
                    eprintln!("{}", err);
                    MethodErr::failed(&err)
                })
            }
        );

        b.method(
            METHOD_ME,
            (),
            ("enabled", "version"),
            |_ctx: &mut Context, state: &mut State, _inputs: ()| {
                eprintln!("ManagementEngine");
                if !state.in_whitelist {
                    return Err(MethodErr::failed(&"product is not in whitelist"));
                }

                match me() {
                    Ok(Some(me_version)) => {
                        Ok((true, me_version))
                    }
                    Ok(None) => {
                        Ok((false, String::new()))
                    }
                    Err(err) => {
                        eprintln!("{}", err);
                        Err(MethodErr::failed(&err))
                    }
                }
            }
        );

        b.method(
            METHOD_FIRMWARE_ID,
            (),
            ("id",),
            |_ctx: &mut Context, state: &mut State, _inputs: ()| {
                eprintln!("FirmwareId");
                if !state.in_whitelist {
                    return Err(MethodErr::failed(&"product is not in whitelist"));
                }

                firmware_id(state.transition_kind)
                    .map(|v| (v,))
                    .map_err(|err| {
                        eprintln!("{}", err);
                        MethodErr::failed(&err)
                    })
            }
        );

        b.method(
            METHOD_DOWNLOAD,
            (),
            ("digest", "changelog"),
            |_ctx: &mut Context, state: &mut State, _inputs: ()| {
                eprintln!("Download");
                if !state.in_whitelist {
                    return Err(MethodErr::failed(&"product is not in whitelist"));
                }

                download(state.transition_kind).map_err(|err| {
                    eprintln!("{}", err);
                    MethodErr::failed(&err)
                })
            }
        );

        b.method(
            METHOD_SCHEDULE,
            ("digest",),
            (),
            |_ctx: &mut Context, state: &mut State, (digest,): (String,)| {
                eprintln!("Schedule({})", digest);
                if !state.in_whitelist {
                    return Err(MethodErr::failed(&"product is not in whitelist"));
                }

                schedule(&digest, &state.efi_dir, state.transition_kind).map_err(|err| {
                    eprintln!("{}", err);
                    MethodErr::failed(&err)
                })
            }
        );

        b.method(
            METHOD_UNSCHEDULE,
            (),
            (),
            |_ctx: &mut Context, state: &mut State, _inputs: ()| {
                eprintln!("Unschedule");
                if !state.in_whitelist {
                    return Err(MethodErr::failed(&"product is not in whitelist"));
                }

                unschedule(&state.efi_dir).map_err(|err| {
                    eprintln!("{}", err);
                    MethodErr::failed(&err)
                })
            }
        );

        b.method(
            METHOD_THELIO_IO_DOWNLOAD,
            (),
            ("digest", "revision"),
            |_ctx: &mut Context, _state: &mut State, _inputs: ()| {
                eprintln!("ThelioIoDownload");

                thelio_io_download().map_err(|err| {
                    eprintln!("{}", err);
                    MethodErr::failed(&err)
                })
            }
        );

        b.method(
            METHOD_THELIO_IO_LIST,
            (),
            ("list",),
            |_ctx: &mut Context, _state: &mut State, _inputs: ()| {
                eprintln!("ThelioIoList");

                thelio_io_list()
                    .map(|v| (v,))
                    .map_err(|err| {
                        eprintln!("{}", err);
                        MethodErr::failed(&err)
                    })
            }
        );

        b.method(
            METHOD_THELIO_IO_UPDATE,
            ("digest",),
            (),
            |_ctx: &mut Context, _state: &mut State, (digest,): (String,)| {
                eprintln!("ThelioIoUpdate({})", digest);

                thelio_io_update(&digest).map_err(|err| {
                    eprintln!("{}", err);
                    MethodErr::failed(&err)
                })
            }
        );
    });

    cr.insert(DBUS_PATH, &[iface_token], state);

    cr.serve(&c).map_err(err_str)
}

fn main() {
    match daemon() {
        Ok(()) => (),
        Err(err) => {
            eprintln!("system76-firmware-daemon: {}", err);
            process::exit(1);
        }
    }
}
