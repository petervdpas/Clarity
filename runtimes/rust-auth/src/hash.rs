//! Secrets kept as hashes: Argon2id for what people choose (passwords),
//! SHA-256 for long random tokens Clarity makes, which are looked up by
//! their hash.

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::{Problem, ProblemKind};

const SHA256: &str = "sha256$";
const SHA256_LINES: &str = "sha256-lines$";

fn digest(text: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(text.as_bytes()))
}

/// A one-time code as it is compared: without spaces, in lower case.
fn code_of(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_lowercase()
}

/// The digests of a `sha256-lines` text; none for any other.
fn line_digests(stored: &str) -> Vec<&str> {
    stored.strip_prefix(SHA256_LINES).map(|d| d.split(',').filter(|d| !d.is_empty()).collect()).unwrap_or_default()
}

/// `stored` without the code `text`, when it holds it.
pub fn without_code(text: &str, stored: &str) -> Option<String> {
    let given = digest(&code_of(text));
    let digests = line_digests(stored);
    let at = digests.iter().position(|d| bool::from(d.as_bytes().ct_eq(given.as_bytes())))?;
    let left: Vec<&str> = digests.iter().enumerate().filter(|(i, _)| *i != at).map(|(_, d)| *d).collect();
    Some(format!("{SHA256_LINES}{}", left.join(",")))
}

/// The text kept for a secret, by the method of its field.
pub fn hash(method: &str, text: &str) -> Result<String, Problem> {
    match method {
        "argon2id" => {
            let mut salt = [0u8; 16];
            random(&mut salt)?;
            let salt = SaltString::encode_b64(&salt).map_err(|e| Problem::new(ProblemKind::Failed, e.to_string()))?;
            Argon2::default().hash_password(text.as_bytes(), &salt).map(|h| h.to_string()).map_err(|e| Problem::new(ProblemKind::Failed, e.to_string()))
        }
        "sha256" => Ok(format!("{SHA256}{}", URL_SAFE_NO_PAD.encode(Sha256::digest(text.as_bytes())))),
        // Several one-time codes, one per line, each kept by its own digest.
        "sha256-lines" => {
            let digests: Vec<String> = text.lines().map(code_of).filter(|c| !c.is_empty()).map(|c| digest(&c)).collect();
            Ok(format!("{SHA256_LINES}{}", digests.join(",")))
        }
        _ => Err(Problem::new(ProblemKind::BadInput, format!("there is no hash method '{method}'; there are argon2id, sha256, sha256-lines"))),
    }
}

/// Whether `text` is the secret kept as `stored`; never for an empty or unknown `stored`.
pub fn verify(text: &str, stored: &str) -> bool {
    if stored.starts_with(SHA256_LINES) {
        return without_code(text, stored).is_some();
    }
    if let Some(digest) = stored.strip_prefix(SHA256) {
        let given = URL_SAFE_NO_PAD.encode(Sha256::digest(text.as_bytes()));
        return given.as_bytes().ct_eq(digest.as_bytes()).into();
    }
    match PasswordHash::new(stored) {
        Ok(parsed) => Argon2::default().verify_password(text.as_bytes(), &parsed).is_ok(),
        Err(_) => false,
    }
}

/// A new random token, 32 bytes as URL-safe text.
pub fn new_token() -> Result<String, Problem> {
    let mut bytes = [0u8; 32];
    random(&mut bytes)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn random(into: &mut [u8]) -> Result<(), Problem> {
    getrandom::fill(into).map_err(|e| Problem::new(ProblemKind::Failed, format!("no randomness from the system: {e}")))
}

/// `count` new one-time codes, one per line, like `k7m2q-x9ftr`: letters
/// and digits that cannot be mistaken for each other.
pub fn new_codes(count: usize) -> Result<String, Problem> {
    const LETTERS: &[u8] = b"abcdefghjkmnpqrstuvwxyz23456789";
    let mut codes = Vec::new();
    for _ in 0..count.min(100) {
        let mut bytes = [0u8; 10];
        random(&mut bytes)?;
        let code: String = bytes.iter().map(|b| LETTERS[*b as usize % LETTERS.len()] as char).collect();
        codes.push(format!("{}-{}", &code[..5], &code[5..]));
    }
    Ok(codes.join("\n"))
}

/// Whether a secret has at least `minimum` characters.
pub fn fits(text: &str, minimum: u32) -> bool {
    text.chars().count() >= minimum as usize
}
