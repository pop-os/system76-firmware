use ecflash::{Ec, EcFlash};
use ectool::{Access, AccessLpcLinux};
use std::str;
use std::time::Duration;

use crate::err_str;

// Helper function for errors
pub fn ectool_err<E: ::std::fmt::Debug>(err: E) -> String {
    format!("{:?}", err)
}

pub fn ec(primary: bool) -> Result<(String, String), String> {
    if primary {
        unsafe {
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
    let mut ec = EcFlash::new(primary)?;
    Ok((ec.project(), ec.version()))
}

pub fn ec_or_none(primary: bool) -> (String, String) {
    match ec(primary) {
        Ok(ok) => ok,
        Err(_err) => ("none".to_string(), "".to_string())
    }
}
