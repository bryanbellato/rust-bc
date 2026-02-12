mod block;
mod blockchain;
mod currency;
mod keypair;
mod transaction;

use blockchain::Blockchain;
use keypair::KeyPair;
use transaction::Transaction;

fn demonstrate_merkle_proof(bc: &blockchain::Blockchain, block_index: usize, tx_index: usize) {
    if block_index >= bc.chain().len() {
        println!("Block index out of range for merkle demo.");
        return;
    }
    let block = &bc.chain()[block_index];
    if tx_index >= block.transactions().len() {
        println!("Tx index out of range for merkle demo.");
        return;
    }

    println!(
        "\nMerkle Proof Verification (Block {}, Tx {}) ",
        block_index, tx_index
    );
    println!(
        "Merkle Root: {}...",
        &hex::encode(block.merkle_root())[..20]
    );

    match block.get_transaction_proof(tx_index) {
        Ok(proof) => {
            println!("Proof Path ({} hashes):", proof.len());
            for (i, hash) in proof.iter().enumerate() {
                println!(" L{}: {}...", i, &hex::encode(hash)[..20]);
            }

            let is_valid =
                block.verify_transaction_proof(&block.transactions()[tx_index], tx_index, &proof);
            println!(
                "Verification Result: {}",
                if is_valid { " VALID" } else { " INVALID" }
            );
        }
        Err(e) => println!("Error: {}", e),
    }
}

fn demonstrate_spv_proof(bc: &blockchain::Blockchain, block_index: usize, tx_index: usize) {
    println!("\n SPV Proof (Light Client) ");

    if block_index >= bc.chain().len() {
        println!("Block index out of range for SPV demo.");
        return;
    }
    let block = &bc.chain()[block_index];

    match block.create_spv_proof(tx_index) {
        Ok(spv) => {
            println!("Target Block Hash: {}...", &spv.block_hash[..20]);
            println!("Tx Amount: {} coins", spv.tx.amount());
            let is_valid = spv.verify();
            println!("SPV verification successful: {}", is_valid);
        }
        Err(e) => println!("Failed to create SPV proof: {}", e),
    }
}

fn demonstrate_invalid_proof(bc: &blockchain::Blockchain) {
    println!("\n Invalid Proof Detection ");

    if bc.chain().len() <= 1 {
        println!("Not enough blocks for invalid proof demo.");
        return;
    }
    let block = &bc.chain()[1];
    if block.transactions().len() < 2 {
        println!("Block doesn't have enough transactions for invalid proof demo.");
        return;
    }

    let proof = block.get_transaction_proof(0).unwrap();
    let wrong_tx = &block.transactions()[1];
    let is_valid = block.verify_transaction_proof(wrong_tx, 0, &proof);

    println!("Attempting to verify Tx #1 using proof for Tx #0...");
    println!(
        "Result: {}",
        if is_valid {
            " PASSED (SECURITY FAILURE)"
        } else {
            " ATTACK FAILED (SECURITY OK)"
        }
    );
}

mod merkle {
    use sha2::{Digest, Sha256};

    pub type MerkleRoot = [u8; 32];
    pub type MerkleProof = Vec<[u8; 32]>;

    fn double_sha256(data: &[u8]) -> [u8; 32] {
        let first = Sha256::digest(data);
        let second = Sha256::digest(first);
        second.into()
    }

    fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
        let mut combined = Vec::with_capacity(64);
        combined.extend_from_slice(left);
        combined.extend_from_slice(right);
        double_sha256(&combined)
    }

    pub fn build_merkle_tree(tx_hashes: &[[u8; 32]]) -> (MerkleRoot, Vec<Vec<[u8; 32]>>) {
        if tx_hashes.is_empty() {
            return ([0u8; 32], vec![]);
        }
        if tx_hashes.len() == 1 {
            return (tx_hashes[0], vec![vec![tx_hashes[0]]]);
        }

        let mut tree: Vec<Vec<[u8; 32]>> = Vec::new();
        let mut current_level = tx_hashes.to_vec();
        tree.push(current_level.clone());

        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            for i in (0..current_level.len()).step_by(2) {
                let left = current_level[i];
                let right = if i + 1 < current_level.len() {
                    current_level[i + 1]
                } else {
                    current_level[i]
                };
                next_level.push(hash_pair(&left, &right));
            }
            tree.push(next_level.clone());
            current_level = next_level;
        }
        (current_level[0], tree)
    }

    pub fn generate_proof(tree: &[Vec<[u8; 32]>], tx_index: usize) -> Result<MerkleProof, String> {
        if tree.is_empty() {
            return Err("Empty tree".to_string());
        }
        if tx_index >= tree[0].len() {
            return Err("Transaction index out of bounds".to_string());
        }

        let mut proof = Vec::new();
        let mut index = tx_index;
        for level in tree.iter().take(tree.len() - 1) {
            let sibling_index = if index % 2 == 0 {
                if index + 1 < level.len() {
                    index + 1
                } else {
                    index
                }
            } else {
                index - 1
            };
            proof.push(level[sibling_index]);
            index /= 2;
        }
        Ok(proof)
    }

    pub fn verify_proof(
        tx_hash: &[u8; 32],
        tx_index: usize,
        proof: &[[u8; 32]],
        merkle_root: &[u8; 32],
    ) -> bool {
        let mut current_hash = *tx_hash;
        let mut index = tx_index;
        for sibling in proof {
            current_hash = if index % 2 == 0 {
                hash_pair(&current_hash, sibling)
            } else {
                hash_pair(sibling, &current_hash)
            };
            index /= 2;
        }
        current_hash == *merkle_root
    }

    pub fn calculate_merkle_root(tx_hashes: &[[u8; 32]]) -> MerkleRoot {
        if tx_hashes.is_empty() {
            return [0u8; 32];
        }
        if tx_hashes.len() == 1 {
            return tx_hashes[0];
        }
        let mut current_level = tx_hashes.to_vec();
        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            for i in (0..current_level.len()).step_by(2) {
                let left = current_level[i];
                let right = if i + 1 < current_level.len() {
                    current_level[i + 1]
                } else {
                    current_level[i]
                };
                next_level.push(hash_pair(&left, &right));
            }
            current_level = next_level;
        }
        current_level[0]
    }

    pub fn print_merkle_tree(tree: &[Vec<[u8; 32]>]) {
        if tree.is_empty() {
            println!("Empty Merkle Tree");
            return;
        }
        let root_level = tree.len() - 1;
        println!("\n Merkle Tree Visualization ");
        for (level_idx, level) in tree.iter().enumerate().rev() {
            let label = if level_idx == root_level {
                "ROOT"
            } else if level_idx == 0 {
                "LEAVES (Transactions)"
            } else {
                "INTERMEDIATE"
            };
            println!("Level {} [{}]:", level_idx, label);
            for (item_idx, hash) in level.iter().enumerate() {
                let full_hex = hex::encode(hash);
                let short_hex = &full_hex[..16];
                println!(" Node {}: {}...", item_idx, short_hex);
            }
            println!("--------------------------------");
        }
        println!("=================================\n");
    }
}

