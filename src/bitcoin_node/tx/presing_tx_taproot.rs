//! Demonstrate creating a transaction that spends to and from p2tr outputs.

use std::str::FromStr;

use crate::bitcoin_node::tx::{
    dummy_unspent_transaction_output, senders_keys, USER_A_PRIVATE_KEY, USER_B_PRIVATE_KEY,
    USER_B_PUBLIC_KEY, USER_C_PUBLIC_KEY,
};
use crate::keygen::Keygen;
use bitcoin::bip32::Xpriv;
use bitcoin::consensus::encode;
use bitcoin::key::{Keypair, TapTweak, TweakedKeypair, UntweakedPublicKey};
use bitcoin::locktime::absolute;
use bitcoin::secp256k1::{rand, Message, Secp256k1, SecretKey, Signing, Verification};
use bitcoin::sighash::{Prevouts, SighashCache, TapSighashType};
use bitcoin::{
    transaction, Address, Amount, Network, OutPoint, PublicKey, ScriptBuf, Sequence, Transaction,
    TxIn, TxOut, Txid, Witness,
};

// const DUMMY_UTXO_AMOUNT: Amount = Amount::from_sat(20_000_000);
const SPEND_AMOUNT: Amount = Amount::from_sat(5_000_000);
// const CHANGE_AMOUNT: Amount = Amount::from_sat(14_999_000); // 1000 sat fee.

const GAS_FEE: Amount = Amount::from_sat(1_000);

