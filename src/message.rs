use crate::block::Block;
use crate::transaction::Transaction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum Message {
    Ping { port: u16 },
    Pong { port: u16 },

    NewBlock(Block),
    RequestBlock { index: usize },
    Block(Block),
    NewTransaction(Transaction),

    RequestChain,
    ChainResponse { blocks: Vec<Block>, length: usize },

    Inventory { block_hashes: Vec<String> },
    RequestInventory,
}
