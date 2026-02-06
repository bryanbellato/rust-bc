use crate::block::Block;
use crate::transaction::Transaction;
use std::fmt;

#[derive(Debug, Clone)]
pub struct Blockchain {
    chain: Vec<Block>,
    difficulty: usize,
    pending_transactions: Vec<Transaction>,
    mining_reward: f64,
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
    pub fn new(difficulty: usize, mining_reward: f64, miner_address: String) -> Self {
        let genesis_block = Self::create_genesis_block(miner_address);

        Blockchain {
            chain: vec![genesis_block],
            difficulty,
            pending_transactions: Vec::new(),
            mining_reward,
        }
    }

    fn create_genesis_block(miner_address: String) -> Block {
        let genesis_transaction = Transaction::new_reward(miner_address, 100.0)
            .expect("Failed to create genesis transaction");

        Block::new(vec![genesis_transaction], "0".to_string())
    }

    pub fn get_latest_block(&self) -> Result<&Block, BlockchainError> {
        self.chain.last().ok_or(BlockchainError::NoBlocks)
    }

    pub fn mine_pending_transactions(
        &mut self,
        mining_reward_address: String,
    ) -> Result<(), BlockchainError> {
        let reward_tx = Transaction::new_reward(mining_reward_address, self.mining_reward)
            .map_err(|e| BlockchainError::InvalidTransaction(e.to_string()))?;

        self.pending_transactions.push(reward_tx);

        let previous_hash = self.get_latest_block()?.hash().to_string();
        let mut new_block = Block::new(self.pending_transactions.clone(), previous_hash);

        new_block.mine_block(self.difficulty);

        self.chain.push(new_block);

        self.pending_transactions.clear();

        Ok(())
    }

    /// add a transaction to the pending transactions pool
    pub fn add_transaction(&mut self, tx: Transaction) -> Result<(), BlockchainError> {
        if tx.from_address().is_empty() || tx.to_address().is_empty() {
            return Err(BlockchainError::EmptyAddress);
        }

        if !tx.from_address().is_empty() {
            let sender_balance = self.get_balance(tx.from_address());
            if tx.amount() > sender_balance {
                return Err(BlockchainError::InvalidTransaction(format!(
                    "insufficient funds: has {:.2}, trying to send {:.2}",
                    sender_balance,
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
            // Removed type annotation 'e: TransactionError'
            Err(e) => {
                return Err(BlockchainError::InvalidSignature(e.to_string()));
            }
        }

        self.pending_transactions.push(tx);

        Ok(())
    }

    /// get the balance of an address across the entire blockchain
    pub fn get_balance(&self, address: &str) -> f64 {
        let mut balance = 0.0;

        for block in &self.chain {
            for tx in block.transactions() {
                // subtract if this address is the sender
                if tx.from_address() == address {
                    balance -= tx.amount();
                }

                // add if this address is the recipient
                if tx.to_address() == address {
                    balance += tx.amount();
                }
            }
        }

        balance
    }

    /// validate the entire blockchain
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

    pub fn mining_reward(&self) -> f64 {
        self.mining_reward
    }

    pub fn print_chain(&self) {
        println!("\n=== Blockchain ===");
        println!("Difficulty: {}", self.difficulty);
        println!("Mining Reward: {}", self.mining_reward);
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
                println!("      From: {}", tx.from_address());
                println!("      To: {}", tx.to_address());
                println!("      Amount: {}", tx.amount());
                println!("      Signature: {:?}", tx.signature());
            }
        }
        println!("\n==================\n");
    }
}
