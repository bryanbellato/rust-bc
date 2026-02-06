use p256::{
    EncodedPoint,
    PublicKey,
    SecretKey,
    ecdsa::{
        Signature, SigningKey, VerifyingKey,
        signature::{Signer, Verifier},
    },
    elliptic_curve::sec1::{FromEncodedPoint, ToEncodedPoint}, // Added ToEncodedPoint
};
use sha2::{Digest, Sha256};
use std::fmt;

#[derive(Debug, Clone)]
pub struct Transaction {
    // Make public
    from_address: String,
    to_address: String,
    amount: f64,
    signature: Option<String>,
}

#[derive(Debug)]
pub enum TransactionError {
    EmptyAddress,
    NonPositiveAmount,
    AmountTooLarge,
    ExistingSignature,
    InsufficientFunds,
    CryptoError(String),
}

impl fmt::Display for TransactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionError::EmptyAddress => write!(f, "from or to address is empty"),
            TransactionError::NonPositiveAmount => write!(f, "amount must be positive"),
            TransactionError::AmountTooLarge => write!(f, "amount is too large"),
            TransactionError::ExistingSignature => write!(f, "transaction is already signed"),
            TransactionError::InsufficientFunds => write!(f, "Not enough funds to send."),
            TransactionError::CryptoError(msg) => write!(f, "crypto error: {}", msg),
        }
    }
}

impl std::error::Error for TransactionError {}

// allow automatic conversion from String errors to our custom Enum
impl From<String> for TransactionError {
    fn from(err: String) -> Self {
        TransactionError::CryptoError(err)
    }
}

impl Transaction {
    pub fn new(
        from_address: impl Into<String>,
        to_address: impl Into<String>,
        amount: f64,
    ) -> Result<Self, TransactionError> {
        let from = from_address.into();
        let to = to_address.into();

        if from.is_empty() || to.is_empty() {
            return Err(TransactionError::EmptyAddress);
        }
        if amount <= 0.0 {
            return Err(TransactionError::NonPositiveAmount);
        }
        if amount > f64::MAX / 2.0 {
            return Err(TransactionError::AmountTooLarge);
        }

        Ok(Transaction {
            from_address: from,
            to_address: to,
            amount,
            signature: None,
        })
    }

    // used for mining rewards and Genesis Block
    // this allows the 'from_address' to be empty.
    pub fn new_reward(
        to_address: impl Into<String>,
        amount: f64,
    ) -> Result<Self, TransactionError> {
        let to = to_address.into();

        if to.is_empty() {
            return Err(TransactionError::EmptyAddress);
        }

        if amount < 0.0 {
            return Err(TransactionError::NonPositiveAmount);
        }

        Ok(Transaction {
            from_address: String::new(), // Explicitly empty
            to_address: to,
            amount,
            signature: None,
        })
    }

    fn get_data_string(&self) -> String {
        format!(
            "FromAddress:{},ToAddress:{},Amount:{}",
            self.from_address, self.to_address, self.amount
        )
    }

    pub fn calc_transaction_hash(&self) -> [u8; 32] {
        let data = self.get_data_string();
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        hasher.finalize().into()
    }

    pub fn sign(&mut self, private_key_hex: &str) -> Result<(), TransactionError> {
        if self.signature.is_some() {
            return Err(TransactionError::ExistingSignature);
        }

        let private_key_bytes = hex::decode(private_key_hex)
            .map_err(|e| format!("failed to decode private key: {}", e))?;

        let secret_key = SecretKey::from_slice(&private_key_bytes)
            .map_err(|e| format!("invalid private key bytes: {}", e))?;

        let public_key = secret_key.public_key();

        // Used ToEncodedPoint here
        let encoded_point = public_key.to_encoded_point(false);
        let public_key_hex = hex::encode(encoded_point.as_bytes());

        if public_key_hex != self.from_address {
            return Err(TransactionError::CryptoError(
                "private key does not correspond to FromAddress".to_string(),
            ));
        }

        let signing_key = SigningKey::from(secret_key);

        // p256::SigningKey:: will sign automatically hashes the input with SHA-256 before signing.
        let data = self.get_data_string();
        let signature: Signature = signing_key.sign(data.as_bytes());

        // serialize Signature to DER (ASN.1) and then Hex
        let der_signature = signature.to_der();
        self.signature = Some(hex::encode(der_signature.as_bytes()));

        Ok(())
    }

    pub fn verify_signature(&self) -> Result<bool, TransactionError> {
        // check if signature exists
        let signature_hex = self.signature.as_ref().ok_or_else(|| {
            TransactionError::CryptoError("transaction is not signed".to_string())
        })?;

        let public_key_bytes = hex::decode(&self.from_address)
            .map_err(|e| format!("failed to decode public key: {}", e))?;

        // parse the encoded point (uncompressed format)
        let encoded_point = EncodedPoint::from_bytes(&public_key_bytes)
            .map_err(|e| format!("failed to parse public key point: {}", e))?;

        // FIX: Use into_option() before calling ok_or()
        let public_key = PublicKey::from_encoded_point(&encoded_point)
            .into_option()
            .ok_or(TransactionError::CryptoError(
                "failed to unmarshal public key".to_string(),
            ))?;

        // create verifying key from public key
        let verifying_key = VerifyingKey::from(public_key);

        let signature_bytes =
            hex::decode(signature_hex).map_err(|e| format!("failed to decode signature: {}", e))?;

        // parse DER-encoded signature
        let signature = Signature::from_der(&signature_bytes)
            .map_err(|e| format!("failed to unmarshal signature: {}", e))?;

        let data = self.get_data_string();

        // p256's verify_signature will hash the data with SHA-256 automatically
        Ok(verifying_key.verify(data.as_bytes(), &signature).is_ok())
    }

    pub fn from_address(&self) -> &str {
        &self.from_address
    }
    pub fn to_address(&self) -> &str {
        &self.to_address
    }
    pub fn amount(&self) -> f64 {
        self.amount
    }
    pub fn signature(&self) -> Option<&str> {
        self.signature.as_deref()
    }
}
