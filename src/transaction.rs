use p256::{
    EncodedPoint, PublicKey, SecretKey,
    ecdsa::{
        Signature, SigningKey, VerifyingKey,
        signature::{Signer, Verifier},
    },
    elliptic_curve::sec1::{FromEncodedPoint, ToEncodedPoint},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

use crate::currency::{Amount, AmountError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    id: String,
    from_address: String,
    to_address: String,
    amount: Amount,
    fee: Amount,
    signature: Option<String>,
}

#[derive(Debug)]
pub enum TransactionError {
    EmptyAddress,
    NonPositiveAmount,
    AmountError(AmountError),
    ExistingSignature,
    CryptoError(String),
}

impl fmt::Display for TransactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionError::EmptyAddress => write!(f, "from or to address is empty"),
            TransactionError::NonPositiveAmount => write!(f, "amount must be positive"),
            TransactionError::AmountError(e) => write!(f, "amount error: {}", e),
            TransactionError::ExistingSignature => write!(f, "transaction is already signed"),
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

impl From<AmountError> for TransactionError {
    fn from(err: AmountError) -> Self {
        TransactionError::AmountError(err)
    }
}

impl Transaction {
    pub fn new(
        from_address: impl Into<String>,
        to_address: impl Into<String>,
        amount_coins: f64,
        fee_coins: f64,
    ) -> Result<Self, TransactionError> {
        let from = from_address.into();
        let to = to_address.into();

        if from.is_empty() || to.is_empty() {
            return Err(TransactionError::EmptyAddress);
        }

        let amount = Amount::from_coins(amount_coins)?;
        let fee = Amount::from_coins(fee_coins)?;

        if amount.is_zero() {
            return Err(TransactionError::NonPositiveAmount);
        }

        let mut tx = Transaction {
            id: String::new(), // temporary
            from_address: from,
            to_address: to,
            amount,
            fee,
            signature: None,
        };

        tx.id = tx.calculate_id();

        Ok(tx)
    }

    pub fn new_reward(
        to_address: impl Into<String>,
        amount_coins: f64,
    ) -> Result<Self, TransactionError> {
        let to = to_address.into();

        if to.is_empty() {
            return Err(TransactionError::EmptyAddress);
        }

        let amount = Amount::from_coins(amount_coins)?;

        let mut tx = Transaction {
            id: String::new(),
            from_address: String::new(),
            to_address: to,
            amount,
            fee: Amount::from_satoshis(0),
            signature: None,
        };

        tx.id = tx.calculate_id();

        Ok(tx)
    }

    fn calculate_id(&self) -> String {
        let data = format!(
            "FromAddress:{},ToAddress:{},Amount:{},Fee:{}",
            self.from_address,
            self.to_address,
            self.amount.satoshis(),
            self.fee.satoshis()
        );

        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let hash = hasher.finalize();
        hex::encode(hash)
    }

    fn get_data_string(&self) -> String {
        format!(
            "FromAddress:{},ToAddress:{},Amount:{},Fee:{}",
            self.from_address,
            self.to_address,
            self.amount.satoshis(),
            self.fee.satoshis()
        )
    }

    /*
     deprecated...

    pub fn calc_transaction_hash(&self) -> [u8; 32] {
        let data = self.get_data_string();
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        hasher.finalize().into()
    }
     */

    pub fn sign(&mut self, private_key_hex: &str) -> Result<(), TransactionError> {
        if self.signature.is_some() {
            return Err(TransactionError::ExistingSignature);
        }

        let private_key_bytes = hex::decode(private_key_hex)
            .map_err(|e| format!("failed to decode private key: {}", e))?;

        let secret_key = SecretKey::from_slice(&private_key_bytes)
            .map_err(|e| format!("invalid private key bytes: {}", e))?;

        let public_key = secret_key.public_key();

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

    pub fn estimate_size(&self) -> usize {
        let base_size = 32 +  // txid (hash)
        65 +  // from_address (uncompressed pubkey)
        65 +  // to_address
        8 +   // amount (u64)
        8; // fee (u64)

        let witness_size = self
            .signature
            .as_ref()
            .map(|sig| sig.len() / 2) // hex to bytes
            .unwrap_or(72); // signature DER ~70-72 bytes

        base_size + witness_size
    }

    pub fn suggest_fee(&self, satoshis_per_byte: u64) -> Amount {
        let size = self.estimate_size() as u64;
        Amount::from_satoshis(size * satoshis_per_byte)
    }

    pub fn total_cost(&self) -> Amount {
        self.amount.checked_add(self.fee).unwrap_or(self.amount) // if occurs a overflow, it returns only the amount
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn from_address(&self) -> &str {
        &self.from_address
    }

    pub fn to_address(&self) -> &str {
        &self.to_address
    }

    pub fn amount(&self) -> Amount {
        self.amount
    }

    pub fn fee(&self) -> Amount {
        self.fee
    }

    pub fn signature(&self) -> Option<&str> {
        self.signature.as_deref()
    }
}