#[test]
fn test_sign_taproot_ab_to_c_with_preign_a() -> anyhow::Result<()> {
    let secp = Secp256k1::new();
    // 1. pre utxo_a
    // bitcoin-cli -regtest -rpcwallet=benefactor listunspent 99 199 '["bcrt1phcnl4zcl2fu047pv4wx6y058v8u0n02at6lthvm7pcf2wrvjm5tqatn90k"]'
    let pre_txid_a = "d29d9439ebc433dfae790f118ab23eb1871cd82d3f0f01e104fd3dde27e34a9c";
    let pre_vout_a = 0;
    let amount_in_sats_a = Amount::from_btc(25.0).unwrap();
    // pre utxo_b
    // bitcoin-cli -regtest -rpcwallet=benefactor listunspent 99 199 '["bcrt1p0p3rvwww0v9znrclp00uneq8ytre9kj922v8fxhnezm3mgsmn9usdxaefc"]'
    let pre_txid_b = "153c18e665096e0859533a6adf71e61c1d780976fbc3d217d046ba60b0637c7d";
    let pre_vout_b = 2;
    let amount_in_sats_b = Amount::from_btc(24.99999000).unwrap();
    ///////////////////////////////////////////////////
    ////////// sign a
    ///////////////////////////////////////////////////
    // Get a keypair we control. In a real application these would come from a stored secret.
    let keypair_a = senders_keys(&secp, USER_A_PRIVATE_KEY);
    let (internal_key_a, _parity) = keypair_a.x_only_public_key();
    let (dummy_out_point_a, dummy_utxo_a) = dummy_unspent_transaction_output(
        &secp,
        internal_key_a,
        pre_txid_a,
        pre_vout_a,
        amount_in_sats_a,
    );

    // 2. Get an address to send to.
    let receiver_pk_c = PublicKey::from_str(USER_C_PUBLIC_KEY)?;
    let receiver_address_c = Keygen::p2tr_addr_from_pk(receiver_pk_c, Network::Regtest)?;

    // 3. consturct txin & change
    // The input for the transaction we are constructing.
    let input_a = TxIn {
        previous_output: dummy_out_point_a, // The dummy output we are spending.
        script_sig: ScriptBuf::default(),   // For a p2tr script_sig is empty.
        sequence: Sequence::MAX,
        witness: Witness::default(), // Filled in after signing.
    };
    // user.a pay spend,
    let change_a = TxOut {
        value: dummy_utxo_a.value.unchecked_sub(SPEND_AMOUNT), // to c,
        script_pubkey: ScriptBuf::new_p2tr(&secp, internal_key_a, None), // Change comes back to us.
    };

    // The transaction we want to sign and broadcast.
    let mut unsigned_tx = Transaction {
        version: transaction::Version::TWO,  // Post BIP-68.
        lock_time: absolute::LockTime::ZERO, // Ignore the locktime.
        input: vec![input_a],                // Input goes into index 0.
        output: vec![change_a],              // Outputs, order does not matter.
    };

    let prevouts = vec![dummy_utxo_a];
    let prevouts = Prevouts::All(&prevouts);

    // 5. sign
    let mut sighasher = SighashCache::new(&mut unsigned_tx);
    // 5.1 signed by a
    let input_index_a = 0;
    let sighash_type = TapSighashType::SinglePlusAnyoneCanPay;
    let sighash = sighasher
        .taproot_key_spend_signature_hash(input_index_a, &prevouts, sighash_type)
        .expect("failed to construct sighash");
    // Sign the sighash using the secp256k1 library (exported by rust-bitcoin).
    let tweaked_a: TweakedKeypair = keypair_a.tap_tweak(&secp, None);
    let msg = Message::from(sighash);
    let signature_a = secp.sign_schnorr(&msg, &tweaked_a.to_inner());

    // Update the witness stack.
    let signature_a = bitcoin::taproot::Signature {
        signature: signature_a,
        sighash_type,
    };
    *sighasher.witness_mut(input_index_a).unwrap() = Witness::p2tr_key_spend(&signature_a);
    let mut presigned_tx = sighasher.into_transaction().to_owned();
    let presigned_tx_hex_str = encode::serialize_hex(&presigned_tx);
    println!("presigned_tx: {:?}", presigned_tx_hex_str);

    // 2. pre utxo_b
    // Get a keypair we control. In a real application these would come from a stored secret.
    let keypair_b = senders_keys(&secp, USER_B_PRIVATE_KEY);
    let (internal_key_b, _parity) = keypair_b.x_only_public_key();
    let (dummy_out_point_b, dummy_utxo_b) = dummy_unspent_transaction_output(
        &secp,
        internal_key_b,
        pre_txid_b,
        pre_vout_b,
        amount_in_sats_b,
    );
    // user.b pay gas
    let change_b = TxOut {
        value: dummy_utxo_b.clone().value.unchecked_sub(GAS_FEE),
        script_pubkey: ScriptBuf::new_p2tr(&secp, internal_key_b, None), // Change comes back to us.
    };
    let input_b = TxIn {
        previous_output: dummy_out_point_b, // The dummy output we are spending.
        script_sig: ScriptBuf::default(),   // For a p2tr script_sig is empty.
        sequence: Sequence::MAX,
        witness: Witness::default(), // Filled in after signing.
    };
    // The spend output is locked to a key controlled by the receiver.
    let spend_c = TxOut {
        value: SPEND_AMOUNT,
        script_pubkey: receiver_address_c.script_pubkey(),
    };
    presigned_tx.input.push(input_b);
    presigned_tx.output.extend(vec![change_b, spend_c]);

    let prevouts_b = vec![dummy_utxo_b; 2];
    let prevouts_b = Prevouts::All(&prevouts_b);
    // 5.2 signed by b
    let input_index_b = 1;
    // let sighash_type = TapSighashType::AllPlusAnyoneCanPay;// this work also
    let mut sighasher = SighashCache::new(&mut presigned_tx);
    let sighash_type_b = TapSighashType::SinglePlusAnyoneCanPay;
    let sighash = sighasher
        .taproot_key_spend_signature_hash(input_index_b, &prevouts_b, sighash_type_b)
        .expect("failed to construct sighash");
    // Sign the sighash using the secp256k1 library (exported by rust-bitcoin).
    let tweaked_b: TweakedKeypair = keypair_b.tap_tweak(&secp, None);
    let msg = Message::from(sighash);
    let signature_b = secp.sign_schnorr(&msg, &tweaked_b.to_inner());

    // Update the witness stack.
    let signature_b = bitcoin::taproot::Signature {
        signature: signature_b,
        sighash_type,
    };
    *sighasher.witness_mut(input_index_b).unwrap() = Witness::p2tr_key_spend(&signature_b);

    // Get the signed transaction.
    let tx = sighasher.into_transaction();
    let txid = tx.compute_txid();
    let tx_hex_str = encode::serialize_hex(&tx);

    // BOOM! Transaction signed and ready to broadcast.
    println!("tx_id {:?}", txid);
    println!("tx_str {:?}", tx_hex_str);
    // bitcoin-cli -rpcwallet=benefactor sendrawtransaction xxxx
    //
    Ok(())
}

