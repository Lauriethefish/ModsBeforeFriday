use std::io::{Read, Write};

use anyhow::{anyhow, Context, Result};
use byteorder::{ReadBytesExt, WriteBytesExt, LE};

use super::FileCompression;

impl From<u16> for FileCompression {
    fn from(value: u16) -> Self {
        match value {
            0 => Self::Store,
            8 => Self::Deflate,
            other => Self::Unsupported(other),
        }
    }
}

impl Into<u16> for FileCompression {
    fn into(self) -> u16 {
        match self {
            Self::Store => 0,
            Self::Deflate => 8,
            Self::Unsupported(other) => other,
        }
    }
}

// ZIP end of central directory record
#[derive(Clone)]
pub struct EndOfCentDir {
    pub cent_dir_records: u16,
    pub cent_dir_size: u32,
    pub cent_dir_offset: u32,
    pub comment: Vec<u8>,
}

// ZIP central directory record
#[derive(Clone)]
pub struct CentDirHeader {
    pub os_version_made_by: u16,
    pub version_needed: u16,
    pub flags: u16,
    pub compression_method: FileCompression,
    pub last_modified: u32, // TODO: parse this
    pub crc32: u32,
    pub compressed_len: u32,
    pub uncompressed_len: u32,

    pub internal_attrs: u16,
    pub external_attrs: u32,
    pub local_header_offset: u32,

    pub file_name: String,
    pub extra_field: Vec<u8>,
    pub comment: String,
}

// ZIP local file header record
#[derive(Clone)]
pub struct LocalFileHeader {
    pub version_needed: u16,
    pub flags: u16,
    pub compression_method: FileCompression,
    pub last_modified: u32, // TODO: parse this
    pub crc32: u32,
    pub compressed_len: u32,
    pub uncompressed_len: u32,

    pub file_name: String,
    pub extra_field: Vec<u8>,
}

impl EndOfCentDir {
    pub const HEADER: u32 = 0x06054b50;
    pub const MIN_SIZE: i64 = 22;
    pub const COMMENT_LENGTH_FIELD_OFFSET: i64 = 20;

    pub fn read(data: &mut impl Read) -> Result<Self> {
        if data.read_u32::<LE>()? != Self::HEADER {
            return Err(anyhow!("Invalid EOCD header"));
        }

        let disk_num = data.read_u16::<LE>()?;
        let start_of_cd_disk = data.read_u16::<LE>()?;
        let cd_records_on_disk = data.read_u16::<LE>()?;

        let mut result = Self {
            cent_dir_records: data.read_u16::<LE>()?,
            cent_dir_size: data.read_u32::<LE>()?,
            cent_dir_offset: data.read_u32::<LE>()?,
            comment: vec![0u8; data.read_u16::<LE>()? as usize],
        };

        data.read_exact(&mut result.comment)?;

        if result.cent_dir_records != cd_records_on_disk || start_of_cd_disk != 0 || disk_num != 0 {
            return Err(anyhow!("Multi-disk archives are not supported"));
        }

        Ok(result)
    }

    pub fn write(&self, data: &mut impl Write) -> Result<()> {
        data.write_u32::<LE>(Self::HEADER)?;

        // Assuming a single-disk archive
        data.write_u16::<LE>(0)?;
        data.write_u16::<LE>(0)?;
        data.write_u16::<LE>(self.cent_dir_records)?;

        data.write_u16::<LE>(self.cent_dir_records)?;
        data.write_u32::<LE>(self.cent_dir_size)?;
        data.write_u32::<LE>(self.cent_dir_offset)?;
        data.write_u16::<LE>(
            self.comment
                .len()
                .try_into()
                .context("File comment longer than max length")?,
        )?;
        data.write_all(&self.comment)?;

        Ok(())
    }
}

impl CentDirHeader {
    pub const HEADER: u32 = 0x02014b50;

