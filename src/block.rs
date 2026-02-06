use crate::transaction::Transaction;
use chrono::{DateTime, Utc};
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
    InvalidTimestamp(String),
    MiningError(String),
}

impl fmt::Display for BlockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockError::InvalidTransaction(msg) => write!(f, "invalid transaction: {}", msg),
            BlockError::InvalidTimestamp(msg) => write!(f, "invalid timestamp: {}", msg),
            BlockError::MiningError(msg) => write!(f, "mining error: {}", msg),
        }
    }
}

impl std::error::Error for BlockError {}

impl Block {
    // for new blocks that need timestamp validations
    pub fn new(
        transactions: Vec<Transaction>,
        previous_hash: String,
        previous_blocks: &[Block], // receive previous blocks for MTP validation
    ) -> Result<Self, BlockError> {
        let timestamp = chrono::Utc::now();

        // Median Time Past (MTP) validation
        if !previous_blocks.is_empty() {
            let median_time_past = Self::calculate_median_time_past(previous_blocks);

            if timestamp <= median_time_past {
                return Err(BlockError::InvalidTimestamp(format!(
                    "Block timestamp ({}) must be greater than Median Time Past ({})",
                    timestamp.to_rfc3339(),
                    median_time_past.to_rfc3339()
                )));
            }
        }

        // not too far in the future validation
        const MAX_FUTURE_DRIFT_SECONDS: i64 = 2 * 60 * 60; // 2 hours
        let max_allowed_time = Utc::now() + chrono::Duration::seconds(MAX_FUTURE_DRIFT_SECONDS);

        if timestamp > max_allowed_time {
            return Err(BlockError::InvalidTimestamp(format!(
                "Block timestamp ({}) is too far in the future. Max allowed: {}",
                timestamp.to_rfc3339(),
                max_allowed_time.to_rfc3339()
            )));
        }

        let mut block = Block {
            timestamp: timestamp.to_rfc3339(),
            transactions,
            previous_hash,
            hash: String::new(),
            nonce: 0,
        };

        block.hash = block.calculate_hash();
        Ok(block)
    }

    // for genesis blocks and possible other non-validation cases
    pub fn origin(transactions: Vec<Transaction>, previous_hash: String) -> Self {
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

    // calculates the median time of the latest 11 blocks
    fn calculate_median_time_past(previous_blocks: &[Block]) -> DateTime<Utc> {
        const MEDIAN_BLOCK_COUNT: usize = 11;

        let block_count = previous_blocks.len().min(MEDIAN_BLOCK_COUNT);

        let mut timestamps: Vec<DateTime<Utc>> = previous_blocks
            .iter()
            .rev()
            .take(block_count)
            .filter_map(|block| {
                DateTime::parse_from_rfc3339(block.timestamp())
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            })
            .collect();

        if timestamps.is_empty() {
            return DateTime::from_timestamp(0, 0).unwrap();
        }

        timestamps.sort_unstable();

        let median_index = timestamps.len() / 2;

        timestamps[median_index]
    }

    // validates a block timestamp comparing to the blockchain chains
    pub fn validate_timestamp(&self, previous_blocks: &[Block]) -> Result<(), BlockError> {
        let block_time = DateTime::parse_from_rfc3339(&self.timestamp)
            .map_err(|e| BlockError::InvalidTimestamp(format!("Invalid timestamp format: {}", e)))?
            .with_timezone(&Utc);

        if !previous_blocks.is_empty() {
            let median_time_past = Self::calculate_median_time_past(previous_blocks);

            if block_time <= median_time_past {
                return Err(BlockError::InvalidTimestamp(format!(
                    "Block timestamp must be greater than Median Time Past. Block: {}, MTP: {}",
                    block_time.to_rfc3339(),
                    median_time_past.to_rfc3339()
                )));
            }
        }

        const MAX_FUTURE_DRIFT_SECONDS: i64 = 2 * 60 * 60;
        let max_allowed = Utc::now() + chrono::Duration::seconds(MAX_FUTURE_DRIFT_SECONDS);

        if block_time > max_allowed {
            return Err(BlockError::InvalidTimestamp(format!(
                "Block timestamp too far in future. Block: {}, Max: {}",
                block_time.to_rfc3339(),
                max_allowed.to_rfc3339()
            )));
        }

        Ok(())
    }

    pub fn calculate_hash(&self) -> String {
        let mut tx_strings = Vec::new();

        for tx in &self.transactions {
            let tx_str = format!(
                "{}|{}|{}|{}",
                tx.from_address(),
                tx.to_address(),
                tx.amount().satoshis(),
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

    // check if all transactions in the block have valid signatures
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
