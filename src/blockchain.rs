use crate::block::Block;
use crate::currency::Amount;
use crate::transaction::Transaction;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::{Arc, RwLock};

// this is the type the network layer will pass around between threads.
/*
   RwLock is chosen over Mutex because get_balance, is_chain_valid and chain()
   can run oncurrently across many threads with no coordination overhead
   only mine_pending_transactions and add_transaction need exclusive access
*/
pub type SharedBlockchain = Arc<RwLock<Blockchain>>;

// convenience constructor so callers don't need to spell out Arc::new(RwLock::new(...))
pub fn new_shared(blockchain: Blockchain) -> SharedBlockchain {
    Arc::new(RwLock::new(blockchain))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blockchain {
    chain: Vec<Block>,
    difficulty: usize,
    pending_transactions: Vec<Transaction>,
    mining_reward: Amount,
    min_fee_rate: u64,
}

#[derive(Debug)]
pub enum BlockchainError {
    EmptyAddress,
    InvalidSignature(String),
    InvalidTransaction(String),
    NoBlocks,
}

impl fmt::Display for BlockchainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockchainError::EmptyAddress => {
                write!(f, "transaction must include from and to address")
            }
            BlockchainError::InvalidSignature(msg) => {
                write!(f, "invalid transaction signature: {}", msg)
            }
            BlockchainError::InvalidTransaction(msg) => {
                write!(f, "invalid transaction: {}", msg)
            }
            BlockchainError::NoBlocks => write!(f, "blockchain has no blocks"),
        }
    }
}

impl std::error::Error for BlockchainError {}

impl Blockchain {
    pub fn new(
        difficulty: usize,
        mining_reward_coins: f64,
        min_fee_rate_sats: u64,
        miner_address: String,
    ) -> Self {
        let mining_reward = Amount::from_coins(mining_reward_coins).expect("Invalid mining reward");

        let genesis_block = Self::create_genesis_block(miner_address, mining_reward);

        Blockchain {
            chain: vec![genesis_block],
            difficulty,
            pending_transactions: Vec::new(),
            mining_reward,
            min_fee_rate: min_fee_rate_sats,
        }
    }

    fn create_genesis_block(miner_address: String, initial_reward: Amount) -> Block {
        let genesis_transaction = Transaction::new_reward(miner_address, initial_reward.as_coins())
            .expect("Failed to create genesis transaction");

        Block::origin(vec![genesis_transaction], "0".to_string())
    }

    pub fn get_latest_block(&self) -> Result<&Block, BlockchainError> {
        self.chain.last().ok_or(BlockchainError::NoBlocks)
    }

    fn calculate_total_fees(&self) -> Amount {
        self.pending_transactions
            .iter()
            .fold(Amount::from_satoshis(0), |acc, tx| {
                acc.checked_add(tx.fee()).unwrap_or(acc)
            })
    }

    pub fn mine_pending_transactions(
        &mut self,
        mining_reward_address: String,
    ) -> Result<(), BlockchainError> {
        let total_fees = self.calculate_total_fees();

        let total_miner_reward = self
            .mining_reward
            .checked_add(total_fees)
            .unwrap_or(self.mining_reward);

        let reward_tx =
            Transaction::new_reward(mining_reward_address, total_miner_reward.as_coins())
                .map_err(|e| BlockchainError::InvalidTransaction(e.to_string()))?;

        self.pending_transactions.push(reward_tx);

        let previous_hash = self.get_latest_block()?.hash().to_string();

        let mut new_block = Block::new(
            self.pending_transactions.clone(),
            previous_hash,
            &self.chain,
        )
        .map_err(|e| BlockchainError::InvalidTransaction(e.to_string()))?;

        new_block.mine_block(self.difficulty);

        self.chain.push(new_block);
        self.pending_transactions.clear();

        Ok(())
    }

