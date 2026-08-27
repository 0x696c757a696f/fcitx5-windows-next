#![deny(unsafe_op_in_unsafe_fn)]

use std::error::Error;
use std::ffi::{c_int, c_void};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::ptr;
use std::sync::atomic::{compiler_fence, Ordering};

const MLDSA65_PUBLIC_KEY_BYTES: usize = 1952;
const MLDSA65_SECRET_KEY_BYTES: usize = 4032;
const MLDSA65_SIGNATURE_BYTES: usize = 3309;
const MAXIMUM_METADATA_BYTES: u64 = 1024 * 1024;
const MAXIMUM_PAYLOAD_BYTES: u64 = 64 * 1024 * 1024;
const BCRYPT_USE_SYSTEM_PREFERRED_RNG: u32 = 0x0000_0002;

#[link(name = "bcrypt")]
unsafe extern "system" {
    fn BCryptGenRandom(algorithm: *mut c_void, output: *mut u8, output_len: u32, flags: u32)
        -> i32;
}

unsafe extern "C" {
    fn fcitx5_mldsa65_release_sign_pk_from_sk(public_key: *mut u8, secret_key: *const u8) -> c_int;
    fn fcitx5_mldsa65_release_sign_signature(
        signature: *mut u8,
        message: *const u8,
        message_len: usize,
        context: *const u8,
        context_len: usize,
        secret_key: *const u8,
    ) -> c_int;
}

/// Supplies cryptographically secure random bytes to pinned `mldsa-native` C code.
///
/// # Safety
///
/// The C caller must provide a valid, writable `output` range of `length` bytes.
#[no_mangle]
pub unsafe extern "C" fn randombytes(output: *mut u8, length: usize) -> c_int {
    if output.is_null() && length != 0 {
        return -1;
    }
    // SAFETY: the native ML-DSA callback contract supplies a writable buffer of `length` bytes.
    let output = unsafe { std::slice::from_raw_parts_mut(output, length) };
    for chunk in output.chunks_mut(u32::MAX as usize) {
        // SAFETY: each chunk is a live, writable slice whose length fits the Win32 u32 contract.
        let status = unsafe {
            BCryptGenRandom(
                ptr::null_mut(),
                chunk.as_mut_ptr(),
                chunk.len() as u32,
                BCRYPT_USE_SYSTEM_PREFERRED_RNG,
            )
        };
        if status < 0 {
            return -1;
        }
    }
    0
}

struct SecretBytes(Vec<u8>);

impl Drop for SecretBytes {
    fn drop(&mut self) {
        zero(&mut self.0);
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("release_sign_failed: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [command, input] if command == "--blake3" => {
            println!(
                "{}",
                blake3::hash(&read_bounded(input, MAXIMUM_PAYLOAD_BYTES)?).to_hex()
            );
            Ok(())
        }
        [command, input, signature_path, secret_path, expected_public] if command == "--sign" => {
            let metadata = read_bounded(input, MAXIMUM_METADATA_BYTES)?;
            let secret = SecretBytes(read_bounded(secret_path, MLDSA65_SECRET_KEY_BYTES as u64)?);
            if secret.0.len() != MLDSA65_SECRET_KEY_BYTES {
                return Err("ML-DSA-65 secret key must be exactly 4032 bytes".into());
            }
            let expected_public = decode_base64(expected_public)?;
            if expected_public.len() != MLDSA65_PUBLIC_KEY_BYTES {
                return Err("expected ML-DSA-65 public key has an invalid length".into());
            }
            let mut derived_public = [0_u8; MLDSA65_PUBLIC_KEY_BYTES];
            // SAFETY: the pinned ML-DSA ABI writes exactly one public key into the
            // fixed-size destination and reads exactly one validated secret key.
            let public_status = unsafe {
                fcitx5_mldsa65_release_sign_pk_from_sk(
                    derived_public.as_mut_ptr(),
                    secret.0.as_ptr(),
                )
            };
            if public_status != 0 || derived_public.as_slice() != expected_public.as_slice() {
                zero(&mut derived_public);
                return Err("secret key does not match the official trusted public key".into());
            }
            let mut signature = [0_u8; MLDSA65_SIGNATURE_BYTES];
            // SAFETY: all pointers reference live buffers of the exact sizes required
            // by the pinned ML-DSA ABI; the empty context is represented by null/zero.
            let signature_status = unsafe {
                fcitx5_mldsa65_release_sign_signature(
                    signature.as_mut_ptr(),
                    metadata.as_ptr(),
                    metadata.len(),
                    ptr::null(),
                    0,
                    secret.0.as_ptr(),
                )
            };
            zero(&mut derived_public);
            if signature_status != 0 {
                zero(&mut signature);
                return Err("ML-DSA-65 signing failed".into());
            }
            fs::write(signature_path, signature)?;
            zero(&mut signature);
            println!("signature=mldsa65");
            Ok(())
        }
        _ => Err(usage().into()),
    }
}

fn usage() -> &'static str {
    "Usage:\n  fcitx5-release-pqc-signer --sign INPUT SIGNATURE_RAW SECRET_KEY EXPECTED_PUBLIC_KEY_BASE64\n  fcitx5-release-pqc-signer --blake3 INPUT"
}

fn read_bounded(path: impl AsRef<Path>, maximum: u64) -> Result<Vec<u8>, Box<dyn Error>> {
    let path = path.as_ref();
    let file = fs::File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > maximum {
        return Err("input is missing or too large".into());
    }
    let capacity = usize::try_from(metadata.len()).map_err(|_| "input is too large")?;
    let mut bytes = Vec::with_capacity(capacity);
    file.take(maximum.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if bytes.len() > maximum as usize {
        return Err("input is missing or too large".into());
    }
    Ok(bytes)
}

fn decode_base64(value: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    if value.is_empty() || !value.len().is_multiple_of(4) {
        return Err("expected public key is not base64".into());
    }
    let mut output = Vec::with_capacity(value.len() / 4 * 3);
    for (index, chunk) in value.as_bytes().chunks_exact(4).enumerate() {
        let last = index + 1 == value.len() / 4;
        let first = base64_value(chunk[0]).ok_or("expected public key is not base64")?;
        let second = base64_value(chunk[1]).ok_or("expected public key is not base64")?;
        let third = if chunk[2] == b'=' {
            if !last || chunk[3] != b'=' {
                return Err("expected public key is not base64".into());
            }
            None
        } else {
            Some(base64_value(chunk[2]).ok_or("expected public key is not base64")?)
        };
        let fourth = if chunk[3] == b'=' {
            if !last {
                return Err("expected public key is not base64".into());
            }
            None
        } else {
            Some(base64_value(chunk[3]).ok_or("expected public key is not base64")?)
        };
        output.push((first << 2) | (second >> 4));
        if let Some(third) = third {
            output.push((second << 4) | (third >> 2));
            if let Some(fourth) = fourth {
                output.push((third << 6) | fourth);
            }
        }
    }
    Ok(output)
}

fn base64_value(value: u8) -> Option<u8> {
    match value {
        b'A'..=b'Z' => Some(value - b'A'),
        b'a'..=b'z' => Some(value - b'a' + 26),
        b'0'..=b'9' => Some(value - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

fn zero(bytes: &mut [u8]) {
    for byte in bytes {
        // SAFETY: `byte` is a unique mutable reference into the supplied live slice.
        unsafe { ptr::write_volatile(byte, 0) };
    }
    compiler_fence(Ordering::SeqCst);
}
