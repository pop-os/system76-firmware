use ecflash::{Ec, EcFlash};

pub fn ec(primary: bool) -> Result<(String, String), String> {
    let mut ec = EcFlash::new(primary)?;
    Ok((ec.project(), ec.version()))
}

pub fn ec_or_none(primary: bool) -> (String, String) {
    match ec(primary) {
        Ok(ok) => ok,
        Err(_err) => ("none".to_string(), "".to_string())
    }
}
