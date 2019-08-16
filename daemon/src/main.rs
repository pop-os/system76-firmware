use dbus::tree::{Factory, MethodErr};
use dbus::{BusType, Connection, NameFlag};
use std::collections::HashMap;
use std::{io, process};

use system76_firmware::*;
use system76_firmware_daemon::*;

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

    let c = Connection::get_private(BusType::System).map_err(err_str)?;
    c.register_name(DBUS_DEST, NameFlag::ReplaceExisting as u32)
        .map_err(err_str)?;

    let f = Factory::new_fn::<()>();

    let tree = f.tree(()).add(
        f.object_path(DBUS_PATH, ()).introspectable().add(
            f.interface(DBUS_DEST, ())
                .add_m(
                    f.method(METHOD_BIOS, (), move |m| {
                        eprintln!("Bios");
                        match bios() {
                            Ok((bios_model, bios_version)) => {
                                let mret = m.msg.method_return().append2(bios_model, bios_version);
                                Ok(vec![mret])
                            }
                            Err(err) => {
                                eprintln!("{}", err);
                                Err(MethodErr::failed(&err))
                            }
                        }
                    })
                    .outarg::<&str, _>("model")
                    .outarg::<&str, _>("version"),
                )
                .add_m(
                    f.method(METHOD_EC, (), move |m| {
                        let primary = m.msg.read1()?;
                        eprintln!("EmbeddedController({})", primary);
                        match ec(primary) {
                            Ok((ec_project, ec_version)) => {
                                let mret = m.msg.method_return().append2(ec_project, ec_version);
                                Ok(vec![mret])
                            }
                            Err(err) => {
                                eprintln!("{}", err);
                                Err(MethodErr::failed(&err))
                            }
                        }
                    })
                    .inarg::<bool, _>("primary")
                    .outarg::<&str, _>("project")
                    .outarg::<&str, _>("version"),
                )
                .add_m(
                    f.method(METHOD_ME, (), move |m| {
                        eprintln!("ManagementEngine");
                        match me() {
                            Ok(Some(me_version)) => {
                                let mret = m.msg.method_return().append2(true, me_version);
                                Ok(vec![mret])
                            }
                            Ok(None) => {
                                let mret = m.msg.method_return().append2(false, "");
                                Ok(vec![mret])
                            }
                            Err(err) => {
                                eprintln!("{}", err);
                                Err(MethodErr::failed(&err))
                            }
                        }
                    })
                    .outarg::<bool, _>("enabled")
                    .outarg::<&str, _>("version"),
                )
                .add_m(
                    f.method(METHOD_FIRMWARE_ID, (), move |m| {
                        eprintln!("FirmwareId");
                        match firmware_id() {
                            Ok(id) => {
                                let mret = m.msg.method_return().append1(id);
                                Ok(vec![mret])
                            }
                            Err(err) => {
                                eprintln!("{}", err);
                                Err(MethodErr::failed(&err))
                            }
                        }
                    })
                    .outarg::<&str, _>("id"),
                )
                .add_m(
                    f.method(METHOD_DOWNLOAD, (), move |m| {
                        eprintln!("Download");
                        match download() {
                            Ok((digest, changelog)) => {
                                let mret = m.msg.method_return().append2(digest, changelog);
                                Ok(vec![mret])
                            }
                            Err(err) => {
                                eprintln!("{}", err);
                                Err(MethodErr::failed(&err))
                            }
                        }
                    })
                    .outarg::<&str, _>("digest")
                    .outarg::<&str, _>("changelog"),
                )
                .add_m(
                    f.method(METHOD_SCHEDULE, (), move |m| {
                        let digest = m.msg.read1()?;
                        eprintln!("Schedule({})", digest);
                        match schedule(digest) {
                            Ok(()) => {
                                let mret = m.msg.method_return();
                                Ok(vec![mret])
                            }
                            Err(err) => {
                                eprintln!("{}", err);
                                Err(MethodErr::failed(&err))
                            }
                        }
                    })
                    .inarg::<&str, _>("digest"),
                )
                .add_m(f.method(METHOD_UNSCHEDULE, (), move |m| {
                    eprintln!("Unschedule");
                    match unschedule() {
                        Ok(()) => {
                            let mret = m.msg.method_return();
                            Ok(vec![mret])
                        }
                        Err(err) => {
                            eprintln!("{}", err);
                            Err(MethodErr::failed(&err))
                        }
                    }
                }))
                .add_m(
                    f.method(METHOD_THELIO_IO_DOWNLOAD, (), move |m| {
                        eprintln!("ThelioIoDownload");
                        match thelio_io_download() {
                            Ok((digest, revision)) => {
                                let mret = m.msg.method_return().append2(digest, revision);
                                Ok(vec![mret])
                            }
                            Err(err) => {
                                eprintln!("{}", err);
                                Err(MethodErr::failed(&err))
                            }
                        }
                    })
                    .outarg::<&str, _>("digest")
                    .outarg::<&str, _>("revision"),
                )
                .add_m(
                    f.method(METHOD_THELIO_IO_LIST, (), move |m| {
                        eprintln!("ThelioIoList");
                        match thelio_io_list() {
                            Ok(list) => {
                                let mret = m.msg.method_return().append1(list);
                                Ok(vec![mret])
                            }
                            Err(err) => {
                                eprintln!("{}", err);
                                Err(MethodErr::failed(&err))
                            }
                        }
                    })
                    .outarg::<HashMap<String, String>, _>("list"),
                )
                .add_m(
                    f.method(METHOD_THELIO_IO_UPDATE, (), move |m| {
                        let digest = m.msg.read1()?;
                        eprintln!("ThelioIoUpdate({})", digest);
                        match thelio_io_update(digest) {
                            Ok(()) => {
                                let mret = m.msg.method_return();
                                Ok(vec![mret])
                            }
                            Err(err) => {
                                eprintln!("{}", err);
                                Err(MethodErr::failed(&err))
                            }
                        }
                    })
                    .inarg::<&str, _>("digest"),
                ),
        ),
    );

    tree.set_registered(&c, true).map_err(err_str)?;

    c.add_handler(tree);

    loop {
        c.incoming(1000).next();
    }
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
