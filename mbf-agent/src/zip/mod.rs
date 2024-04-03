use std::{collections::HashMap, fs::File, io::{Cursor, Read, Seek, SeekFrom, Write}, path::Path};
use byteorder::{ReadBytesExt, LE};
use anyhow::{Result, anyhow, Context};
use crc::{Crc, Algorithm};
use libflate::deflate;
use rasn_pkix::Certificate;
use rsa::RsaPrivateKey;

use self::data::{EndOfCentDir, CentDirHeader, LocalFileHeader};

mod data;
pub mod signing;

// Minimum version needed to extract ZIP files made by this module
const VERSION_NEEDED_TO_EXTRACT: u16 = 0x0002;

const ZIP_CRC: Crc<u32> =  Crc::<u32>::new(&Algorithm {
    width: 32,
    poly: 0x04c11db7,
    init: 0xffffffff,
    refin: true,
    refout: true,
    xorout: 0xffffffff,
    check: 0xcbf43926,
    residue: 0xdebb20e3,
});

// The compression method of a file within the archive, which may be an unsupported method.
#[derive(Copy, Clone)]
pub enum FileCompression {
    Deflate,
    Store,
    Unsupported(u16)
}

pub struct ZipFile<T: Read + Seek> {
    file: T,
    entries: HashMap<String, CentDirHeader>,
    end_of_entries_offset: u32,
}

impl<T: Read + Seek> ZipFile<T> {
    /// Opens a ZIP archive from a readable stream.
    pub fn open(mut file: T) -> Result<Self> {
        file.seek(SeekFrom::End(-22))?; // Assuming zero comment, this is the latest position for the EOCD header

        // Read backwards until an EOCD header is found.
        while file.read_u32::<LE>()? != EndOfCentDir::HEADER {
            if file.stream_position()? == 4 {
                return Err(anyhow!("No EOCD found in APK"));
            }

            file.seek(SeekFrom::Current(-8))?;
        }
        file.seek(SeekFrom::Current(-4))?; // Seek back before the LFH

        let eocd: EndOfCentDir = EndOfCentDir::read(&mut file).context("Invalid EOCD")?;
        file.seek(SeekFrom::Start(eocd.cent_dir_offset as u64))?;

        // Read the central directory file headers
        let mut entries = HashMap::new();
        let mut last_lfh_offset = 0;

        for _ in 0..eocd.cent_dir_records {
            let cd_record = CentDirHeader::read(&mut file).context("Invalid CD file header")?;
            last_lfh_offset = last_lfh_offset.max(cd_record.local_header_offset);

            entries.insert(cd_record.file_name.clone(), cd_record);
        }
        
        // Read the last LFH to figure out the location of the first byte after the last entry.
        // We could just use the central directory offset here, however this will leave the original signature intact,
        // ... which may not cause any problems but is a waste of space.
        file.seek(SeekFrom::Start(last_lfh_offset as u64))?;
        let last_header = LocalFileHeader::read(&mut file)?;

        Ok(Self {
            end_of_entries_offset: (file.stream_position()? + last_header.compressed_len as u64).try_into().context("ZIP file too large")?,
            file,
            entries
        })
    }

    /// Reads the contents of the file with the given name from the ZIP.
    pub fn read_file(&mut self, name: &str) -> Result<Vec<u8>> {
        let mut cursor = Cursor::new(vec![]);

        self.read_file_contents(name, &mut cursor)?;
        Ok(cursor.into_inner())
    }

    /// Extracts a file from the ZIP to a particular path.
    pub fn extract_file_to(&mut self, name: &str, to: impl AsRef<Path>) -> Result<()> {
        let mut handle = std::fs::OpenOptions::new()
            .truncate(true)
            .create(true)
            .write(true)
            .open(to)
            .context("Failed to create extracted file at")?;

        self.read_file_contents(name, &mut handle)?;
        Ok(())
    }

    pub fn read_file_contents(&mut self, name: &str, write_to: &mut impl Write) -> Result<()> {
        let cd_header = match self.entries.get(name) {
            Some(header) => header,
            None => return Err(anyhow!("File with name {name} did not exist"))
        };

        self.file.seek(SeekFrom::Start(cd_header.local_header_offset as u64))?;
        let _ = LocalFileHeader::read(&mut self.file).context("Invalid local file header")?;
        // TODO: Verify CRC32, file name, and other attributes match?

        let mut compressed_contents = (&mut self.file)
            .take(cd_header.compressed_len as u64);
        match cd_header.compression_method {
            FileCompression::Deflate => {
                // Limit the bytes to be decompressed
                let mut decoder = deflate::Decoder::new(compressed_contents);

                std::io::copy(&mut decoder, write_to)?;
            },
            FileCompression::Store => {
                std::io::copy(&mut compressed_contents, write_to)?;
            },
            FileCompression::Unsupported(method) => return Err(anyhow!("Compression method `{method}` not supported for reading"))
        };

        Ok(())
    }

    /// Returns an iterator over the entries within the ZIP file.
    pub fn iter_entry_names(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_ref)
    }

    /// Returns true if and only if a file exists with name `name`
    pub fn contains_file(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }
}

// Copies the contents of `from` to `to`, calculating the ZIP CRC-32 of the copied data.
fn copy_to_with_crc(from: &mut impl Read, to: &mut impl Write) -> Result<u32> {
    const BUFFER_SIZE: usize = 4096;
    let mut buffer = vec![0; BUFFER_SIZE];

    let mut crc = ZIP_CRC.digest();
    loop {
        let bytes_read = from.read(&mut buffer)?;
        if bytes_read == 0 {
            break Ok(crc.finalize());
        }

        crc.update(&buffer[0..bytes_read]);
        to.write_all(&buffer[0..bytes_read])?;
    }
}

