use ecflash::{Ec, EcFlash};
use ectool::Timeout;
use std::str;
use std::time::{Duration, Instant};

use err_str;

// Helper function for errors
pub fn ectool_err<E: ::std::fmt::Debug>(err: E) -> String {
    format!("{:?}", err)
}

pub struct StdTimeout {
    instant: Instant,
    duration: Duration,
}

impl StdTimeout {
    pub fn new(duration: Duration) -> Self {
        StdTimeout {
            instant: Instant::now(),
            duration
        }
    }
}

impl Timeout for StdTimeout {
    fn reset(&mut self) {
        self.instant = Instant::now();
    }

    fn running(&self) -> bool {
        self.instant.elapsed() < self.duration
    }
}

pub fn ec(primary: bool) -> Result<(String, String), String> {
    if primary {
        unsafe {
            if let Ok(mut ec) = ectool::Ec::new(
                StdTimeout::new(Duration::new(1, 0))
            ) {
                let version = {
                    let mut data = [0; 256];
                    let count = ec.version(&mut data).map_err(ectool_err)?;
                    str::from_utf8(&data[..count]).map_err(err_str)?.to_string()
                };
                return Ok(("76ec".to_string(), version));
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
