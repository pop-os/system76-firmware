#[macro_use]
extern crate thiserror;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate shrinkwraprs;

use dbus::{ffidisp::Connection, Message};
use std::collections::HashMap;

pub const DBUS_DEST: &str = "com.system76.FirmwareDaemon";
pub const DBUS_IFACE: &str = DBUS_DEST;
pub const DBUS_PATH: &str = "/com/system76/FirmwareDaemon";

pub const METHOD_BIOS: &str = "Bios";
pub const METHOD_DOWNLOAD: &str = "Download";
pub const METHOD_EC: &str = "EmbeddedController";
pub const METHOD_FIRMWARE_ID: &str = "FirmwareId";
pub const METHOD_ME: &str = "ManagementEngine";
pub const METHOD_SCHEDULE: &str = "Schedule";
pub const METHOD_THELIO_IO_DOWNLOAD: &str = "ThelioIoDownload";
pub const METHOD_THELIO_IO_LIST: &str = "ThelioIoList";
pub const METHOD_THELIO_IO_UPDATE: &str = "ThelioIoUpdate";
pub const METHOD_UNSCHEDULE: &str = "Unschedule";

/// An error that may occur when interacting with the system76-firmware daemon.
#[derive(Debug, Error)]
pub enum Error {
    /// Received an unexpected arrangement of DBus arguments.
    #[error("argument mismatch in {} method", _0)]
    ArgumentMismatch(&'static str, #[source] dbus::arg::TypeMismatchError),
    /// Failed to call one of the daemon's methods.
    #[error("calling {} method failed", _0)]
    Call(&'static str, #[source] dbus::Error),
    /// Failed to parse the changelog file received from the daemon.
    #[error("failed to parse changelog JSON: {}", _0)]
    Changelog(Box<str>, #[source] serde_json::Error),
    /// Failed to establish a DBus connection to the system.
    #[error("unable to establish dbus connection")]
    Connection(#[source] dbus::Error),
    /// Failed to create a new method call.
    #[error("failed to create {} method call: {}", _0, _1)]
    NewMethodCall(&'static str, Box<str>),
}

/// DBus client connection for interacting with the system76-firmware daemon.
pub struct Client(Connection);

impl Client {
    pub fn new() -> Result<Self, Error> {
        Connection::new_system()
            .map_err(Error::Connection)
            .map(Self)
    }

    /// Retrieves information about the BIOS currently installed on the system.
    pub fn bios(&self) -> Result<BiosInfo, Error> {
        self.call_method(METHOD_BIOS, |m| m)?
            .read2::<String, String>()
            .map_err(|why| Error::ArgumentMismatch(METHOD_BIOS, why))
            .map(|(model, version)| BiosInfo {
                model: model.into(),
                version: version.into(),
            })
    }

    /// Downloads the latest firmware metadata for the system..
    pub fn download(&self) -> Result<SystemInfo, Error> {
        let (digest, changelog) = self
            .call_method(METHOD_DOWNLOAD, |m| m)?
            .read2::<String, String>()
            .map_err(|why| Error::ArgumentMismatch(METHOD_DOWNLOAD, why))?;

        serde_json::from_str(changelog.as_str())
            .map_err(move |why| Error::Changelog(changelog.into(), why))
            .map(move |changelog| SystemInfo {
                digest: Digest(digest.into()),
                changelog: changelog,
            })
    }

    /// Retrieves information about the EC.
    pub fn embedded_control(&self, primary: bool) -> Result<EcInfo, Error> {
        let cb = move |mut m: Message| {
            m = m.append1(primary);
            m
        };

        self.call_method(METHOD_EC, cb)?
            .read2::<String, String>()
            .map_err(|why| Error::ArgumentMismatch(METHOD_EC, why))
            .map(|(project, version)| EcInfo {
                project: project.into(),
                version: version.into(),
            })
    }

    /// Retrieves the firmware ID of the system.
    pub fn firmware_id(&self) -> Result<FirmwareId, Error> {
        self.call_method(METHOD_FIRMWARE_ID, |m| m)?
            .read1::<String>()
            .map_err(|why| Error::ArgumentMismatch(METHOD_FIRMWARE_ID, why))
            .map(|id| FirmwareId(Box::from(id)))
    }

    /// Retrieves information about the management engine.
    pub fn management_engine(&self) -> Result<IntelMeInfo, Error> {
        self.call_method(METHOD_ME, |m| m)?
            .read2::<bool, String>()
            .map_err(|why| Error::ArgumentMismatch(METHOD_ME, why))
            .map(|(enabled, version)| IntelMeInfo {
                enabled: enabled.into(),
                version: version.into(),
            })
    }

    /// Schedules a system firmware update with the given digest.
    pub fn schedule(&self, digest: &Digest) -> Result<(), Error> {
        let cb = move |mut m: Message| {
            m = m.append1(digest.0.as_ref());
            m
        };

        self.call_method(METHOD_SCHEDULE, cb).map(|_| ())
    }

    /// Downloads firmware metadata information about the Thelio I/O.
    pub fn thelio_io_download(&self) -> Result<ThelioIoInfo, Error> {
        self.call_method(METHOD_THELIO_IO_DOWNLOAD, |m| m)?
            .read2::<String, String>()
            .map_err(|why| Error::ArgumentMismatch(METHOD_THELIO_IO_DOWNLOAD, why))
            .map(|(digest, revision)| ThelioIoInfo {
                digest: Digest(digest.into()),
                revision: revision.into(),
            })
    }

    /// Retrieves a list of available Thelio I/O devices.
    pub fn thelio_io_list(&self) -> Result<ThelioIoList, Error> {
        self.call_method(METHOD_THELIO_IO_LIST, |m| m)?
            .read1::<HashMap<String, String>>()
            .map_err(|why| Error::ArgumentMismatch(METHOD_THELIO_IO_LIST, why))
            .map(ThelioIoList)
    }

    /// Updates all Thelio I/O devices in the system with the given firmware digest.
    pub fn thelio_io_update(&self, digest: &Digest) -> Result<(), Error> {
        let cb = move |mut m: Message| {
            m = m.append1(digest.0.as_ref());
            m
        };

        self.call_method(METHOD_THELIO_IO_UPDATE, cb).map(|_| ())
    }

    /// Unschedules a scheduled system firmware update.
    pub fn unschedule(&self) -> Result<(), Error> {
        self.call_method(METHOD_UNSCHEDULE, |m| m).map(|_| ())
    }

    /// Convenience method for calling a DBus method.
    fn call_method<F: FnMut(Message) -> Message>(
        &self,
        method: &'static str,
        mut append_args: F,
    ) -> Result<Message, Error> {
        let mut m = Message::new_method_call(DBUS_DEST, DBUS_PATH, DBUS_IFACE, method)
            .map_err(|why| Error::NewMethodCall(method, why.into()))?;

        m = append_args(m);

        self.0
            .send_with_reply_and_block(m, -1)
            .map_err(|why| Error::Call(method, why))
    }
}

/// Information about system's BIOS.
#[derive(Clone, Debug)]
pub struct BiosInfo {
    pub model: Box<str>,
    pub version: Box<str>,
}

/// Changelog containing details about each version of firmware.
#[derive(Clone, Debug, Deserialize)]
pub struct Changelog {
    pub versions: Vec<Version>,
}

/// Details about a version of firmware.
#[derive(Clone, Debug, Deserialize)]
pub struct Version {
    pub bios_me: bool,
    pub bios_set: bool,
    pub bios: Box<str>,
    pub description: Box<str>,
    pub me_cr: Option<bool>,
    pub me_hap: Option<bool>,
    pub me: Option<Box<str>>,
    pub date: Box<str>,
}

/// Signature of the firmware that can be installed on the system.
#[derive(Clone, Debug, Shrinkwrap)]
pub struct Digest(Box<str>);

/// Information about the latest system firmware, and all changelogs since then.
#[derive(Clone, Debug)]
pub struct SystemInfo {
    pub digest: Digest,
    pub changelog: Changelog,
}

/// Information about the EC.
#[derive(Clone, Debug)]
pub struct EcInfo {
    pub project: Box<str>,
    pub version: Box<str>,
}

/// A signature describing the current system firmware.
#[derive(Clone, Debug, Shrinkwrap)]
pub struct FirmwareId(Box<str>);

/// Information about the Intel ME.
#[derive(Clone, Debug)]
pub struct IntelMeInfo {
    pub enabled: bool,
    pub version: Box<str>,
}

/// The latest firmware information for Thelio I/O devices.
#[derive(Clone, Debug)]
pub struct ThelioIoInfo {
    pub digest: Digest,
    pub revision: Box<str>,
}

/// A list of Thelio I/O devices discovered on the system.
#[derive(Clone, Debug, Shrinkwrap)]
pub struct ThelioIoList(pub HashMap<String, String>);
