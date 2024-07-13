
use std::{fs::OpenOptions, io::{BufReader, BufWriter, Read}, path::Path};
use crc::{Algorithm, Crc};
use log::info;
use crate::models::Diff;
use anyhow::Result;

// Reads the contents of a file as a Vec.
fn read_to_vec(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    let mut reader = BufReader::new(std::fs::File::open(path)?);
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;

    Ok(buf)
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
    let from_crc = mbf_zip::crc_bytes(&from_bytes);
    let to_crc = mbf_zip::crc_bytes(&to_bytes);

    info!("Generating diff (this may take several minutes)");
    let mut output = BufWriter::new(OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&output_path)?);

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