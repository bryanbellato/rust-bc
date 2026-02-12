use sha2::{Digest, Sha256};

pub type MerkleRoot = [u8; 32];
pub type MerkleProof = Vec<[u8; 32]>;

fn double_sha256(data: &[u8]) -> [u8; 32] {
    let first_hash = Sha256::digest(data);
    let second_hash = Sha256::digest(first_hash);
    second_hash.into()
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

    for sibling_hash in proof {
        current_hash = if index % 2 == 0 {
            hash_pair(&current_hash, sibling_hash)
        } else {
            hash_pair(sibling_hash, &current_hash)
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

            println!("   Node {}: {}...", item_idx, short_hex);
        }
        println!("--------------------------------");
    }
    println!("=================================\n");
}
