mod block;
mod blockchain;
mod currency;
mod keypair;
mod transaction;

use blockchain::Blockchain;
use keypair::KeyPair;
use transaction::Transaction;

fn main() {
    println!("\n Creating Wallets: \n");

    let miner_keypair = KeyPair::generate().expect("Failed to generate miner keypair");
    println!("Miner (Genesis) Key Pair:");
    miner_keypair.print();
    let miner_address = miner_keypair.get_public_key().to_string();

    let recipient_a_keypair = KeyPair::generate().expect("Failed to generate recipient A keypair");
    println!("Recipient A Key Pair:");
    recipient_a_keypair.print();
    let recipient_a_address = recipient_a_keypair.get_public_key().to_string();

    let wallet_c_keypair = KeyPair::generate().expect("Failed to generate wallet C keypair");
    println!("Wallet C Key Pair:");
    wallet_c_keypair.print();
    let wallet_c_address = wallet_c_keypair.get_public_key().to_string();

    let wallet_d_keypair = KeyPair::generate().expect("Failed to generate wallet D keypair");
    println!("Wallet D Key Pair:");
    wallet_d_keypair.print();
    let wallet_d_address = wallet_d_keypair.get_public_key().to_string();

    println!("\n Creating Blockchain: \n");
    let mut bc = Blockchain::new(5, 100.0, 10, miner_address.clone());

    println!(
        "Genesis balance (Miner): {} coins\n",
        bc.get_balance(&miner_address)
    );

    println!("\n Transaction 1: Miner → Recipient A (50 coins) \n");

    let mut tx1 = Transaction::new(&miner_address, &recipient_a_address, 50.0, 0.001)
        .expect("Failed to create tx1");
    tx1.sign(miner_keypair.get_private_key())
        .expect("Failed to sign tx1");

    bc.add_transaction(tx1).expect("Failed to add tx1");
    println!("Transaction added to mempool");

    println!("\n Mining block 1...\n");
    bc.mine_pending_transactions(miner_address.clone())
        .expect("Failed to mine block 1");

    println!("\n Balances after block 1:");
    println!("  Miner: {} coins", bc.get_balance(&miner_address));
    println!(
        "  Recipient A: {} coins",
        bc.get_balance(&recipient_a_address)
    );

    println!("\n Wallet C mines a block (earns mining reward)\n");

    // dummy transaction
    let mut tx2 = Transaction::new(&miner_address, &recipient_a_address, 10.0, 0.001)
        .expect("Failed to create tx2");
    tx2.sign(miner_keypair.get_private_key())
        .expect("Failed to sign tx2");
    bc.add_transaction(tx2).expect("Failed to add tx2");

    println!("Wallet C mining block 2...\n");
    bc.mine_pending_transactions(wallet_c_address.clone())
        .expect("Failed to mine block 2");

    println!("\n Balances after block 2:");
    println!("  Miner: {} coins", bc.get_balance(&miner_address));
    println!(
        "  Recipient A: {} coins",
        bc.get_balance(&recipient_a_address)
    );
    println!("  Wallet C: {} coins", bc.get_balance(&wallet_c_address));

    println!("\nTransaction 3: Wallet C → Wallet D (30 coins)\n");

    let mut tx3 = Transaction::new(&wallet_c_address, &wallet_d_address, 30.0, 0.001)
        .expect("Failed to create tx3");
    tx3.sign(wallet_c_keypair.get_private_key())
        .expect("Failed to sign tx3");

    bc.add_transaction(tx3).expect("Failed to add tx3");
    println!("Transaction added to mempool");

    println!("\n Mining block 3...\n");
    bc.mine_pending_transactions(miner_address.clone())
        .expect("Failed to mine block 3");

    println!("\n Balances after block 3:");
    println!("  Wallet C: {} coins", bc.get_balance(&wallet_c_address));
    println!("  Wallet D: {} coins", bc.get_balance(&wallet_d_address));

    println!("\n Transaction 4: Wallet C → Recipient A (20 coins) \n");

    let mut tx4 = Transaction::new(&wallet_c_address, &recipient_a_address, 20.0, 0.001)
        .expect("Failed to create tx4");
    tx4.sign(wallet_c_keypair.get_private_key())
        .expect("Failed to sign tx4");

    bc.add_transaction(tx4).expect("Failed to add tx4");
    println!("Transaction added to mempool");

    println!("\nMining block 4...\n");
    bc.mine_pending_transactions(wallet_c_address.clone())
        .expect("Failed to mine block 4");

    println!("\nBalances after block 4:");
    println!("  Wallet C: {} coins", bc.get_balance(&wallet_c_address));
    println!(
        "  Recipient A: {} coins",
        bc.get_balance(&recipient_a_address)
    );

    println!("\n Transaction 5: Recipient A → Wallet C (15 coins - refund) \n");

    let mut tx5 = Transaction::new(&recipient_a_address, &wallet_c_address, 15.0, 0.001)
        .expect("Failed to create tx5");
    tx5.sign(recipient_a_keypair.get_private_key())
        .expect("Failed to sign tx5");

    bc.add_transaction(tx5).expect("Failed to add tx5");
    println!("Transaction added to mempool");

    println!("\nMining block 5...\n");
    bc.mine_pending_transactions(wallet_d_address.clone())
        .expect("Failed to mine block 5");

    println!("\n Balances after block 5:");
    println!(
        "  Recipient A: {} coins",
        bc.get_balance(&recipient_a_address)
    );
    println!("  Wallet C: {} coins", bc.get_balance(&wallet_c_address));
    println!("  Wallet D: {} coins", bc.get_balance(&wallet_d_address));

    println!("\nTransaction 6: Wallet D → Miner/Genesis (25 coins) \n");

    let mut tx6 = Transaction::new(&wallet_d_address, &miner_address, 25.0, 0.001)
        .expect("Failed to create tx6");
    tx6.sign(wallet_d_keypair.get_private_key())
        .expect("Failed to sign tx6");

    bc.add_transaction(tx6).expect("Failed to add tx6");
    println!("Transaction added to mempool");

    println!("\n Mining block 6...\n");
    bc.mine_pending_transactions(miner_address.clone())
        .expect("Failed to mine block 6");

    println!("\n Final Balances:");
    println!(
        "  Miner (Genesis): {} coins",
        bc.get_balance(&miner_address)
    );
    println!(
        "  Recipient A: {} coins",
        bc.get_balance(&recipient_a_address)
    );
    println!("  Wallet C: {} coins", bc.get_balance(&wallet_c_address));
    println!("  Wallet D: {} coins", bc.get_balance(&wallet_d_address));

    println!("\n Blockchain Validation");
    let is_valid = bc.is_chain_valid();
    println!("Is blockchain valid? {}\n", is_valid);

    bc.print_chain();
}
