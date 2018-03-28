use ecflash::{Ec, EcFlash};

pub fn ec(primary: bool) -> Result<(String, String), String> {
    let mut ec = EcFlash::new(primary)?;
    Ok((ec.project(), ec.version()))
}