    pub fn read(data: &mut impl Read) -> Result<Self> {
        if data.read_u32::<LE>()? != Self::HEADER {
            return Err(anyhow!("Invalid CD header signature"));
        }

        let version_made_by = data.read_u16::<LE>()?;
        let version_needed = data.read_u16::<LE>()?;
        let flags = data.read_u16::<LE>()?;
        let compression_method = FileCompression::from(data.read_u16::<LE>()?);
        let last_modified = data.read_u32::<LE>()?;
        let crc32 = data.read_u32::<LE>()?;
        let compressed_len = data.read_u32::<LE>()?;
        let uncompressed_len = data.read_u32::<LE>()?;

        let mut file_name_buf = vec![0u8; data.read_u16::<LE>()? as usize];
        let mut extra_field_buf = vec![0u8; data.read_u16::<LE>()? as usize];
        let mut comment_buf = vec![0u8; data.read_u16::<LE>()? as usize];

        if data.read_u16::<LE>()? != 0 {
            return Err(anyhow!("Multi-disk archives are not supported"));
        }

        let internal_attrs = data.read_u16::<LE>()?;
        let external_attrs = data.read_u32::<LE>()?;
        let local_header_offset = data.read_u32::<LE>()?;

        data.read_exact(&mut file_name_buf)?;
        data.read_exact(&mut extra_field_buf)?;
        data.read_exact(&mut comment_buf)?;

        Ok(Self {
            os_version_made_by: version_made_by,
            version_needed,
            flags,
            compression_method,
            last_modified,
            crc32,
            compressed_len,
            uncompressed_len,
            internal_attrs,
            external_attrs,
            local_header_offset,

            // NB: Strictly speaking this should be converted to code page 437
            // ...but I am yet to find an APK with file names that aren't just UTF-8
            file_name: String::from_utf8(file_name_buf).context("File name was not valid UTF-8")?,
            extra_field: extra_field_buf,
            comment: String::from_utf8(comment_buf).context("File comment was not valid UTF-8")?,
        })
    }

    pub fn write(&self, data: &mut impl Write) -> Result<()> {
        data.write_u32::<LE>(Self::HEADER)?;
        data.write_u16::<LE>(self.os_version_made_by)?;
        data.write_u16::<LE>(self.version_needed)?;
        data.write_u16::<LE>(self.flags)?;
        data.write_u16::<LE>(self.compression_method.into())?;
        data.write_u32::<LE>(self.last_modified)?;
        data.write_u32::<LE>(self.crc32)?;
        data.write_u32::<LE>(self.compressed_len)?;
        data.write_u32::<LE>(self.uncompressed_len)?;

        data.write_u16::<LE>(
            self.file_name
                .len()
                .try_into()
                .context("File name longer than max length")?,
        )?;
        data.write_u16::<LE>(
            self.extra_field
                .len()
                .try_into()
                .context("Extra field longer than max length")?,
        )?;
        data.write_u16::<LE>(
            self.comment
                .len()
                .try_into()
                .context("Comment longer than max length")?,
        )?;

        data.write_u16::<LE>(0)?; // Disk number
        data.write_u16::<LE>(self.internal_attrs)?;
        data.write_u32::<LE>(self.external_attrs)?;
        data.write_u32::<LE>(self.local_header_offset)?;

        data.write_all(&self.file_name.as_bytes())?;
        data.write_all(&self.extra_field)?;
        data.write_all(&self.comment.as_bytes())?;

        Ok(())
    }
}

impl LocalFileHeader {
    const HEADER: u32 = 0x04034b50;

    pub fn read(data: &mut impl Read) -> Result<Self> {
        if data.read_u32::<LE>()? != Self::HEADER {
            return Err(anyhow!("Invalid LFH header signature"));
        }

        let version_needed = data.read_u16::<LE>()?;
        let flags = data.read_u16::<LE>()?;
        let compression_method = FileCompression::from(data.read_u16::<LE>()?);
        let last_modified = data.read_u32::<LE>()?;
        let crc32 = data.read_u32::<LE>()?;
        let compressed_len = data.read_u32::<LE>()?;
        let uncompressed_len = data.read_u32::<LE>()?;

        let mut file_name_buf = vec![0u8; data.read_u16::<LE>()? as usize];
        let mut extra_field_buf = vec![0u8; data.read_u16::<LE>()? as usize];

        data.read_exact(&mut file_name_buf)?;
        data.read_exact(&mut extra_field_buf)?;

        Ok(Self {
            version_needed,
            flags,
            compression_method,
            last_modified,
            crc32,
            compressed_len,
            uncompressed_len,
            file_name: String::from_utf8(file_name_buf).context("File name was not valid UTF-8")?,
            extra_field: extra_field_buf,
        })
    }

    pub fn write(&self, data: &mut impl Write) -> Result<()> {
        data.write_u32::<LE>(Self::HEADER)?;
        data.write_u16::<LE>(self.version_needed)?;
        data.write_u16::<LE>(self.flags)?;
        data.write_u16::<LE>(self.compression_method.into())?;
        data.write_u32::<LE>(self.last_modified)?;
        data.write_u32::<LE>(self.crc32)?;
        data.write_u32::<LE>(self.compressed_len)?;
        data.write_u32::<LE>(self.uncompressed_len)?;

        data.write_u16::<LE>(
            self.file_name
                .len()
                .try_into()
                .context("File name longer than max length")?,
        )?;
        data.write_u16::<LE>(
            self.extra_field
                .len()
                .try_into()
                .context("Extra field longer than max length")?,
        )?;

        data.write_all(&self.file_name.as_bytes())?;
        data.write_all(&self.extra_field)?;

        Ok(())
    }
}
