use lighter_rs::curve::scalar::Scalar;
use lighter_rs::curve::point::Point;
use lighter_rs::field::quintic::Fp5;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CurveVector {
    name: String,
    op: String,
    #[serde(default)]
    scalar_a: String,
    #[serde(default)]
    scalar_b: String,
    #[serde(default)]
    scalar_r: String,
    #[serde(default)]
    encoded: String,
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
fn cross_validate_curve_against_go() {
    let data = include_str!("curve_go_vectors.json");
    let vectors: Vec<CurveVector> = serde_json::from_str(data).unwrap();

    for v in &vectors {
        match v.op.as_str() {
            "scalar_add" => {
                let a = scalar_from_hex(&v.scalar_a);
                let b = scalar_from_hex(&v.scalar_b);
                let r = a.add(&b);
                assert_eq!(r.to_bytes_le().to_vec(), hex::decode(&v.scalar_r).unwrap(),
                    "{}: scalar add mismatch", v.name);
            }
            "scalar_mul" => {
                let a = scalar_from_hex(&v.scalar_a);
                let b = scalar_from_hex(&v.scalar_b);
                let r = a.mul(&b);
                assert_eq!(r.to_bytes_le().to_vec(), hex::decode(&v.scalar_r).unwrap(),
                    "{}: scalar mul mismatch", v.name);
            }
            "scalar_sub" => {
                let a = scalar_from_hex(&v.scalar_a);
                let b = scalar_from_hex(&v.scalar_b);
                let r = a.sub(&b);
                assert_eq!(r.to_bytes_le().to_vec(), hex::decode(&v.scalar_r).unwrap(),
                    "{}: scalar sub mismatch", v.name);
            }
            "encode" | "decode" | "mul_enc" | "double_enc" | "add_enc" => {
                // Point operations: verify encode output
                let expected_enc = fp5_from_hex(&v.encoded);
                let expected_bytes = expected_enc.to_bytes_le().to_vec();

                // Reconstruct point: decode the encoding
                let (point, ok) = Point::decode(&expected_enc);
                assert!(ok, "{}: decode failed", v.name);

                // Re-encode and verify it matches
                let re_enc = point.encode();
                let re_bytes = re_enc.to_bytes_le().to_vec();
                assert_eq!(re_bytes, expected_bytes,
                    "{}: encode/decode roundtrip mismatch", v.name);
            }
            _ => {}
        }
    }
}