impl ZipFile<File> {
    pub fn write_file(&mut self,
        name: &str,
        contents: &mut (impl Read + Seek),
        compression_method: FileCompression) -> Result<()> {
        self.file.seek(SeekFrom::Start(self.end_of_entries_offset as u64))?;

        let lfh_offset = self.file.stream_position()?;
        self.file.seek(SeekFrom::Current(30 + name.len() as i64))?; // Skip the location of the new LFH for now, since we don't know the data size yet.
        
        let data_start = self.file.stream_position()?;

        // TODO: Alignment for entries created with STORE compression method
        contents.seek(SeekFrom::Start(0))?;
        let crc32 = match compression_method {
            FileCompression::Deflate => {
                let mut encoder = deflate::Encoder::new(&mut self.file);
                let crc = copy_to_with_crc(contents, &mut encoder).context("Failed to write/compress file data")?;
                encoder.finish().into_result()?;

                crc
            },
            FileCompression::Store => copy_to_with_crc(contents, &mut self.file)
                .context("Failed to write file data")?,
            FileCompression::Unsupported(method) => return Err(anyhow!("Compression method `{method}` is not supported"))
        };

        // Update the offset for the next file to be written
        self.end_of_entries_offset = self.file.stream_position()?.try_into().context("ZIP file too large")?;

        let compressed_len: u32 = (self.file.stream_position()? - data_start).try_into().context("Compressed file length too big for 32 bit ZIP file")?;
        let uncompressed_len: u32 = contents.stream_position()?.try_into().context("Uncompressed file length too big for 32 bit ZIP file")?;

        let local_header = LocalFileHeader {
            version_needed: VERSION_NEEDED_TO_EXTRACT,
            flags: 0,
            compression_method,
            last_modified: 0, // TODO: write correct value
            crc32,
            compressed_len,
            uncompressed_len,
            file_name: name.to_string(),
            extra_field: Vec::new(),
        };

        // Write the local header with the known length/CRC
        self.file.seek(SeekFrom::Start(lfh_offset))?;
        local_header.write(&mut self.file).context("Failed to write local file header")?;


        let central_dir_header = CentDirHeader {
            os_version_made_by: 0, // 0 seems to be accepted as a valid OS, TODO: give actual value?
            version_needed: VERSION_NEEDED_TO_EXTRACT,
            flags: 0,
            compression_method,
            last_modified: 0, // TODO: write correct value
            crc32,
            compressed_len,
            uncompressed_len,
            file_name: name.to_string(),
            extra_field: Vec::new(),
            internal_attrs: 0,
            external_attrs: 0,
            local_header_offset: lfh_offset.try_into().context("ZIP file too big")?,
            comment: String::new(),
        };

        // Insert/replace the central directory header. (replacing the header will delete an existing file with the same name)
        self.entries.insert(name.to_string(), central_dir_header);
        Ok(())
    }

    // Deletes the file with the given name from the ZIP, if it existed.
    pub fn delete_file(&mut self, name: &str) -> bool {
        self.entries.remove(name).is_some()
    }

    /// Saves the ZIP central directory, while signing the APK with the V2 signature scheme.
    pub fn save_and_sign_v2(&mut self, priv_key: &RsaPrivateKey, cert: &Certificate) -> Result<()> {
        let mut cd_bytes = Vec::new();
        let mut cd_cursor = Cursor::new(&mut cd_bytes);

        for cd_header in self.entries.values() {
            cd_header.write(&mut cd_cursor)?;
        }

        let mut eocd = EndOfCentDir {
            cent_dir_records: self.entries.len().try_into().context("Too many ZIP entries")?,
            cent_dir_size: cd_bytes.len().try_into().context("Central directory too big")?,
            cent_dir_offset: 0, // Can be set after we know the length of the signing block
            comment: Vec::new(),
        };

        // Remove existing CD and EOCD
        self.file.set_len(self.end_of_entries_offset as u64)?;

        // Add signature
        self.file.seek(SeekFrom::Start(self.end_of_entries_offset as u64))?;
        signing::write_v2_signature(&mut self.file, priv_key, cert, &cd_bytes, eocd.clone())
            .context("Failed to sign APK")?;

        eocd.cent_dir_offset = self.file.stream_position()?.try_into().context("APK file too big")?;
        self.file.write_all(&cd_bytes)?;
        eocd.write(&mut self.file)?;

        Ok(())
    }

    // Saves the ZIP central directory.
    // If this is not called, any newly written files or deleted files will not be respected in the final archive.
    /// The CD is NOT automatically saved on drop.
    pub fn save(mut self) -> Result<()> {
        // Remove existing CD and EOCD
        self.file.set_len(self.end_of_entries_offset as u64)?;

        self.file.seek(SeekFrom::Start(self.end_of_entries_offset as u64))?;

        for cd_header in self.entries.values() {
            cd_header.write(&mut self.file).context("Failed to save central directory header")?;
        }

        let eocd = EndOfCentDir {
            cent_dir_records: self.entries.len().try_into().context("Too many ZIP entries")?,
            cent_dir_size: (self.file.stream_position()? - self.end_of_entries_offset as u64).try_into().context("Central directory too big")?,
            cent_dir_offset: self.end_of_entries_offset,
            comment: Vec::new(),
        };

        eocd.write(&mut self.file).context("Failed to save end of central directory")?;
        return Ok(())
    }
}