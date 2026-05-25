use lighter_rs::field::goldilocks::GoldilocksField;
use lighter_rs::hash::poseidon2;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Poseidon2Vector {
    name: String,
    input: String,
    hash_out: String,
    #[serde(default)]
    fp5_hash: String, // empty for hash_n_to_one vectors
}

fn hex_to_goldilocks(bytes: &[u8]) -> GoldilocksField {
    GoldilocksField::from_bytes_le(bytes).expect("non-canonical in vector")
}

fn parse_gf_vec(hex: &str) -> Vec<GoldilocksField> {
    let data = hex::decode(hex).expect("invalid hex");
    data.chunks(8).map(hex_to_goldilocks).collect()
}

#[test]
fn cross_validate_poseidon2_against_go() {
    let data = include_str!("poseidon2_go_vectors.json");
    let vectors: Vec<Poseidon2Vector> = serde_json::from_str(data).expect("failed to parse vectors");

    for v in &vectors {
        let input = parse_gf_vec(&v.input);

        // Test hash_out (HashNoPad / HashNToHashNoPad)
        if !v.hash_out.is_empty() && v.name.starts_with("vector_") {
            let out = poseidon2::hash_no_pad(&input);
            let out_bytes = poseidon2::hash_out_to_bytes(&out);
            let expected = hex::decode(&v.hash_out).unwrap();
            assert_eq!(
                out_bytes.to_vec(),
                expected,
                "{}: hash_no_pad mismatch", v.name
            );
        }

        // Test Fp5 hash (HashToQuinticExtension)
        if !v.fp5_hash.is_empty() {
            let fp5 = poseidon2::hash_to_quintic_extension(&input);
            let fp5_bytes = fp5.to_bytes_le().to_vec();
            let expected = hex::decode(&v.fp5_hash).unwrap();
            assert_eq!(
                fp5_bytes,
                expected,
                "{}: hash_to_quintic_extension mismatch", v.name
            );
        }

        // Test hash_n_to_one (input is concatenation of two 32-byte HashOuts)
        if v.name.starts_with("hash_n_to_one_") {
            let data = hex::decode(&v.input).unwrap();
            let h1 = poseidon2::hash_out_from_bytes(&data[..32]).unwrap();
            let h2 = poseidon2::hash_out_from_bytes(&data[32..]).unwrap();
            let combined = poseidon2::hash_n_to_one(&[h1, h2]);
            let combined_bytes = poseidon2::hash_out_to_bytes(&combined);
            let expected = hex::decode(&v.hash_out).unwrap();
            assert_eq!(
                combined_bytes.to_vec(),
                expected,
                "{}: hash_n_to_one mismatch", v.name
            );
        }
    }
}