fn main() {
    println!(" Generating 11 Wallets \n");

    let miner_keypair = keypair::KeyPair::generate().expect("Failed to generate miner keypair");
    let miner_address = miner_keypair.get_public_key().to_string();
    println!("Miner/Genesis : {}...", &miner_address[..20]);

    let mut wallets = Vec::new();
    let mut wallet_addresses = Vec::new();

    for i in 1..=11 {
        let kp = keypair::KeyPair::generate().expect("Failed to generate keypair");
        let addr = kp.get_public_key().to_string();
        println!("Wallet {:02} : {}...", i, &addr[..20]);
        wallets.push(kp);
        wallet_addresses.push(addr);
    }

    println!("\n Blockchain Initialization ");
    let mut bc = blockchain::Blockchain::new(5, 100.0, 10, miner_address.clone());

    println!("Genesis Block created.");
    println!("Miner Balance: {} coins\n", bc.get_balance(&miner_address));

    for i in 0..5 {
        let mut tx =
            transaction::Transaction::new(&miner_address, &wallet_addresses[i], 10.0, 0.001)
                .expect("Failed to create tx");
        tx.sign(miner_keypair.get_private_key()).unwrap();
        bc.add_transaction(tx).expect("Failed to add tx");
    }

    println!("Mining Block 1...");
    bc.mine_pending_transactions(miner_address.clone()).unwrap();

    if let Ok(block) = bc.get_latest_block() {
        block.print_merkle_tree();
    }

    for i in 0..5 {
        let mut tx = transaction::Transaction::new(
            &wallet_addresses[i],
            &wallet_addresses[i + 5],
            5.0,
            0.001,
        )
        .expect("Failed to create tx");

        tx.sign(wallets[i].get_private_key()).unwrap();
        bc.add_transaction(tx).expect("Failed to add tx");
    }

    let mut tx_noise =
        transaction::Transaction::new(&miner_address, &wallet_addresses[10], 2.0, 0.001).unwrap();
    tx_noise.sign(miner_keypair.get_private_key()).unwrap();
    bc.add_transaction(tx_noise).unwrap();

    println!("Mining Block 2 (6 Transactions)...");
    bc.mine_pending_transactions(miner_address.clone()).unwrap();

    if let Ok(block) = bc.get_latest_block() {
        block.print_merkle_tree();
    }

    for i in 0..5 {
        let sender_idx = i + 5;
        let receiver_idx = i;

        let mut tx = transaction::Transaction::new(
            &wallet_addresses[sender_idx],
            &wallet_addresses[receiver_idx],
            1.0,
            0.001,
        )
        .unwrap();
        tx.sign(wallets[sender_idx].get_private_key()).unwrap();
        bc.add_transaction(tx).unwrap();
    }

    let w11_idx = 10;
    for i in 0..3 {
        let mut tx = transaction::Transaction::new(
            &wallet_addresses[w11_idx],
            &wallet_addresses[i],
            0.5,
            0.001,
        )
        .unwrap();
        tx.sign(wallets[w11_idx].get_private_key()).unwrap();
        bc.add_transaction(tx).unwrap();
    }

    println!("Mining Block 3 (8 Transactions)...");
    bc.mine_pending_transactions(miner_address.clone()).unwrap();

    if let Ok(block) = bc.get_latest_block() {
        block.print_merkle_tree();
    }

    demonstrate_merkle_proof(&bc, 2, 2);
    demonstrate_spv_proof(&bc, 3, 0);
    demonstrate_invalid_proof(&bc);

    println!(
        "Miner: {:.8} coins",
        bc.get_balance(&miner_address).as_coins()
    );
    for i in 0..11 {
        println!(
            "Wallet {:02}: {:.8} coins",
            i + 1,
            bc.get_balance(&wallet_addresses[i]).as_coins()
        );
    }

    println!("\nBlockchain valid: {}", bc.is_chain_valid());
}
