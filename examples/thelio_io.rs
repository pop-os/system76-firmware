extern crate system76_firmware;

use system76_firmware::{thelio_io_download, thelio_io_update};

fn main() -> Result<(), String> {
    let (digest, _revision) = thelio_io_download()?;
    thelio_io_update(&digest)
}
