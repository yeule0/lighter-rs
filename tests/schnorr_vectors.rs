use lighter_rs::signature::schnorr::{self, Signature};
use lighter_rs::curve::scalar::Scalar;
use lighter_rs::field::quintic::Fp5;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct SchnorrVector {
    name: String,
    sk: String,
    pk: String,
    msg: String,
    nonce: String,
    sig: String,
}

fn scalar_from_hex(s: &str) -> Scalar {
    let bytes = hex::decode(s).unwrap();
    let arr: [u8; 40] = bytes.try_into().unwrap();
    Scalar::from_bytes_le(&arr)
}

fn fp5_from_hex(s: &str) -> Fp5 {
    let bytes = hex::decode(s).unwrap();
    Fp5::from_bytes_le(&bytes).unwrap()
}

#[test]
fn cross_validate_schnorr_against_go() {
    let data = include_str!("schnorr_go_vectors.json");
    let vectors: Vec<SchnorrVector> = serde_json::from_str(data).unwrap();

    for v in &vectors {
        let sk = scalar_from_hex(&v.sk);
        let pk = fp5_from_hex(&v.pk);
        let msg = fp5_from_hex(&v.msg);
        let nonce = scalar_from_hex(&v.nonce);

        // Test public key derivation
        let pk_derived = schnorr::public_key_from_secret(&sk);
        assert_eq!(pk_derived.to_bytes_le().to_vec(), pk.to_bytes_le().to_vec(),
            "{}: pk derivation mismatch", v.name);

        // Test signing with known nonce (deterministic)
        let sig = schnorr::sign_with_nonce(&msg, &sk, &nonce);
        let expected_sig_bytes = hex::decode(&v.sig).unwrap();
        assert_eq!(sig.to_bytes().to_vec(), expected_sig_bytes,
            "{}: signature mismatch", v.name);

        // Test verification
        let sig_from_bytes = Signature::from_bytes(&expected_sig_bytes).unwrap();
        assert!(schnorr::verify(&pk, &msg, &sig_from_bytes),
            "{}: verification failed", v.name);
    }
}
