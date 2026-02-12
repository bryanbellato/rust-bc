use crate::merkle::{self, MerkleProof, MerkleRoot};
use crate::transaction::Transaction;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use std::fmt;

#[derive(Debug, Clone)]
pub struct Block {
    timestamp: String,
    transactions: Vec<Transaction>,
    merkle_root: MerkleRoot,
    previous_hash: String,
    hash: String,
    nonce: u64,
}

#[derive(Debug, Clone)]
pub struct SPVProof {
    pub tx: Transaction,
    pub tx_index: usize,
    pub merkle_proof: MerkleProof,
    pub block_hash: String,
    pub merkle_root: MerkleRoot,
}

impl SPVProof {
    pub fn verify(&self) -> bool {
        Block::verify_proof_with_root(
            &self.tx,
            self.tx_index,
            &self.merkle_proof,
            &self.merkle_root,
        )
    }
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
        previous_blocks: &[Block],
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

        let merkle_root = Self::calculate_merkle_root_from_txs(&transactions)?;

        let mut block = Block {
            timestamp: timestamp.to_rfc3339(),
            transactions,
            merkle_root,
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

        let merkle_root = Self::calculate_merkle_root_from_txs(&transactions).unwrap_or([0u8; 32]);

        let mut block = Block {
            timestamp,
            transactions,
            merkle_root,
            previous_hash,
            hash: String::new(),
            nonce: 0,
        };

        block.hash = block.calculate_hash();
        block
    }

    fn calculate_merkle_root_from_txs(
        transactions: &[Transaction],
    ) -> Result<MerkleRoot, BlockError> {
        if transactions.is_empty() {
            return Ok([0u8; 32]);
        }

        let tx_hashes: Vec<[u8; 32]> = transactions
            .iter()
            .map(|tx| Self::hash_transaction(tx))
            .collect();

        Ok(merkle::calculate_merkle_root(&tx_hashes))
    }

    fn hash_transaction(tx: &Transaction) -> [u8; 32] {
        let tx_data = format!(
            "{}|{}|{}|{}|{}",
            tx.from_address(),
            tx.to_address(),
            tx.amount().satoshis(),
            tx.fee().satoshis(),
            tx.signature().unwrap_or("")
        );

        let first_hash = Sha256::digest(tx_data.as_bytes());
        let second_hash = Sha256::digest(first_hash);
        second_hash.into()
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
        let data = format!(
            "{}{}{}{}",
            self.timestamp,
            hex::encode(self.merkle_root),
            self.previous_hash,
            self.nonce
        );

        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        hex::encode(hasher.finalize())
    }

    pub fn get_transaction_proof(&self, tx_index: usize) -> Result<MerkleProof, BlockError> {
        if tx_index >= self.transactions.len() {
            return Err(BlockError::InvalidTransaction(
                "Transaction index out of bounds".to_string(),
            ));
        }

        let tx_hashes: Vec<[u8; 32]> = self
            .transactions
            .iter()
            .map(|tx| Self::hash_transaction(tx))
            .collect();

        let (_root, tree) = merkle::build_merkle_tree(&tx_hashes);

        merkle::generate_proof(&tree, tx_index).map_err(|e| BlockError::InvalidTransaction(e))
    }

    pub fn verify_transaction_proof(
        &self,
        tx: &Transaction,
        tx_index: usize,
        proof: &MerkleProof,
    ) -> bool {
        let tx_hash = Self::hash_transaction(tx);
        merkle::verify_proof(&tx_hash, tx_index, proof, &self.merkle_root)
    }

    pub fn verify_proof_with_root(
        tx: &Transaction,
        tx_index: usize,
        proof: &MerkleProof,
        merkle_root: &MerkleRoot,
    ) -> bool {
        let tx_hash = Self::hash_transaction(tx);
        merkle::verify_proof(&tx_hash, tx_index, proof, merkle_root)
    }

    pub fn create_spv_proof(&self, tx_index: usize) -> Result<SPVProof, BlockError> {
        if tx_index >= self.transactions.len() {
            return Err(BlockError::InvalidTransaction(
                "Transaction index out of bounds".to_string(),
            ));
        }

        let proof = self.get_transaction_proof(tx_index)?;

        Ok(SPVProof {
            tx: self.transactions[tx_index].clone(),
            tx_index,
            merkle_proof: proof,
            block_hash: self.hash.clone(),
            merkle_root: self.merkle_root,
        })
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
        if self.hash != self.calculate_hash() {
            return false;
        }

        if let Ok(computed_root) = Self::calculate_merkle_root_from_txs(&self.transactions) {
            if computed_root != self.merkle_root {
                println!("Merkle root mismatch!");
                return false;
            }
        } else {
            return false;
        }

        true
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

    pub fn print_merkle_tree(&self) {
        println!("Generatin Merkle Tree for Block Hash: {}", self.hash);

        let tx_hashes: Vec<[u8; 32]> = self
            .transactions
            .iter()
            .map(|tx| Self::hash_transaction(tx))
            .collect();

        let (_root, tree_structure) = merkle::build_merkle_tree(&tx_hashes);

        merkle::print_merkle_tree(&tree_structure);
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

    pub fn merkle_root(&self) -> &MerkleRoot {
        &self.merkle_root
    }
}
