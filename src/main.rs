mod block;
mod blockchain;
mod keypair;
mod transaction;

use blockchain::Blockchain;
use keypair::KeyPair;
use transaction::Transaction;

fn main() {
    let difficulty: usize = 5;
    let mining_reward: f64 = 100.0;

    let miner_keypair = match KeyPair::generate() {
        Ok(kp) => kp,
        Err(e) => {
            eprintln!("Failed to generate miner key pair: {}", e);
            return;
        }
    };

    println!("Miner Key Pair:");
    miner_keypair.print();
    let miner_address = miner_keypair.get_public_key().to_string();

    let recipient_keypair = match KeyPair::generate() {
        Ok(kp) => kp,
        Err(e) => {
            eprintln!("Failed to generate recipient key pair: {}", e);
            return;
        }
    };

    println!("Recipient Key Pair:");
    recipient_keypair.print();
    let recipient_address = recipient_keypair.get_public_key().to_string();

    // create blockchain with genesis block
    let mut bc = Blockchain::new(difficulty, mining_reward, miner_address.clone());

    let miner_balance = bc.get_balance(&miner_address);
    println!(
        "\nMiner's balance after genesis block: {:.2}\n",
        miner_balance
    );

    // create a transaction from miner to recipient
    let mut tx = match Transaction::new(&miner_address, &recipient_address, 50.0) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error creating transaction: {}", e);
            return;
        }
    };

    // sign the transaction
    if let Err(e) = tx.sign(miner_keypair.get_private_key()) {
        eprintln!("Error signing transaction: {}", e);
        return;
    }

    println!("Transaction created and signed successfully!");

    // add transaction to blockchain
    if let Err(e) = bc.add_transaction(tx) {
        eprintln!("Error adding transaction: {}", e);
        return;
    }

    println!("Transaction added to pending transactions pool.\n");

    // mine pending transactions
    println!("Starting to mine pending transactions...\n");
    if let Err(e) = bc.mine_pending_transactions(miner_address.clone()) {
        eprintln!("Error mining pending transactions: {}", e);
        return;
    }

    let miner_balance_after = bc.get_balance(&miner_address);
    let recipient_balance = bc.get_balance(&recipient_address);

    println!("\n Balances After Mining:");
    println!("Miner's balance: {:.2}", miner_balance_after);
    println!("Recipient's balance: {:.2}", recipient_balance);

    let is_valid = bc.is_chain_valid();
    println!("\n Blockchain Validation:");
    println!("Is blockchain valid? {}\n", is_valid);

    // print full blockchain details
    bc.print_chain();

    println!("\n Creating Second Transaction: \n");

    let mut tx2 = match Transaction::new(&miner_address, &recipient_address, 25.0) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error creating second transaction: {}", e);
            return;
        }
    };

    if let Err(e) = tx2.sign(miner_keypair.get_private_key()) {
        eprintln!("Error signing second transaction: {}", e);
        return;
    }

    if let Err(e) = bc.add_transaction(tx2) {
        eprintln!("Error adding second transaction: {}", e);
        return;
    }

    println!("Second transaction created and added.\n");
    println!("Mining second block...\n");

    if let Err(e) = bc.mine_pending_transactions(miner_address.clone()) {
        eprintln!("Error mining second block: {}", e);
        return;
    }

    let miner_balance_final = bc.get_balance(&miner_address);
    let recipient_balance_final = bc.get_balance(&recipient_address);

    println!("\n Final Balances: ");
    println!("Miner's final balance: {:.2}", miner_balance_final);
    println!("Recipient's final balance: {:.2}", recipient_balance_final);

    let is_valid_final = bc.is_chain_valid();
    println!("\n Final Blockchain Validation: ");
    println!("Is blockchain valid? {}\n", is_valid_final);

    bc.print_chain();
}
