use crate::transaction::Transaction;
use sha2::{Digest, Sha256};
use std::fmt;

#[derive(Debug, Clone)]
pub struct Block {
    timestamp: String,
    transactions: Vec<Transaction>,
    previous_hash: String,
    hash: String,
    nonce: u64,
}

#[derive(Debug)]
pub enum BlockError {
    InvalidTransaction(String),
    MiningError(String),
}

impl fmt::Display for BlockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockError::InvalidTransaction(msg) => write!(f, "invalid transaction: {}", msg),
            BlockError::MiningError(msg) => write!(f, "mining error: {}", msg),
        }
    }
}

impl std::error::Error for BlockError {}

impl Block {
    pub fn new(transactions: Vec<Transaction>, previous_hash: String) -> Self {
        let timestamp = chrono::Utc::now().to_rfc3339();
        let mut block = Block {
            timestamp,
            transactions,
            previous_hash,
            hash: String::new(),
            nonce: 0,
        };
        block.hash = block.calculate_hash();
        block
    }

    pub fn calculate_hash(&self) -> String {
        let mut tx_strings = Vec::new();

        for tx in &self.transactions {
            let tx_str = format!(
                "{}|{}|{}|{}",
                tx.from_address(),
                tx.to_address(),
                tx.amount(),
                tx.signature().unwrap_or("")
            );
            tx_strings.push(tx_str);
        }

        let transactions_str = tx_strings.join(",");
        let data = format!(
            "{}{}{}{}",
            self.timestamp, transactions_str, self.previous_hash, self.nonce
        );

        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let hash = hasher.finalize();

        hex::encode(hash)
    }

    pub fn mine_block(&mut self, difficulty: usize) {
        let target = "0".repeat(difficulty);

        println!("Mining block with {} transactions", self.transactions.len());

        loop {
            self.hash = self.calculate_hash();

            if self.hash.starts_with(&target) {
                println!("Block mined! Hash: {}", self.hash);
                println!("Nonce used: {}", self.nonce);
                return;
            }

            self.nonce += 1;

            if self.nonce % 100_000 == 0 {
                println!("Mining in progress... Nonce: {}", self.nonce);
            }
        }
    }

    pub fn validate_block(&self) -> bool {
        self.hash == self.calculate_hash()
    }

    /// check if all transactions in the block have valid signatures
    pub fn has_valid_transactions(&self) -> bool {
        for tx in &self.transactions {
            if tx.from_address().is_empty() {
                continue;
            }

            match tx.verify_signature() {
                Ok(valid) => {
                    if !valid {
                        return false;
                    }
                }
                Err(_) => return false,
            }
        }
        true
    }

    // getters
    pub fn timestamp(&self) -> &str {
        &self.timestamp
    }

    pub fn transactions(&self) -> &[Transaction] {
        &self.transactions
    }

    pub fn previous_hash(&self) -> &str {
        &self.previous_hash
    }

    pub fn hash(&self) -> &str {
        &self.hash
    }

    pub fn nonce(&self) -> u64 {
        self.nonce
    }
}
