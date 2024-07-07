
use std::{fs::OpenOptions, io::{BufReader, Read}, path::Path};
use crc::{Algorithm, Crc};
use log::info;
use crate::models::Diff;
use anyhow::Result;

// The CRC-32 algorithm used by the diff index. (currently the same as the ZIP CRC32)
pub const ZIP_CRC: Crc<u32> =  Crc::<u32>::new(&Algorithm {
    width: 32,
    poly: 0x04c11db7,
    init: 0xffffffff,
    refin: true,
    refout: true,
    xorout: 0xffffffff,
    check: 0xcbf43926,
    residue: 0xdebb20e3,
});


// Reads the contents of a file as a Vec.
fn read_to_vec(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    let mut reader = BufReader::new(std::fs::File::open(path)?);
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;

    Ok(buf)
}

// Calculates the CRC-32 hash of a slice. (using the same CRC algorithm as in ZIP files)
fn crc_bytes(bytes: &[u8]) -> u32 {
    let mut digest = ZIP_CRC.digest();
    digest.update(bytes);
    digest.finalize()
}

// Gets the file name of the given path.
fn get_file_name(path: impl AsRef<Path>) -> String {
    path.as_ref()
        .file_name()
        .expect("Invalid file path")
        .to_string_lossy()
        .to_string()
}

// Generates a diff to patch `from_file` into `to_file``.
// Outputs the diff to `output_path`
pub fn generate_diff(
    from_file: impl AsRef<Path>,
    to_file: impl AsRef<Path>,
    output_path: impl AsRef<Path>) -> Result<Diff> {
    let from_bytes = read_to_vec(&from_file)?;
    let to_bytes = read_to_vec(&to_file)?;

    info!("Generating checksums");
    let from_crc = crc_bytes(&from_bytes);
    let to_crc = crc_bytes(&to_bytes);

    info!("Generating diff (this may take several minutes)");
    let mut output = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&output_path)?;

    let _diff_bytes = qbsdiff::Bsdiff::new(&from_bytes, &to_bytes)
        .compression_level(6)
        .compare(&mut output)?;

    Ok(Diff {
        diff_name: get_file_name(output_path),
        file_name: get_file_name(from_file),
        file_crc: from_crc,
        output_file_name: get_file_name(to_file),
        output_crc: to_crc,
        output_size: to_bytes.len(),
    })
}