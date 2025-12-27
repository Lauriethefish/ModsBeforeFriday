use anyhow::{anyhow, Context, Result};
use byteorder::{ReadBytesExt, WriteBytesExt, LE};
use crc::{Algorithm, Crc};
use libflate::deflate;
use rasn_pkix::Certificate;
use rsa::RsaPrivateKey;
use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Write},
    path::Path,
};

use self::data::{CentDirHeader, EndOfCentDir, LocalFileHeader};

mod data;
pub mod signing;

/// Minimum version needed to extract ZIP files made by this module
pub const VERSION_NEEDED_TO_EXTRACT: u16 = 0x0002;
// Max size of the comment
pub const UINT16_MAX_VALUE: u16 = 0xffff;

/// The CRC-32 algorithm used by the ZIP file format.
pub const ZIP_CRC: Crc<u32> = Crc::<u32>::new(&Algorithm {
    width: 32,
    poly: 0x04c11db7,
    init: 0xffffffff,
    refin: true,
    refout: true,
    xorout: 0xffffffff,
    check: 0xcbf43926,
    residue: 0xdebb20e3,
});

/// Calculates the (ZIP) CRC-32 hash of the data within the given stream.
/// Will continue reading until the end of the stream.
pub fn crc_of_stream(mut stream: impl Read) -> Result<u32> {
    let mut crc = ZIP_CRC.digest();
    let mut buffer = vec![0u8; 4096];

    loop {
        let read_bytes = stream.read(&mut buffer)?;
        if read_bytes == 0 {
            break Ok(crc.finalize());
        }

        crc.update(&buffer[0..read_bytes])
    }
}

/// Calculates the CRC-32 hash of a slice. (using the same CRC algorithm as in ZIP files)
pub fn crc_bytes(bytes: &[u8]) -> u32 {
    let mut digest = ZIP_CRC.digest();
    digest.update(bytes);
    digest.finalize()
}

// The compression method of a file within the archive, which may be an unsupported method.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum FileCompression {
    Deflate,
    Store,
    Unsupported(u16),
}

pub struct ZipFile<T: Read + Seek> {
    file: T,
    entries: HashMap<String, CentDirHeader>,
    end_of_entries_offset: u32,
    // Alignment of entries created with the STORE compression method
    // Alignment is preferred for non-compressed files in APKs so that they can be MMAP'd directly into
    // memory, improving performance.
    // Typically, already-compressed media files like PNG use the STORE compression method.
    store_aligment: u16,
}

