use util;

pub fn bios() -> Result<(String, String), String> {
    let bios_model = match util::read_string("/sys/class/dmi/id/product_version") {
        Ok(ok) => ok.trim().to_string(),
        Err(err) => {
            return Err(format!("failed to read BIOS model: {}", err));
        }
    };

    let bios_version = match util::read_string("/sys/class/dmi/id/bios_version") {
        Ok(ok) => ok.trim().to_string(),
        Err(err) => {
            return Err(format!("failed to read BIOS version: {}", err));
        }
    };

    Ok((bios_model, bios_version))
}