#[test]
fn test_sign_taproot_ab_to_c_with_presigned_ab() -> anyhow::Result<()> {
    let secp = Secp256k1::new();

    // 1. pre utxo_a
    // bitcoin-cli -regtest -rpcwallet=benefactor listunspent 99 199 '["bcrt1phcnl4zcl2fu047pv4wx6y058v8u0n02at6lthvm7pcf2wrvjm5tqatn90k"]'
    let pre_txid_a = "4a4ee7ba06f2edf8a70a4a3d9100297f67540afac1f9a070439e46d0aff9db06";
    let pre_vout_a = 0;
    let amount_in_sats_a = Amount::from_btc(25.0).unwrap();
    // Get a keypair we control. In a real application these would come from a stored secret.
    let keypair_a = senders_keys(&secp, USER_A_PRIVATE_KEY);
    let (internal_key_a, _parity) = keypair_a.x_only_public_key();
    let (dummy_out_point_a, dummy_utxo_a) = dummy_unspent_transaction_output(
        &secp,
        internal_key_a,
        pre_txid_a,
        pre_vout_a,
        amount_in_sats_a,
    );

    // 2. pre utxo_b
    // bitcoin-cli -regtest -rpcwallet=benefactor listunspent 99 199 '["bcrt1p0p3rvwww0v9znrclp00uneq8ytre9kj922v8fxhnezm3mgsmn9usdxaefc"]'
    let pre_txid_b = "cc7a307941737448f63956ca1587768dce4dff1d2dc372a73a60145b79528bfd";
    let pre_vout_b = 0;
    let amount_in_sats_b = Amount::from_btc(12.5).unwrap();
    // Get a keypair we control. In a real application these would come from a stored secret.
    let keypair_b = senders_keys(&secp, USER_B_PRIVATE_KEY);
    let (internal_key_b, _parity) = keypair_b.x_only_public_key();
    let (dummy_out_point_b, dummy_utxo_b) = dummy_unspent_transaction_output(
        &secp,
        internal_key_b,
        pre_txid_b,
        pre_vout_b,
        amount_in_sats_b,
    );

    // 3. Get an address to send to.
    let receiver_pk_c = PublicKey::from_str(USER_C_PUBLIC_KEY)?;
    let receiver_address_c = Keygen::p2tr_addr_from_pk(receiver_pk_c, Network::Regtest)?;

    ///////////////////////////////////////////////
    ///////// 1.construct presign tx by A & B
    ///////////////////////////////////////////////
    // 4. consturct txin & change
    // The input for the transaction we are constructing.
    let input_a = TxIn {
        previous_output: dummy_out_point_a, // The dummy output we are spending.
        script_sig: ScriptBuf::default(),   // For a p2tr script_sig is empty.
        sequence: Sequence::MAX,
        witness: Witness::default(), // Filled in after signing.
    };
    let input_b = TxIn {
        previous_output: dummy_out_point_b, // The dummy output we are spending.
        script_sig: ScriptBuf::default(),   // For a p2tr script_sig is empty.
        sequence: Sequence::MAX,
        witness: Witness::default(), // Filled in after signing.
    };

    // user.a pay spend,
    let change_a = TxOut {
        value: dummy_utxo_a.value.unchecked_sub(SPEND_AMOUNT), // to c,
        script_pubkey: ScriptBuf::new_p2tr(&secp, internal_key_a, None), // Change comes back to us.
    };
    // user.b pay gas
    let change_b = TxOut {
        value: dummy_utxo_b.value.unchecked_sub(GAS_FEE),
        script_pubkey: ScriptBuf::new_p2tr(&secp, internal_key_b, None), // Change comes back to us.
    };

    // The transaction we want to sign and broadcast.
    let mut unsigned_tx = Transaction {
        version: transaction::Version::TWO,  // Post BIP-68.
        lock_time: absolute::LockTime::ZERO, // Ignore the locktime.
        input: vec![input_a, input_b],       // Input goes into index 0.
        output: vec![change_a, change_b],    // Outputs, order does not matter.
    };

    let prevouts = vec![dummy_utxo_a, dummy_utxo_b];
    let prevouts = Prevouts::All(&prevouts);

    // 5. sign
    let mut sighasher = SighashCache::new(&mut unsigned_tx);
    // 5.1 signed by a
    let input_index_a = 0;
    let sighash_type = TapSighashType::SinglePlusAnyoneCanPay;
    let sighash = sighasher
        .taproot_key_spend_signature_hash(input_index_a, &prevouts, sighash_type)
        .expect("failed to construct sighash");
    // Sign the sighash using the secp256k1 library (exported by rust-bitcoin).
    let tweaked_a: TweakedKeypair = keypair_a.tap_tweak(&secp, None);
    let msg = Message::from(sighash);
    let signature_a = secp.sign_schnorr(&msg, &tweaked_a.to_inner());

    // Update the witness stack.
    let signature_a = bitcoin::taproot::Signature {
        signature: signature_a,
        sighash_type,
    };
    *sighasher.witness_mut(input_index_a).unwrap() = Witness::p2tr_key_spend(&signature_a);

    // 5.2 signed by b
    let input_index_b = 1;

    // let sighash_type = TapSighashType::AllPlusAnyoneCanPay;// this work also
    let sighash_type = TapSighashType::SinglePlusAnyoneCanPay;
    let sighash = sighasher
        .taproot_key_spend_signature_hash(input_index_b, &prevouts, sighash_type)
        .expect("failed to construct sighash");
    // Sign the sighash using the secp256k1 library (exported by rust-bitcoin).
    let tweaked_b: TweakedKeypair = keypair_b.tap_tweak(&secp, None);
    let msg = Message::from(sighash);
    let signature_b = secp.sign_schnorr(&msg, &tweaked_b.to_inner());

    // Update the witness stack.
    let signature_b = bitcoin::taproot::Signature {
        signature: signature_b,
        sighash_type,
    };
    *sighasher.witness_mut(input_index_b).unwrap() = Witness::p2tr_key_spend(&signature_b);
    let mut presigned_tx = sighasher.into_transaction().to_owned();

    ///////////////////////////////////////////////
    ///////// construct spent output.
    ///////////////////////////////////////////////

    // The spend output is locked to a key controlled by the receiver.
    let spend_c = TxOut {
        value: SPEND_AMOUNT,
        script_pubkey: receiver_address_c.script_pubkey(),
    };

    presigned_tx.output.push(spend_c);
    let tx = presigned_tx;
    // Get the signed transaction.
    let txid = tx.compute_txid();
    let tx_hex_str = encode::serialize_hex(&tx);

    // BOOM! Transaction signed and ready to broadcast.
    println!("tx_id {:?}", txid);
    println!("tx_str {:?}", tx_hex_str);

    Ok(())
}