impl<T: Read + Seek> ZipFile<T> {
    /// Opens a ZIP archive from a readable stream.
    pub fn open(mut file: T) -> Result<Self> {
        let mut buf_file = BufReader::new(&mut file);

        let mut found_eocd_pos = None;
        {
            let archive_size = buf_file.seek(SeekFrom::End(0))?;
            if archive_size < EndOfCentDir::MIN_SIZE as u64 {
                return Err(anyhow!("File too small to be a valid ZIP archive"));
            }

            let max_comment_len = std::cmp::min(
                archive_size - EndOfCentDir::MIN_SIZE as u64,
                UINT16_MAX_VALUE as u64,
            );

            let eocd_empty_comment_pos = archive_size - EndOfCentDir::MIN_SIZE as u64;
            for expected_comment_len in 0..=max_comment_len {
                let eocd_pos = eocd_empty_comment_pos - expected_comment_len;
                buf_file.seek(SeekFrom::Start(eocd_pos))?;

                if buf_file.read_u32::<LE>()? == EndOfCentDir::HEADER {
                    buf_file.seek(SeekFrom::Start(
                        eocd_pos + EndOfCentDir::COMMENT_LENGTH_FIELD_OFFSET as u64,
                    ))?;

                    let actual_comment_len = buf_file.read_u16::<LE>()? as u64;

                    if actual_comment_len == expected_comment_len {
                        found_eocd_pos = Some(eocd_pos);
                        break;
                    }
                }
            }
        }
        let eocd_pos = found_eocd_pos.ok_or_else(|| anyhow!("No EOCD found in APK"))?;
        buf_file.seek(SeekFrom::Start(eocd_pos))?;

        let eocd: EndOfCentDir = EndOfCentDir::read(&mut buf_file).context("Invalid EOCD")?;
        buf_file.seek(SeekFrom::Start(eocd.cent_dir_offset as u64))?;

        // Read the central directory file headers
        let mut entries = HashMap::new();
        let mut last_lfh_offset = 0;

        for _ in 0..eocd.cent_dir_records {
            let cd_record = CentDirHeader::read(&mut buf_file).context("Invalid CD file header")?;
            last_lfh_offset = last_lfh_offset.max(cd_record.local_header_offset);

            entries.insert(cd_record.file_name.clone(), cd_record);
        }

        // Read the last LFH to figure out the location of the first byte after the last entry.
        // We could just use the central directory offset here, however this will leave the original signature intact,
        // ... which may not cause any problems but is a waste of space.
        buf_file.seek(SeekFrom::Start(last_lfh_offset as u64))?;
        let last_header = LocalFileHeader::read(&mut buf_file)?;

        Ok(Self {
            end_of_entries_offset: (buf_file.stream_position()?
                + last_header.compressed_len as u64)
                .try_into()
                .context("ZIP file too large")?,
            file,
            entries,
            store_aligment: 1,
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
            .context("Creating extracted file")?;

        self.read_file_contents(name, &mut handle)?;
        Ok(())
    }

    /// Extracts all of the files in the ZIP file to the given directory.
    pub fn extract_to_directory(&mut self, to: impl AsRef<Path>) -> Result<()> {
        let to = to.as_ref();

        // Create a clone of the entry names as a workaround since we need a mutable reference to self in order to extract files
        // TODO: This will use additional memory although the amount of memory used is not likely to be significant
        let entries = self
            .entries
            .iter()
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        for entry_name in entries.iter() {
            let extract_path = to.join(entry_name);
            if let Some(parent) = extract_path.parent() {
                std::fs::create_dir_all(parent)
                    .context("Creating directory to extract ZIP file")?;
            }

            let mut handle = std::fs::OpenOptions::new()
                .truncate(true)
                .create(true)
                .write(true)
                .open(extract_path)
                .context("Creating extracted file")?;

            self.read_file_contents(entry_name, &mut handle)
                .context("Reading file contents into extracted file")?;
        }

        Ok(())
    }

    // Checks if an entry with name `name` exists, and if it does:
    // - Creates a buffered reader to read the local file header.
    // - Reads the local file header of the entry
    // - Leaves the `reader` at the first byte of the entry content.
    // If the entry does not exist, this gives an appropriate error.
    fn read_lfh_and_seek_to_contents(
        &mut self,
        name: &str,
    ) -> Result<(LocalFileHeader, &CentDirHeader, BufReader<&mut T>)> {
        let cd_header = match self.entries.get(name) {
            Some(header) => header,
            None => return Err(anyhow!("File with name {name} did not exist")),
        };

        let mut buf_reader = BufReader::new(&mut self.file);
        buf_reader.seek(SeekFrom::Start(cd_header.local_header_offset as u64))?;
        let lfh = LocalFileHeader::read(&mut buf_reader).context("Invalid local file header")?;

        // TODO: Verify CRC32, file name, and other attributes match?
        Ok((lfh, cd_header, buf_reader))
    }

    /// Reads the contents of entry with full name `name` and writes them to `write_to`.
    /// Gives an Err if the file does not exist (or the ZIP file header is corrupt)
    pub fn read_file_contents(&mut self, name: &str, write_to: &mut impl Write) -> Result<()> {
        let (lfh, cdh, mut buf_reader) = self.read_lfh_and_seek_to_contents(name)?;

        // Use CDH for compressed length as LFH may have it set to 0 if this archive uses data descriptors.
        let mut compressed_contents = (&mut buf_reader).take(cdh.compressed_len as u64);

        match lfh.compression_method {
            FileCompression::Deflate => {
                // Limit the bytes to be decompressed
                let mut decoder = deflate::Decoder::new(compressed_contents);

                std::io::copy(&mut decoder, write_to)?;
            }
            FileCompression::Store => {
                std::io::copy(&mut compressed_contents, write_to)?;
            }
            FileCompression::Unsupported(method) => {
                return Err(anyhow!(
                    "Compression method `{method}` not supported for reading"
                ))
            }
        };

        Ok(())
    }

    /// Copies all entries in this ZIP file into `dst_archive`. For each entry, the path is the same in both archives.
    /// Any files that already exist in `dst_archive` will be overwritten.
    pub fn copy_all_entries_to(&mut self, dst_archive: &mut ZipFile<File>) -> Result<()> {
        let mut buf_reader = BufReader::new(&mut self.file);

        for (src_name, cd_header) in &self.entries {
            buf_reader.seek(SeekFrom::Start(cd_header.local_header_offset as u64))?;

            let lfh =
                LocalFileHeader::read(&mut buf_reader).context("Invalid local file header")?;

            Self::copy_entry_internal(
                lfh,
                &cd_header,
                &mut buf_reader,
                src_name.clone(),
                dst_archive,
            )?;
        }

        Ok(())
    }

    /// Copies the entry in this ZIP file with name `src_name` to `dst_archive` with name `dst_name`.
    /// If the entry already exists, it will be overwritten.
    pub fn copy_entry(
        &mut self,
        src_name: &str,
        dst_archive: &mut ZipFile<File>,
        dst_name: String,
    ) -> Result<()> {
        let (lfh, cd_header, mut buf_reader) = self.read_lfh_and_seek_to_contents(src_name)?;

        Self::copy_entry_internal(lfh, cd_header, &mut buf_reader, dst_name, dst_archive)
    }

    fn copy_entry_internal(
        mut lfh: LocalFileHeader,
        src_cdh: &CentDirHeader,
        buf_reader: &mut BufReader<&mut T>,
        dst_name: String,
        dst_archive: &mut ZipFile<File>,
    ) -> Result<()> {
        // Update the new position of the LFH in the CDH
        let mut dst_cdh = src_cdh.clone();
        dst_cdh.local_header_offset = dst_archive.end_of_entries_offset;
        dst_cdh.file_name = dst_name.clone();
        lfh.file_name = dst_name.clone();

        // Locate a position in the destination archive for the new local header.

        dst_archive
            .file
            .seek(SeekFrom::Start(dst_archive.end_of_entries_offset as u64))?;
        let mut buf_writer = BufWriter::new(&mut dst_archive.file);
        lfh.write(&mut buf_writer)
            .context("Writing local file header")?;
        let lfh_length = buf_writer.stream_position()? - dst_archive.end_of_entries_offset as u64;

        // Copy the contents of the entry to the other archive (no need to decompress and recompress)
        std::io::copy(
            &mut buf_reader.take(lfh.compressed_len as u64),
            &mut buf_writer,
        )
        .context("Copying content of entry")?;

        dst_archive.end_of_entries_offset =
            (lfh.compressed_len as u64 + lfh_length + dst_archive.end_of_entries_offset as u64)
                .try_into()
                .context("ZIP file too large")?;

        dst_archive.entries.insert(dst_name, dst_cdh);

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
    /// Sets the alignment for files written with the STORE compression method.
    pub fn set_store_alignment(&mut self, alignment: u16) {
        self.store_aligment = alignment;
    }

    // Creates a field used to align the ZIP entry data to store_alignment
    // `data_offset` is what the offset in the ZIP of the first byte of the data would be,
    // with no alignment field.
    fn create_alignment_field(&self, data_offset: u64) -> Result<Vec<u8>> {
        const ALIGNMENT_EXTRA_DATA_HEADER: u16 = 0xD935;

        let offset_from_alignment = data_offset % self.store_aligment as u64;
        // No need for an alignment field if we are already aligned.
        if offset_from_alignment == 0 {
            return Ok(Vec::new());
        }

        // The alignment field is at least 6 bytes long, before the padding null bytes that achieve
        // the desired alignment.
        // First there is an extra data ID and data length (2 bytes each), then a 2 byte unsigned integer
        // storing the level of aligment.
        let after_min_len = data_offset + 6;
        // Number of 0 bytes needed after the extra data field's header.
        let padding_bytes = (self.store_aligment as u64
            - (after_min_len % self.store_aligment as u64))
            % self.store_aligment as u64;

        let mut output_buf: Vec<u8> = Vec::new();
        let mut cursor = Cursor::new(&mut output_buf);

        // Write the extra data header and level of alignment.
        cursor.write_u16::<LE>(ALIGNMENT_EXTRA_DATA_HEADER)?;
        cursor.write_u16::<LE>((padding_bytes + 2) as u16)?;
        cursor.write_u16::<LE>(self.store_aligment as u16)?;
        // Actually write the padding which is contained within all these layers
        for _ in 0..padding_bytes {
            cursor.write_u8(0)?;
        }

        Ok(output_buf)
    }

    /// Writes a file to the ZIP with entry name `name` and contents copied from `contents` (which is read until EOF)
    pub fn write_file(
        &mut self,
        name: &str,
        contents: &mut (impl Read + Seek),
        compression_method: FileCompression,
    ) -> Result<()> {
        self.file
            .seek(SeekFrom::Start(self.end_of_entries_offset as u64))?;

        let lfh_offset = self.file.stream_position()?;
        // Find the offset of the first byte after the LFH ignoring alignment.
        let unaligned_post_lfh_offset = self.file.stream_position()? + 30 + name.len() as u64;
        let aligment_field = if compression_method == FileCompression::Store {
            self.create_alignment_field(unaligned_post_lfh_offset)?
        } else {
            // No need for alignment fields if using the DEFLATE compression method
            Vec::new()
        };

        // Skip the location of the new LFH for now, since we don't know the data size yet.
        self.file.seek(SeekFrom::Start(
            unaligned_post_lfh_offset + aligment_field.len() as u64,
        ))?;

        let data_start = self.file.stream_position()?;

        // TODO: Alignment for entries created with STORE compression method
        contents.seek(SeekFrom::Start(0))?;
        let crc32 = match compression_method {
            FileCompression::Deflate => {
                let mut buf_writer = BufWriter::new(&mut self.file);

                let mut encoder = deflate::Encoder::new(&mut buf_writer);
                let crc = copy_to_with_crc(contents, &mut encoder)
                    .context("Writing/compressing file data")?;
                encoder.finish().into_result()?;

                // Update the offset for the next file to be written
                self.end_of_entries_offset = buf_writer
                    .stream_position()?
                    .try_into()
                    .context("ZIP file too large")?;

                crc
            }
            FileCompression::Store => {
                let crc =
                    copy_to_with_crc(contents, &mut self.file).context("Writing file data")?;
                // Update the offset for the next file to be written
                self.end_of_entries_offset = self
                    .file
                    .stream_position()?
                    .try_into()
                    .context("ZIP file too large")?;

                crc
            }
            FileCompression::Unsupported(method) => {
                return Err(anyhow!("Compression method `{method}` is not supported"))
            }
        };

        let compressed_len: u32 = (self.end_of_entries_offset as u64 - data_start)
            .try_into()
            .context("Compressed file length too big for 32 bit ZIP file")?;
        let uncompressed_len: u32 = contents
            .stream_position()?
            .try_into()
            .context("Uncompressed file length too big for 32 bit ZIP file")?;

        let local_header = LocalFileHeader {
            version_needed: VERSION_NEEDED_TO_EXTRACT,
            flags: 0,
            compression_method,
            last_modified: 0, // TODO: write correct value
            crc32,
            compressed_len,
            uncompressed_len,
            file_name: name.to_string(),
            extra_field: aligment_field,
        };

        // Write the local header with the known length/CRC
        self.file.seek(SeekFrom::Start(lfh_offset))?;
        local_header
            .write(&mut BufWriter::new(&mut self.file))
            .context("Writing local file header")?;

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
            cent_dir_records: self
                .entries
                .len()
                .try_into()
                .context("Too many ZIP entries")?,
            cent_dir_size: cd_bytes
                .len()
                .try_into()
                .context("Central directory too big")?,
            cent_dir_offset: 0, // Can be set after we know the length of the signing block
            comment: Vec::new(),
        };

        // Remove existing CD and EOCD
        self.file.set_len(self.end_of_entries_offset as u64)?;

        // Add signature
        self.file
            .seek(SeekFrom::Start(self.end_of_entries_offset as u64))?;
        signing::write_v2_signature(&mut self.file, priv_key, cert, &cd_bytes, eocd.clone())
            .context("Signing APK")?;

        eocd.cent_dir_offset = self
            .file
            .stream_position()?
            .try_into()
            .context("APK file too big")?;
        self.file.write_all(&cd_bytes)?;
        eocd.write(&mut self.file)?;

        Ok(())
    }

    /// Saves the ZIP central directory.
    /// If this is not called, any newly written files or deleted files will not be respected in the final archive.
    /// The CD is NOT automatically saved on drop.
    /// Currently, this project doesn't save any ZIP files without signing them, but this is kept in-case this is needed in the future.
    #[allow(unused)]
    pub fn save(mut self) -> Result<()> {
        // Remove existing CD and EOCD
        self.file.set_len(self.end_of_entries_offset as u64)?;

        self.file
            .seek(SeekFrom::Start(self.end_of_entries_offset as u64))?;

        for cd_header in self.entries.values() {
            cd_header
                .write(&mut self.file)
                .context("Saving central directory header")?;
        }

        let eocd = EndOfCentDir {
            cent_dir_records: self
                .entries
                .len()
                .try_into()
                .context("Too many ZIP entries")?,
            cent_dir_size: (self.file.stream_position()? - self.end_of_entries_offset as u64)
                .try_into()
                .context("Central directory too big")?,
            cent_dir_offset: self.end_of_entries_offset,
            comment: Vec::new(),
        };

        eocd.write(&mut self.file)
            .context("Saving end of central directory")?;
        return Ok(());
    }
}
