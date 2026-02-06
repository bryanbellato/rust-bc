use crate::block::Block;
use crate::transaction::Transaction;
use std::fmt;

use crate::currency::Amount;

#[derive(Debug, Clone)]
pub struct Blockchain {
    chain: Vec<Block>,
    difficulty: usize,
    pending_transactions: Vec<Transaction>,
    mining_reward: Amount,
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
    pub fn new(difficulty: usize, mining_reward_coins: f64, miner_address: String) -> Self {
        let mining_reward = Amount::from_coins(mining_reward_coins).expect("Invalid mining reward");

        let genesis_block = Self::create_genesis_block(miner_address, mining_reward);

        Blockchain {
            chain: vec![genesis_block],
            difficulty,
            pending_transactions: Vec::new(),
            mining_reward,
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

    pub fn mine_pending_transactions(
        &mut self,
        mining_reward_address: String,
    ) -> Result<(), BlockchainError> {
        let reward_tx =
            Transaction::new_reward(mining_reward_address, self.mining_reward.as_coins())
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
                    // subtract
                    balance = balance
                        .checked_sub(tx.amount())
                        .unwrap_or(Amount::from_satoshis(0));
                }
                if tx.to_address() == address {
                    // add
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
                    .checked_sub(tx.amount())
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
            let available = self.get_available_balance(tx.from_address());

            if tx.amount() > available {
                return Err(BlockchainError::InvalidTransaction(format!(
                    "insufficient funds: has {} coins, trying to send {} coins",
                    available,
                    tx.amount()
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

    // validate the entire blockchain
    pub fn is_chain_valid(&self) -> bool {
        // skip genesis block (index 0), start from index 1
        for i in 1..self.chain.len() {
            let current_block = &self.chain[i];
            let previous_block = &self.chain[i - 1];

            // check if all transactions in the block are valid
            if !current_block.has_valid_transactions() {
                println!("Block {} has invalid transactions", i);
                return false;
            }

            // check if the block's hash is valid
            if !current_block.validate_block() {
                println!("Block {} hash is invalid", i);
                return false;
            }

            // check if the previous hash matches
            if current_block.previous_hash() != previous_block.hash() {
                println!(
                    "Block {} previous hash doesn't match block {} hash",
                    i,
                    i - 1
                );
                return false;
            }

            // validates timestamp
            let previous_blocks = &self.chain[0..i];
            if let Err(e) = current_block.validate_timestamp(previous_blocks) {
                println!("Block {} has invalid timestamp: {}", i, e);
                return false;
            }
        }

        true
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
