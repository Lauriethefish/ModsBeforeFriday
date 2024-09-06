//! Basic APK signing implementation, which supports the V2 signature scheme as described here: 
//! https://source.android.com/docs/security/features/apksigning/v2
//! 
//! V1 signatures are not supported, so this module cannot be used for APKs that will be installed on any Android version before 7.0.

use std::{io::{Seek, Read, Write, SeekFrom, Cursor}, fs::File};
use byteorder::{LE, WriteBytesExt, ByteOrder};
use rasn_pkix::Certificate;
use rsa::{sha2::{Sha256, Digest}, RsaPrivateKey, pkcs1::DecodeRsaPrivateKey, Pkcs1v15Sign};
use anyhow::{Result, Context};

use super::data::EndOfCentDir;

/// Writes the v2 signature block to the APK.
/// The `apk` stream should be seeked to the first byte after the contents of the last ZIP entry.
pub(super) fn write_v2_signature(apk: &mut File, priv_key: &RsaPrivateKey, cert: &Certificate, central_dir_bytes: &[u8], mut eocd: EndOfCentDir) -> Result<()> {
    let after_entries_offset = apk.stream_position()?;

    // For the purpose of signing, the EOCD must set the central directory offset to point to the position of the signature.
    eocd.cent_dir_offset = after_entries_offset
        .try_into().context("ZIP file too large (to sign)")?;

    let mut eocd_bytes = Vec::new();
    eocd.write(&mut Cursor::new(&mut eocd_bytes))?;

    let apk_digest = calculate_apk_digest(apk, after_entries_offset, central_dir_bytes, &eocd_bytes)?;
    write_signature_block(apk, &apk_digest, cert, priv_key)?;
    Ok(())
}

/// Loads an X509 certificate and RSA private key from the given PEM data.
/// Panics in the case of invalid PEM or an invalid key/cert, so this should be used on certificates that are known to be valid.
/// (i.e. the debug certificate included with the agent)
pub fn load_cert_and_priv_key(pem_data: &[u8]) -> (Certificate, RsaPrivateKey) {
    let pem = pem::parse_many(pem_data).expect("Invalid PEM");

    let mut cert = None;
    let mut priv_key = None;

    for pem_sect in pem.iter() {
        if pem_sect.tag() == "RSA PRIVATE KEY" {
            priv_key = Some(RsaPrivateKey::from_pkcs1_der(pem_sect.contents())
                .expect("Invalid private key"));
        }

        if pem_sect.tag() == "CERTIFICATE" {
            cert = Some(rasn::der::decode::<Certificate>(pem_sect.contents())
                .expect("Invalid certificate"));
        }
    }

    return (cert.expect("No certificate"), priv_key.expect("No private key"))
}

const CHUNK_SIZE: u64 = 0x100000;
const APK_SIG_BLOCK_FOOTER: [u8; 16] = *b"APK Sig Block 42";
const RSA_PKCS1_15_SHA256: u32 = 0x0103;
const V2_SIGNATURE_ID: u32 = 0x7109871a;

// Calculates the digest of contiguous data in a stream, using the chunked method described in the V2 signing documentation.
// `chunk_buffer.len()` should match `CHUNK_SIZE`
fn calculate_chunked_digest(offset: u64,
    length: u64,
    source: &mut (impl Read + Seek),
    output: &mut impl Write,
    chunk_buffer: &mut [u8]) -> Result<u32> {

    let section_end = offset + length;

    source.seek(SeekFrom::Start(offset))?;
    let mut pos = offset;
    let mut chunk_count = 0;
    while pos < section_end {
        let bytes_in_chunk = CHUNK_SIZE.min(section_end - pos) as u32;

        let mut sha = Sha256::default();
        sha.update(&[0xa5u8]); // Magic value for chunk

        // Chunk size, which may be less than CHUNK_SIZE for the final chunk
        let mut buf = [0u8; 4];
        LE::write_u32(&mut buf, bytes_in_chunk);
        sha.update(&buf);

        let this_chunk_buf = &mut chunk_buffer[0..(bytes_in_chunk as usize)];
        source.read_exact(this_chunk_buf)?;       
        sha.update(this_chunk_buf);
        let hash = sha.finalize();

        output.write_all(&hash)?;
        pos += CHUNK_SIZE;
        chunk_count += 1;
    }

    Ok(chunk_count)
}

