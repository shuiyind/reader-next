use crate::util::hash::md5_hex;
use hmac::{Hmac, Mac};
use rand::{distributions::Alphanumeric, Rng};
use sha2::Sha256;
use subtle::ConstantTimeEq;

const PBKDF2_ROUNDS: u32 = 210_000;
const PBKDF2_PREFIX: &str = "pbkdf2_sha256";

type HmacSha256 = Hmac<Sha256>;

pub fn random_string(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

pub fn hash_password(password: &str, salt: &str) -> String {
    let derived = pbkdf2_sha256(password.as_bytes(), salt.as_bytes(), PBKDF2_ROUNDS);
    format!(
        "{PBKDF2_PREFIX}${PBKDF2_ROUNDS}${}${}",
        hex::encode(salt.as_bytes()),
        hex::encode(derived)
    )
}

pub fn verify_password(password: &str, stored: &str, legacy_salt: &str) -> bool {
    if let Some(stored_hash) = parse_and_verify_pbkdf2(password, stored) {
        return stored_hash;
    }

    constant_time_eq(
        gen_encrypted_password(password, legacy_salt).as_bytes(),
        stored.as_bytes(),
    )
}

pub fn password_needs_rehash(stored: &str) -> bool {
    !stored.starts_with(&format!("{PBKDF2_PREFIX}$"))
}

pub fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    bool::from(left.ct_eq(right))
}

pub fn gen_encrypted_password(password: &str, salt: &str) -> String {
    let first = md5_hex(&format!("{password}{salt}"));
    md5_hex(&format!("{first}{salt}"))
}

fn parse_and_verify_pbkdf2(password: &str, stored: &str) -> Option<bool> {
    let mut parts = stored.split('$');
    if parts.next()? != PBKDF2_PREFIX {
        return None;
    }
    let rounds = parts.next()?.parse::<u32>().ok()?;
    let expected_salt = parts.next()?;
    let expected_hash = hex::decode(parts.next()?).ok()?;
    if parts.next().is_some() || !(10_000..=2_000_000).contains(&rounds) {
        return None;
    }

    let salt = hex::decode(expected_salt).ok()?;
    let derived = pbkdf2_sha256(password.as_bytes(), &salt, rounds);
    Some(constant_time_eq(&derived, &expected_hash))
}

fn pbkdf2_sha256(password: &[u8], salt: &[u8], rounds: u32) -> [u8; 32] {
    let mut input = salt.to_vec();
    input.extend_from_slice(&1u32.to_be_bytes());
    let mut current = hmac_sha256(password, &input);
    let mut output = current;
    for _ in 1..rounds {
        current = hmac_sha256(password, &current);
        for index in 0..output.len() {
            output[index] ^= current[index];
        }
    }
    output
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts keys of any size");
    mac.update(data);
    let bytes = mac.finalize().into_bytes();
    let mut output = [0u8; 32];
    output.copy_from_slice(&bytes);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_password_hashes_are_verifiable_and_not_legacy_md5() {
        let hash = hash_password("password123", "salt");
        assert!(hash.starts_with("pbkdf2_sha256$"));
        assert!(verify_password("password123", &hash, "salt"));
        assert!(!verify_password("password124", &hash, "salt"));
    }

    #[test]
    fn legacy_passwords_remain_verifiable() {
        let legacy = gen_encrypted_password("password123", "salt");
        assert!(verify_password("password123", &legacy, "salt"));
        assert!(password_needs_rehash(&legacy));
    }
}