    /*
     get the balance of an address across
     mined blocks
    */
    pub fn get_balance(&self, address: &str) -> Amount {
        let mut balance = Amount::from_satoshis(0);

        for block in &self.chain {
            for tx in block.transactions() {
                if tx.from_address() == address {
                    // cost of fee for the sender
                    let total_cost = tx.total_cost();
                    balance = balance
                        .checked_sub(total_cost)
                        .unwrap_or(Amount::from_satoshis(0));
                }
                if tx.to_address() == address {
                    // receiver receives the amount without fee
                    balance = balance.checked_add(tx.amount()).unwrap_or(balance);
                }
            }
        }

        balance
    }

    /*
     get the balance of an address across
     the previously confirmed balance
     minus pending transactions
    */
    pub fn get_available_balance(&self, address: &str) -> Amount {
        let mut balance = self.get_balance(address);

        for tx in &self.pending_transactions {
            if tx.from_address() == address {
                balance = balance
                    .checked_sub(tx.total_cost()) // amount + fee
                    .unwrap_or(Amount::from_satoshis(0));
            }
        }

        balance
    }

    // add a transaction to the pending transactions pool
    pub fn add_transaction(&mut self, tx: Transaction) -> Result<(), BlockchainError> {
        if tx.from_address().is_empty() || tx.to_address().is_empty() {
            return Err(BlockchainError::EmptyAddress);
        }

        if !tx.from_address().is_empty() {
            // validates minimum fee
            let min_fee = Amount::from_satoshis(tx.estimate_size() as u64 * self.min_fee_rate);

            if tx.fee() < min_fee {
                return Err(BlockchainError::InvalidTransaction(format!(
                    "fee too low: {} coins, minimum {} coins",
                    tx.fee(),
                    min_fee
                )));
            }

            // validates balance (amount + fee)
            let available = self.get_available_balance(tx.from_address());
            let total_cost = tx.total_cost();

            if total_cost > available {
                return Err(BlockchainError::InvalidTransaction(format!(
                    "insufficient funds: has {} coins, trying to send {} coins (amount: {}, fee: {})",
                    available,
                    total_cost,
                    tx.amount(),
                    tx.fee()
                )));
            }
        }

        match tx.verify_signature() {
            Ok(valid) => {
                if !valid {
                    return Err(BlockchainError::InvalidSignature(
                        "signature verification failed".to_string(),
                    ));
                }
            }
            Err(e) => {
                return Err(BlockchainError::InvalidSignature(e.to_string()));
            }
        }

        self.pending_transactions.push(tx);
        Ok(())
    }

    /*
     replace_chain — used during P2P sync (longest valid chain wins)

     replaces the local chain only if the incoming chain is longer or
     100% internally valid (all hashes, merkle roots, timestamps check out)

     returns true if the chain was replaced or false if ours was kept.
    */
    pub fn replace_chain(&mut self, incoming: Vec<Block>) -> bool {
        if incoming.len() <= self.chain.len() {
            return false;
        }

        if !Self::validate_chain(&incoming) {
            println!("[SYNC] Rejected incoming chain: failed validation");
            return false;
        }

        println!(
            "[SYNC] Replacing chain: {} blocks -> {} blocks",
            self.chain.len(),
            incoming.len()
        );
        self.chain = incoming;
        self.pending_transactions.clear();
        true
    }

    fn validate_chain(chain: &[Block]) -> bool {
        for i in 1..chain.len() {
            let current = &chain[i];
            let previous = &chain[i - 1];

            if !current.has_valid_transactions() {
                return false;
            }
            if !current.validate_block() {
                return false;
            }
            if current.previous_hash() != previous.hash() {
                return false;
            }
            if current.validate_timestamp(&chain[0..i]).is_err() {
                return false;
            }
        }
        true
    }

    pub fn is_chain_valid(&self) -> bool {
        Self::validate_chain(&self.chain)
    }

    // getters
    pub fn chain(&self) -> &[Block] {
        &self.chain
    }

    pub fn difficulty(&self) -> usize {
        self.difficulty
    }

    pub fn pending_transactions(&self) -> &[Transaction] {
        &self.pending_transactions
    }

    pub fn mining_reward(&self) -> Amount {
        self.mining_reward
    }

