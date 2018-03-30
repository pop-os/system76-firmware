use lzma::reader::LzmaReader;
use std::{fs, io, path};
use std::io::Read;
use sha2::{Sha256, Digest};
use tar::Archive;

pub fn extract<P: AsRef<path::Path>>(data: &[u8], p: P) -> io::Result<()> {
    let decompressor = LzmaReader::new_decompressor(data).map_err(|err| io::Error::new(
        io::ErrorKind::Other,
        err
    ))?;
    let mut tar = Archive::new(decompressor);

    for file_res in tar.entries()?{
        let mut file = file_res?;

        // Inspect metadata about the file
        println!("{:?}", file.path());
        if ! file.unpack_in(&p)? {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid file path {:?}", file.path())
            ));
        }
    }

    Ok(())
}

pub fn read_string<P: AsRef<path::Path>>(p: P) -> io::Result<String> {
    let mut string = String::new();
    {
        let mut file = fs::File::open(p)?;
        file.read_to_string(&mut string)?;
    }
    Ok(string)
}

pub fn sha256(input: &[u8]) -> String {
    format!("{:x}", Sha256::digest(input))
}