#[test]
fn test_sign_taproot_a_to_bc_with_presiend_a() -> anyhow::Result<()> {
    // bitcoin-cli -regtest -rpcwallet=benefactor listunspent 99 199 '["bcrt1phcnl4zcl2fu047pv4wx6y058v8u0n02at6lthvm7pcf2wrvjm5tqatn90k"]'
    let pre_txid = "35dc86d506b74d091984061403098eb7fc9a0d569dc9a80290e2db46ffc0671a";
    let pre_vout = 0;
    let amount_in_sats = Amount::from_btc(25.0).unwrap();
    ///////////////////////////////////////////////////
    ////////// sign a
    ///////////////////////////////////////////////////
    let secp = Secp256k1::new();
    // Get a keypair we control. In a real application these would come from a stored secret.
    let keypair = senders_keys(&secp, USER_A_PRIVATE_KEY);
    let (internal_key, _parity) = keypair.x_only_public_key();
    // let sender_address = Keygen::p2tr_addr_from_pk(*keypair.public_key(), Network::Regtest)?;

    // Get an unspent output that is locked to the key above that we control.
    // In a real application these would come from the chain.
    let (dummy_out_point, dummy_utxo) =
        dummy_unspent_transaction_output(&secp, internal_key, pre_txid, pre_vout, amount_in_sats);

    // The input for the transaction we are constructing.
    let input = TxIn {
        previous_output: dummy_out_point, // The dummy output we are spending.
        script_sig: ScriptBuf::default(), // For a p2tr script_sig is empty.
        sequence: Sequence::MAX,
        witness: Witness::default(), // Filled in after signing.
    };

    // The change output is locked to a key controlled by us.
    let change = TxOut {
        value: dummy_utxo
            .value
            .unchecked_sub(SPEND_AMOUNT) // to b
            .unchecked_sub(SPEND_AMOUNT) // to c
            .unchecked_sub(GAS_FEE),
        script_pubkey: ScriptBuf::new_p2tr(&secp, internal_key, None), // Change comes back to us.
    };

    // The transaction we want to sign and broadcast.
    let mut unsigned_tx = Transaction {
        version: transaction::Version::TWO,  // Post BIP-68.
        lock_time: absolute::LockTime::ZERO, // Ignore the locktime.
        input: vec![input],                  // Input goes into index 0.
        output: vec![change],                // Outputs, order does not matter.
    };
    let input_index = 0;

    // Get the sighash to sign.

    let sighash_type = TapSighashType::SinglePlusAnyoneCanPay;
    let prevouts = vec![dummy_utxo];
    let prevouts = Prevouts::All(&prevouts);

    let mut sighasher = SighashCache::new(&mut unsigned_tx);
    let sighash = sighasher
        .taproot_key_spend_signature_hash(input_index, &prevouts, sighash_type)
        .expect("failed to construct sighash");

    // Sign the sighash using the secp256k1 library (exported by rust-bitcoin).
    let tweaked: TweakedKeypair = keypair.tap_tweak(&secp, None);
    let msg = Message::from(sighash);
    let signature = secp.sign_schnorr(&msg, &tweaked.to_inner());

    // Update the witness stack.
    let signature = bitcoin::taproot::Signature {
        signature,
        sighash_type,
    };
    *sighasher.witness_mut(input_index).unwrap() = Witness::p2tr_key_spend(&signature);
    let mut presigned_tx = sighasher.into_transaction().to_owned();

    ///////////////////////////////////////////////
    ///////// construct spent_b,spent_c output.
    ///////////////////////////////////////////////

    // Get an address to send to.
    let receiver_pk_b = PublicKey::from_str(USER_B_PUBLIC_KEY)?;
    let receiver_address_b = Keygen::p2tr_addr_from_pk(receiver_pk_b, Network::Regtest)?;
    // Get an address to send to.
    let receiver_pk_c = PublicKey::from_str(USER_C_PUBLIC_KEY)?;
    let receiver_address_c = Keygen::p2tr_addr_from_pk(receiver_pk_c, Network::Regtest)?;
    // The spend output is locked to a key controlled by the receiver.
    let spend_b = TxOut {
        value: SPEND_AMOUNT,
        script_pubkey: receiver_address_b.script_pubkey(),
    };
    let spend_c = TxOut {
        value: SPEND_AMOUNT,
        script_pubkey: receiver_address_c.script_pubkey(),
    };
    presigned_tx.output.extend(vec![spend_b, spend_c]);

    // Get the signed transaction.
    let tx = presigned_tx;
    let txid = tx.compute_txid();
    let tx_hex_str = encode::serialize_hex(&tx);

    // BOOM! Transaction signed and ready to broadcast.
    println!("tx_id {:?}", txid);
    println!("tx_str {:?}", tx_hex_str);

    Ok(())
}
