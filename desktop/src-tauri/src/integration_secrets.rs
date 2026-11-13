//! Independent Windows user-scope protection for official integration credentials.
//! This module has no dependency on the OBR activation or licensing system.
#[cfg(windows)]
fn transform(input: &[u8], encrypt: bool) -> Result<Vec<u8>, String> {
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN};
    let source = CRYPT_INTEGER_BLOB { cbData: input.len().try_into().map_err(|_| "integration.secret_size")?, pbData: input.as_ptr() as *mut u8 };
    let entropy_bytes = b"Realty Office/official-integrations/v1";
    let entropy = CRYPT_INTEGER_BLOB { cbData: entropy_bytes.len() as u32, pbData: entropy_bytes.as_ptr() as *mut u8 };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        let result = if encrypt {
            CryptProtectData(&source, windows::core::PCWSTR::null(), Some(&entropy), None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut output)
        } else {
            CryptUnprotectData(&source, None, Some(&entropy), None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut output)
        };
        result.map_err(|_| "integration.secret_protection".to_string())?;
        let bytes = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        // Clear the native plaintext allocation before releasing it.
        if !encrypt { std::ptr::write_bytes(output.pbData, 0, output.cbData as usize); }
        let _ = LocalFree(HLOCAL(output.pbData as *mut _));
        Ok(bytes)
    }
}

pub fn protect(input: &[u8]) -> Result<Vec<u8>, String> {
    #[cfg(windows)] { transform(input, true) }
    #[cfg(not(windows))] { let _ = input; Err("integration.windows_required".into()) }
}
pub fn unprotect(input: &[u8]) -> Result<Vec<u8>, String> {
    #[cfg(windows)] { transform(input, false) }
    #[cfg(not(windows))] { let _ = input; Err("integration.windows_required".into()) }
}
