use rand::Rng;

pub fn generate_token() -> String {
    let bytes: [u8; 32] = rand::thread_rng().gen();
    hex::encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_token_is_64_hex_chars() {
        let tok = generate_token();
        assert_eq!(tok.len(), 64);
        assert!(tok.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
