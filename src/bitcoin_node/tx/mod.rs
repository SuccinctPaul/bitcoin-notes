use bitcoin::bip32::Xpriv;
use bitcoin::key::UntweakedPublicKey;
use bitcoin::{Amount, OutPoint, ScriptBuf, TxOut, Txid};
use secp256k1::{Keypair, Secp256k1, Signing, Verification};
use std::str::FromStr;

mod presing_tx_taproot;
pub mod rbf;
pub mod sign_tx_taproot;
pub mod taproot_tree_tx;

// User BTC regtest info:
// -rpcwallet=benefactor
// Address: bcrt1phcnl4zcl2fu047pv4wx6y058v8u0n02at6lthvm7pcf2wrvjm5tqatn90k
pub const USER_A_PRIVATE_KEY: &str = "tprv8jzau9CfsdkXMzqWFWSgu7f4z1vRk53yiqYqByfoakSLNFQ4bBuTsrUDLXtKHTPZhp161h49vEJr2zwN92G7ZHLZMFvome2U8GcAqDzVRhW";
pub const USER_A_PUBLIC_KEY: &str =
    "02a6ac32163539c16b6b5dbbca01b725b8e8acaa5f821ba42c80e7940062140d19";

// BTC regtest info:
// -rpcwallet=beneficiary
// p2tr_addr: bcrt1p0p3rvwww0v9znrclp00uneq8ytre9kj922v8fxhnezm3mgsmn9usdxaefc
pub const USER_B_PRIVATE_KEY: &str = "tprv8kpW9A9EhycN2QsL8UvvfARxvd1w5aq971AKmJNsRDPWpqNX41d1kdscpK5uT9HrNG9hfLqfjFkwqRXpN7cL2EBfyvb6BZjEBACDsaJQPzW";
pub const USER_B_PUBLIC_KEY: &str =
    "03259ea961fd6bf615c7328ec9538cfc911d50c44f07cbe71bad0f9367e566cc1b";

// BTC regtest info:
// -rpcwallet=benefactor
// p2tr_addr: bcrt1pmdx8nnpllj3x750zzfqmjvedv34swuka06vda8qau6csnyx2hq9s6p89qf
pub const USER_C_PRIVATE_KEY: &str = "tprv8jzau9CfsdkXPkVBGi313RjQvsXggNwC4SZEBm3ohYAHQrHvBBG9GrPwMRWmzvB2UgkH7vEEjoMwia8kiY1jo6FzeshAfEw8d95ziJHYSTp";
pub const USER_C_PUBLIC_KEY: &str =
    "0385a34c3603c616afaa9da80ee2f354b8caf0308890193b4083cbdee09f998fd0";

// It's a absolutely receiver addr
pub const RECEIVER_ADDR_STR: &str =
    "bcrt1p3ndyc0r2s4khcqka5vpeyt00ky378nk7mr6tmmja7yu6jg24uscq064uer";

/// Creates a p2wpkh output locked to the key associated with `wpkh`.
///
/// An utxo is described by the `OutPoint` (txid and index within the transaction that it was
/// created). Using the out point one can get the transaction by `txid` and using the `vout` get the
/// transaction value and script pubkey (`TxOut`) of the utxo.
///
/// This output is locked to keys that we control, in a real application this would be a valid
/// output taken from a transaction that appears in the chain.
pub fn dummy_unspent_transaction_output<C: Verification>(
    secp: &Secp256k1<C>,
    internal_key: UntweakedPublicKey,
    txid: &str,
    vout: u32,
    amount_in_sats: Amount,
) -> (OutPoint, TxOut) {
    let script_pubkey = ScriptBuf::new_p2tr(secp, internal_key, None);

    let txid = Txid::from_str(txid).unwrap();
    let out_point = OutPoint { txid, vout };

    let utxo = TxOut {
        value: amount_in_sats,
        script_pubkey,
    };

    (out_point, utxo)
}

/// An example of keys controlled by the transaction sender.
///
/// In a real application these would be actual secrets.
pub fn senders_keys<C: Signing>(secp: &Secp256k1<C>, sk: &str) -> Keypair {
    // let sk = SecretKey::new(&mut rand::thread_rng());
    let sk = Xpriv::from_str(&sk).unwrap();
    let sk = sk.private_key;
    Keypair::from_secret_key(secp, &sk)
}

#[cfg(test)]
mod test {
    use crate::bitcoin_node::tx::{USER_A_PRIVATE_KEY, USER_A_PUBLIC_KEY, USER_B_PRIVATE_KEY};
    use crate::keygen::Keygen;
    use bitcoin::bip32::Xpriv;
    use bitcoin::{Network, PublicKey, ScriptBuf};
    use secp256k1::{Keypair, XOnlyPublicKey};
    use std::str::FromStr;

    #[test]
    fn p2tr_lock_script_test() -> anyhow::Result<()> {
        let extend_sk = Xpriv::from_str(USER_A_PRIVATE_KEY)?;
        let private_key = extend_sk.to_priv();
        let public_key = PublicKey::from_str(USER_A_PUBLIC_KEY)?;
        let address = Keygen::p2tr_addr_from_pk(public_key, Network::Regtest)?;
        let x_only_public_key: XOnlyPublicKey = public_key.into();
        let addr_script_pubkey = address.script_pubkey();

        let secp = secp256k1::Secp256k1::new();
        let keypair = Keypair::from_secret_key(&secp, &private_key.inner);
        let (internal_key, _parity) = keypair.x_only_public_key();
        let p2tr_lock_script = ScriptBuf::new_p2tr(&secp, x_only_public_key, None);
        assert_eq!(p2tr_lock_script, addr_script_pubkey);
        assert_eq!(internal_key, x_only_public_key);

        Ok(())
    }
}
