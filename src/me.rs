use libc;
use plain;
use std::{fs, io};
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::path::Path;
use uuid::Uuid;

use err_str;

#[repr(packed)]
struct PackedResponse(
    u8, u8, u8, u8,
    u16,
    u8, u8,
    u16, u16,
    u16,
    u8, u8,
    u16, u16,
    u16,
    u8, u8,
    u16, u16,
);

unsafe impl plain::Plain for PackedResponse {}

pub fn me() -> Result<Option<String>, String> {
    let mei_path = Path::new("/dev/mei0");
    if mei_path.exists() {
        let mut mei_f = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(mei_path)
            .map_err(err_str)?;

        let mei_fd = mei_f.as_raw_fd();

        let uuid = Uuid::parse_str("8e6a6715-9abc-4043-88ef-9e39c6f63e0f").unwrap();
        let mut uuid_bytes = [0; 16];
        {
            let (
                time_low,
                time_mid,
                time_hi_version,
                bytes
            ) = uuid.as_fields();

            uuid_bytes[0] = time_low as u8;
            uuid_bytes[1] = (time_low >> 8) as u8;
            uuid_bytes[2] = (time_low >> 16) as u8;
            uuid_bytes[3] = (time_low >> 24) as u8;

            uuid_bytes[4] = time_mid as u8;
            uuid_bytes[5] = (time_mid >> 8) as u8;

            uuid_bytes[6] = time_hi_version as u8;
            uuid_bytes[7] = (time_hi_version >> 8) as u8;

            uuid_bytes[8..].copy_from_slice(bytes);
        }

        if unsafe { libc::ioctl(mei_fd, 0xc0104801, uuid_bytes.as_mut_ptr()) } != 0 {
           return Err(format!(
               "failed to send MEI UUID: {}",
               io::Error::last_os_error()
           ));
        }

        let request = [0xFF, 0x02, 0x00, 0x00];
        mei_f.write(&request).map_err(err_str)?;

        let mut response = [0; 28];
        mei_f.read(&mut response).map_err(err_str)?;

        let packed_response: &PackedResponse = plain::from_bytes(&response).unwrap();

        let a = packed_response.5;
        let b = packed_response.4;
        let c = packed_response.8;
        let d = packed_response.7;

        Ok(Some(format!("{}.{}.{}.{}", a, b, c, d)))
    } else {
        Ok(None)
    }
}
