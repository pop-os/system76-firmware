use ecflash::{Ec, EcFlash};
use ectool::{Access, AccessLpcLinux};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::process::Command;
use std::str;
use std::time::Duration;

use crate::{err_str, util};

// Helper function for errors
pub fn ectool_err<E: ::std::fmt::Debug>(err: E) -> String {
    format!("{:?}", err)
}

pub fn ec(primary: bool) -> Result<(String, String), String> {
    let sys_vendor = match util::read_string("/sys/class/dmi/id/sys_vendor") {
        Ok(ok) => ok.trim().to_string(),
        Err(err) => {
            return Err(format!("failed to read DMI system vendor: {}", err));
        }
    };

    let product_version = match util::read_string("/sys/class/dmi/id/product_version") {
        Ok(ok) => ok.trim().to_string(),
        Err(err) => {
            return Err(format!("failed to read DMI product version: {}", err));
        }
    };

    if primary {
        unsafe {
            // Handle specific model variations
            #[allow(clippy::single_match)]
            match (sys_vendor.as_str(), product_version.as_str()) {
                ("System76", "pang12" | "pang13" | "pang14" | "pang15") => {
                    let ec_io_path = Path::new("/sys/kernel/debug/ec/ec0/io");
                    if !ec_io_path.exists() {
                        let status = Command::new("modprobe")
                            .arg("ec_sys")
                            .status()
                            .map_err(err_str)?;
                        if !status.success() {
                            return Err(format!("failed to modprobe ec_sys: {}", status));
                        }
                    }

                    let mut ec_io = File::open(ec_io_path).map_err(err_str)?;

                    let mut hms = [0u8; 3];
                    ec_io.seek(SeekFrom::Start(0x08)).map_err(err_str)?;
                    ec_io.read(&mut hms).map_err(err_str)?;

                    let mut ymd = [0u8; 3];
                    ec_io.seek(SeekFrom::Start(0x0C)).map_err(err_str)?;
                    ec_io.read(&mut ymd).map_err(err_str)?;

                    return Ok((
                        product_version,
                        format!(
                            "20{:02}/{:02}/{:02}_{:02}:{:02}:{:02}",
                            ymd[0], ymd[1], ymd[2], hms[0], hms[1], hms[2]
                        ),
                    ));
                }
                _ => (),
            }

            // Handle System76 EC
            if let Ok(access) = AccessLpcLinux::new(Duration::new(1, 0)) {
                let data_size = access.data_size();
                if let Ok(mut ec) = ectool::Ec::new(access) {
                    let version = {
                        let mut data = vec![0; data_size];
                        let count = ec.version(&mut data).map_err(ectool_err)?;
                        str::from_utf8(&data[..count]).map_err(err_str)?.to_string()
                    };
                    return Ok(("76ec".to_string(), version));
                }
            }
        }
    }

    // Fall back to proprietary EC interface
    let mut ec = EcFlash::new(primary)?;
    Ok((ec.project(), ec.version()))
}

pub fn ec_or_none(primary: bool) -> (String, String) {
    match ec(primary) {
        Ok(ok) => ok,
        Err(_err) => ("none".to_string(), "".to_string()),
    }
}