// Calculates the digest of an APK, based on the chunked contents of the CD, EOCD and file headers/entries.
fn calculate_apk_digest(apk: &mut File, entries_data_length: u64, central_dir: &[u8], eocd: &[u8]) -> Result<Vec<u8>> {
    let mut digests: Vec<u8> = Vec::new();
    let mut digests_stream = Cursor::new(&mut digests);
    digests_stream.write_u8(0x5a)?; // Magic value for the APK digest
    digests_stream.write_u32::<LE>(0)?; // Chunk count, not yet known

    // TODO: Buffer unnecessarily large.
    let mut chunk_buffer = vec![0u8; CHUNK_SIZE as usize];

    let mut chunk_count = 0;
    let mut cd_stream = Cursor::new(central_dir);
    let mut eocd_stream = Cursor::new(eocd);

    // Add the digests of each chunk, keeping track of the overall chunk count
    chunk_count += calculate_chunked_digest(0, entries_data_length, apk, &mut digests_stream, &mut chunk_buffer)?;
    chunk_count += calculate_chunked_digest(0, central_dir.len() as u64, &mut cd_stream, &mut digests_stream, &mut chunk_buffer)?;
    chunk_count += calculate_chunked_digest(0, eocd.len() as u64, &mut eocd_stream, &mut digests_stream, &mut chunk_buffer)?;

    // Overwrite the chunk count now that we know the correct value
    digests_stream.seek(SeekFrom::Start(1))?;
    digests_stream.write_u32::<LE>(chunk_count)?;

    let mut top_level_sha = Sha256::default();
    top_level_sha.update(digests);

    Ok(top_level_sha.finalize().to_vec())
}

fn write_signature_block(apk: &mut File, apk_digest: &[u8], cert: &Certificate, priv_key: &RsaPrivateKey) -> Result<()> {
    // Generate the block of data that we will be signing, which includes the APK digest and certificate
    let signed_data = generate_signed_data(apk_digest, cert)?;

    let mut signed_data_digest = Sha256::new();
    signed_data_digest.update(&signed_data);

    let signature = priv_key.sign(Pkcs1v15Sign::new::<Sha256>(), &signed_data_digest.finalize())
        .context("Signing data")?;

    let public_key_info = rasn::der::encode(&cert.tbs_certificate.subject_public_key_info)
        .expect("Failed to encode public key");

    // Calculate the total length of the signer section
    let signer_len = 4 + signed_data.len() + 4 + 4 + 4 + 4 + signature.len() + 4 + public_key_info.len();

    // Calculate the total length of the signing block
    let v2_signature_value_len = signer_len + 4 + 4;
    let v2_signature_pair_len = 4 + v2_signature_value_len;
    let signing_block_len = 8 + v2_signature_pair_len + 8 + APK_SIG_BLOCK_FOOTER.len();

    // Begin the APK signing block
    apk.write_u64::<LE>(signing_block_len as u64)?;
    apk.write_u64::<LE>(v2_signature_pair_len as u64)?;
    apk.write_u32::<LE>(V2_SIGNATURE_ID)?;

    // Write the signer, containing within it the signed data
    apk.write_u32::<LE>((4 + signer_len) as u32)?; // Length of signers array
    apk.write_u32::<LE>(signer_len as u32)?; // Length of first and only signer

    apk.write_u32::<LE>(signed_data.len() as u32)?;
    apk.write_all(&signed_data)?;

    apk.write_u32::<LE>((4 + 4 + 4 + signature.len()) as u32)?; // Length of the signatures
    apk.write_u32::<LE>((4 + 4 + signature.len()) as u32)?; // Length of our one signature
    apk.write_u32::<LE>(RSA_PKCS1_15_SHA256)?;
    apk.write_u32::<LE>(signature.len() as u32)?;
    apk.write_all(&signature)?;

    apk.write_u32::<LE>(public_key_info.len() as u32)?;
    apk.write_all(&public_key_info)?;

    // Write the APK signing block footer
    apk.write_u64::<LE>(signing_block_len as u64)?;
    apk.write_all(&APK_SIG_BLOCK_FOOTER)?;
    Ok(())
}

fn generate_signed_data(apk_digest: &[u8], cert: &Certificate) -> Result<Vec<u8>> {
    let mut signed_data: Vec<u8> = Vec::new();
    let mut signed_data_stream = Cursor::new(&mut signed_data);

    let digest_length = 4 + 4 + 32; // 4 bytes for signature alg. ID, 4 bytes for digest length, 32 bytes for digest
    let digest_seq_length = digest_length + 4; // 4 extra bytes for the digest length
    signed_data_stream.write_u32::<LE>(digest_seq_length)?; // Write the length of the digests array, which includes the length written just below
    signed_data_stream.write_u32::<LE>(digest_length)?; // Write the length of our one digest
    signed_data_stream.write_u32::<LE>(RSA_PKCS1_15_SHA256)?;
    signed_data_stream.write_u32::<LE>(apk_digest.len() as u32)?;
    signed_data_stream.write_all(apk_digest)?;

    let cert_data = rasn::der::encode(cert)
        .expect("Failed to encode certificate");

    signed_data_stream.write_u32::<LE>((cert_data.len() + 4) as u32)?; // Length of certificates array
    signed_data_stream.write_u32::<LE>(cert_data.len() as u32)?; // Length of our one certificate
    signed_data_stream.write_all(&cert_data)?;

    signed_data_stream.write_u32::<LE>(0)?; // No additional attributes
   
    Ok(signed_data)
}