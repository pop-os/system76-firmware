use std::process;
use system76_firmware::{bios, ec_or_none, generate_firmware_id, model_variant, TransitionKind};

fn inner() -> Result<(), String> {
    let (bios_model, _bios_version) = bios()?;
    let variant = model_variant(&bios_model)?;
    let (ec_project, _ec_version) = ec_or_none(true);

    println!("Model: {}", bios_model);
    println!("Variant: {}", variant);
    println!("Project: {}", ec_project);

    for transition_kind in &[
        TransitionKind::Automatic,
        TransitionKind::Open,
        TransitionKind::Proprietary,
    ] {
        println!("{:?}", transition_kind);
        match transition_kind.transition(&bios_model, variant, &ec_project) {
            Ok((transition_model, transition_ec)) => {
                println!("  Model: {}", transition_model);
                println!("  Project: {}", transition_ec);
                println!(
                    "  Firmware ID: {}",
                    generate_firmware_id(&transition_model, &transition_ec)
                );
            }
            Err(err) => {
                println!("  Error: {}", err);
            }
        }
    }

    Ok(())
}

fn main() {
    if unsafe { libc::geteuid() } != 0 {
        eprintln!("must be run as root");
        process::exit(1);
    }

    if let Err(err) = inner() {
        eprintln!("{}", err);
        process::exit(1);
    }
}
