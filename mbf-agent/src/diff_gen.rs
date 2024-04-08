#![allow(unused)]

use std::{fs::OpenOptions, io::{BufReader, Read}, path::Path};
use anyhow::{Result, anyhow};
use external_res::{Diff, VersionDiffs};
use zip::ZIP_CRC;

mod external_res;
mod zip;

fn read_to_vec(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    let mut reader = BufReader::new(std::fs::File::open(path)?);
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;

    Ok(buf)
}

fn crc_bytes(bytes: &[u8]) -> u32 {
    let mut digest = ZIP_CRC.digest();
    digest.update(bytes);
    digest.finalize()
}

fn get_file_name(path: impl AsRef<Path>) -> String {
    path.as_ref()
        .file_name()
        .expect("Invalid file path")
        .to_string_lossy()
        .to_string()
}

fn generate_diff(
    from_file: impl AsRef<Path>,
    to_file: impl AsRef<Path>,
    output_path: impl AsRef<Path>) -> Result<Diff> {
    let from_bytes = read_to_vec(&from_file)?;
    let to_bytes = read_to_vec(&to_file)?;

    println!("Generating checksums");
    let from_crc = crc_bytes(&from_bytes);
    let to_crc = crc_bytes(&to_bytes);


    println!("Generating diff");
    let mut output = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&output_path)?;

    let _diff_bytes = qbsdiff::Bsdiff::new(&from_bytes, &to_bytes)
        .compression_level(9)
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

fn main() -> Result<()> {
    let mut args = std::env::args();
    args.next().unwrap();
    println!("Usage: <from_version> <to_version> <file_a_from> <file_a_to> <file_a_output> <file_b_from> <file_b_to> <file_b_output> etc...");

    let from_version = args.next().unwrap();
    let to_version = args.next().unwrap();

    let mut obb_diffs = Vec::new();
    let mut apk_diff: Option<Diff> = None;
    while let Some(from) = args.next() {
        let to = args.next().unwrap();
        let output = args.next().unwrap();

        println!("Generating diff from {from} to {to}");
        let diff = generate_diff(&from, &to, output)?;

        if to.to_ascii_lowercase().ends_with(".apk") {
            apk_diff = Some(diff);
        }   else {
            obb_diffs.push(diff);
        }
    }

    if let Some(apk_diff) = apk_diff {
        let output = serde_json::to_string_pretty(&VersionDiffs {
            from_version,
            to_version,
            apk_diff,
            obb_diffs
        })?;

        println!("{output}");
        Ok(())
    }   else {
        Err(anyhow!("Missing diff for the APK file"))
    }
}