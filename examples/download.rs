use buildchain::{Downloader, Manifest};
use system76_firmware::{config, download};

fn inner() -> Result<(), String> {
    let dl = Downloader::new(
        config::KEY,
        config::URL,
        config::PROJECT,
        config::BRANCH,
        Some(config::CERT)
    )?;

    eprintln!("downloading tail");
    let tail = dl.tail()?;

    eprintln!("opening download cache");
    let cache = download::Cache::new(config::CACHE, Some(dl))?;

    eprintln!("downloading manifest.json");
    let manifest_json = cache.object(&tail.digest)?;
    let manifest = serde_json::from_slice::<Manifest>(&manifest_json).map_err(|e| e.to_string())?;
    println!("{:?}", manifest);

    Ok(())
}

fn main() {
    inner().unwrap();
}
