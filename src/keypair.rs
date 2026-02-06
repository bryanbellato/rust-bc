use p256::{
    EncodedPoint,
    PublicKey,
    SecretKey,
    elliptic_curve::sec1::{FromEncodedPoint, ToEncodedPoint}, // Added ToEncodedPoint
};
use rand::rngs::OsRng;
use std::fmt;

#[derive(Debug, Clone)]
pub struct KeyPair {
    private_key: String,
    public_key: String,
}

#[derive(Debug)]
pub enum KeyError {
    WrongFormat,
    CryptoError(String),
}

impl fmt::Display for KeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyError::WrongFormat => write!(f, "invalid key formatting"),
            KeyError::CryptoError(msg) => write!(f, "crypto error: {}", msg),
        }
    }
}

impl std::error::Error for KeyError {}

impl From<String> for KeyError {
    fn from(err: String) -> Self {
        KeyError::CryptoError(err)
    }
}

impl KeyPair {
    pub fn generate() -> Result<Self, KeyError> {
        // generate a random secret key using P256 curve
        let secret_key = SecretKey::random(&mut OsRng);

        // convert secret key to hex (big-endian bytes)
        let private_key_hex = hex::encode(secret_key.to_bytes());

        let public_key = secret_key.public_key();

        // uncompressed format (0x04 || X || Y)
        let encoded_point = public_key.to_encoded_point(false);
        let public_key_hex = hex::encode(encoded_point.as_bytes());

        Ok(KeyPair {
            private_key: private_key_hex,
            public_key: public_key_hex,
        })
    }

    pub fn new(
        private_key: impl Into<String>,
        public_key: impl Into<String>,
    ) -> Result<Self, KeyError> {
        let private = private_key.into();
        let public = public_key.into();

        // only hexadecimal characters are valid
        let hex_regex = regex::Regex::new(r"^[0-9a-fA-F]+$").unwrap();

        if !hex_regex.is_match(&private) || !hex_regex.is_match(&public) {
            return Err(KeyError::WrongFormat);
        }

        if private.len() != 64 {
            return Err(KeyError::CryptoError(
                "private key must be 64 hex characters (32 bytes)".to_string(),
            ));
        }

        if public.len() != 130 {
            return Err(KeyError::CryptoError(
                "public key must be 130 hex characters (65 bytes uncompressed)".to_string(),
            ));
        }

        let private_bytes = hex::decode(&private)
            .map_err(|e| KeyError::CryptoError(format!("failed to decode private key: {}", e)))?;

        let _secret_key = SecretKey::from_slice(&private_bytes)
            .map_err(|e| KeyError::CryptoError(format!("invalid private key: {}", e)))?;

        let public_bytes = hex::decode(&public)
            .map_err(|e| KeyError::CryptoError(format!("failed to decode public key: {}", e)))?;

        let encoded_point = EncodedPoint::from_bytes(&public_bytes)
            .map_err(|e| KeyError::CryptoError(format!("invalid public key format: {}", e)))?;

        // FIX: Use into_option() before calling ok_or()
        let _public_key = PublicKey::from_encoded_point(&encoded_point)
            .into_option()
            .ok_or(KeyError::CryptoError(
                "invalid public key point".to_string(),
            ))?;

        Ok(KeyPair {
            private_key: private,
            public_key: public,
        })
    }

    pub fn get_private_key(&self) -> &str {
        &self.private_key
    }

    pub fn get_public_key(&self) -> &str {
        &self.public_key
    }

    pub fn print(&self) {
        println!();
        println!("Private Key: {}", self.private_key);
        println!();
        println!("Public Key: {}", self.public_key);
    }
}
