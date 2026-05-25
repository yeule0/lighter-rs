use lighter_rs::field::quintic::Fp5;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Fp5Vector {
    name: String,
    a: String,
    b: String,
    add: String,
    sub: String,
    mul: String,
    square: String,
    #[serde(default)]
    inv: String, // empty for zero
}

fn hex_to_fp5(hex: &str) -> Fp5 {
    let bytes = hex::decode(hex).expect("invalid hex");
    Fp5::from_bytes_le(&bytes).expect("non-canonical bytes in test vector")
}

#[test]
fn cross_validate_against_go_vectors() {
    let data = include_str!("fp5_go_vectors.json");
    let vectors: Vec<Fp5Vector> = serde_json::from_str(data).expect("failed to parse vectors");

    for v in &vectors {
        let a = hex_to_fp5(&v.a);
        let b = hex_to_fp5(&v.b);

        // add
        assert_eq!(
            a.add(&b).to_bytes_le().to_vec(),
            hex::decode(&v.add).unwrap(),
            "{}: add mismatch", v.name
        );

        // sub
        assert_eq!(
            a.sub(&b).to_bytes_le().to_vec(),
            hex::decode(&v.sub).unwrap(),
            "{}: sub mismatch", v.name
        );

        // mul
        assert_eq!(
            a.mul(&b).to_bytes_le().to_vec(),
            hex::decode(&v.mul).unwrap(),
            "{}: mul mismatch", v.name
        );

        // square (square of a)
        assert_eq!(
            a.square().to_bytes_le().to_vec(),
            hex::decode(&v.square).unwrap(),
            "{}: square mismatch", v.name
        );

        // inverse (skip zero)
        if !v.inv.is_empty() {
            assert_eq!(
                a.inverse_or_zero().to_bytes_le().to_vec(),
                hex::decode(&v.inv).unwrap(),
                "{}: inverse mismatch", v.name
            );
        }
    }
}