    pub fn min_fee_rate(&self) -> u64 {
        self.min_fee_rate
    }

    pub fn print_chain(&self) {
        println!("\n Blockchain:");
        println!("Difficulty: {}", self.difficulty);
        println!("Mining Reward: {} coins", self.mining_reward);
        println!("Total Blocks: {}", self.chain.len());
        println!("Pending Transactions: {}", self.pending_transactions.len());
        println!("\n--- Blocks ---");

        for (i, block) in self.chain.iter().enumerate() {
            println!("\nBlock #{}:", i);
            println!("  Timestamp: {}", block.timestamp());
            println!("  Previous Hash: {}", block.previous_hash());
            println!("  Hash: {}", block.hash());
            println!("  Nonce: {}", block.nonce());
            println!("  Transactions: {}", block.transactions().len());

            for (j, tx) in block.transactions().iter().enumerate() {
                println!("    Transaction #{}:", j);
                println!("    Hash ID: #{}:", tx.id());
                println!("      From: {}", tx.from_address());
                println!("      To: {}", tx.to_address());
                println!("      Amount: {} coins", tx.amount());
                println!("      Signature: {:?}", tx.signature());
            }
        }
        println!("\n==================\n");
    }
}

#[cfg(test)]
mod serde_tests {
    use super::*;
    use crate::keypair::KeyPair;

    fn make_test_blockchain() -> Blockchain {
        let kp = KeyPair::generate().unwrap();
        let addr = kp.get_public_key().to_string();

        let mut bc = Blockchain::new(1, 10.0, 1, addr.clone());

        let mut tx = Transaction::new(&addr, &addr, 1.0, 0.001).unwrap();
        tx.sign(kp.get_private_key()).unwrap();
        bc.add_transaction(tx).unwrap();
        bc.mine_pending_transactions(addr).unwrap();

        bc
    }

    #[test]
    fn amount_roundtrip() {
        let a = Amount::from_satoshis(150_000_000);
        let json = serde_json::to_string(&a).unwrap();
        assert_eq!(json, "150000000");
        let back: Amount = serde_json::from_str(&json).unwrap();
        assert_eq!(a, back);
    }

    #[test]
    fn transaction_roundtrip() {
        let kp = KeyPair::generate().unwrap();
        let addr = kp.get_public_key().to_string();
        let mut tx = Transaction::new(&addr, &addr, 1.0, 0.001).unwrap();
        tx.sign(kp.get_private_key()).unwrap();

        let json = serde_json::to_string(&tx).unwrap();
        let back: Transaction = serde_json::from_str(&json).unwrap();

        assert_eq!(tx.id(), back.id());
        assert_eq!(tx.amount(), back.amount());
        assert_eq!(tx.signature(), back.signature());
        assert!(back.verify_signature().unwrap());
    }

    #[test]
    fn block_roundtrip() {
        let bc = make_test_blockchain();
        let block = &bc.chain()[1];

        let json = serde_json::to_string(block).unwrap();
        let back: Block = serde_json::from_str(&json).unwrap();

        assert_eq!(block.hash(), back.hash());
        assert_eq!(block.merkle_root(), back.merkle_root());
        assert_eq!(block.transactions().len(), back.transactions().len());
        assert!(back.validate_block());
    }

    #[test]
    fn blockchain_roundtrip() {
        let bc = make_test_blockchain();

        let json = serde_json::to_string(&bc).unwrap();
        let back: Blockchain = serde_json::from_str(&json).unwrap();

        assert_eq!(bc.chain().len(), back.chain().len());
        assert!(back.is_chain_valid());
    }

    #[test]
    fn merkle_root_serializes_as_hex_string() {
        let bc = make_test_blockchain();
        let block = &bc.chain()[1];
        let json = serde_json::to_string(block).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(
            parsed["merkle_root"].is_string(),
            "merkle_root should serialize as a hex string"
        );
        let hex_str = parsed["merkle_root"].as_str().unwrap();
        assert_eq!(
            hex_str.len(),
            64,
            "hex string should be 64 chars (32 bytes)"
        );
    }
}
