use crate::field::goldilocks::GoldilocksField;
use crate::field::quintic::Fp5;

// ------------------------------------------------------------------
// Constants
// ------------------------------------------------------------------

const WIDTH: usize = 12;
const RATE: usize = 8;
const ROUNDS_F: usize = 8;
const ROUNDS_F_HALF: usize = 4;
const ROUNDS_P: usize = 22;

/// 8 rounds × 12 elements each. Used in full rounds.
const EXTERNAL_CONSTANTS: [[u64; WIDTH]; ROUNDS_F] = [
    [
        15492826721047263190, 11728330187201910315, 8836021247773420868, 16777404051263952451,
        5510875212538051896, 6173089941271892285, 2927757366422211339, 10340958981325008808,
        8541987352684552425, 9739599543776434497, 15073950188101532019, 12084856431752384512,
    ],
    [
        4584713381960671270, 8807052963476652830, 54136601502601741, 4872702333905478703,
        5551030319979516287, 12889366755535460989, 16329242193178844328, 412018088475211848,
        10505784623379650541, 9758812378619434837, 7421979329386275117, 375240370024755551,
    ],
    [
        3331431125640721931, 15684937309956309981, 578521833432107983, 14379242000670861838,
        17922409828154900976, 8153494278429192257, 15904673920630731971, 11217863998460634216,
        3301540195510742136, 9937973023749922003, 3059102938155026419, 1895288289490976132,
    ],
    [
        5580912693628927540, 10064804080494788323, 9582481583369602410, 10186259561546797986,
        247426333829703916, 13193193905461376067, 6386232593701758044, 17954717245501896472,
        1531720443376282699, 2455761864255501970, 11234429217864304495, 4746959618548874102,
    ],
    [
        13571697342473846203, 17477857865056504753, 15963032953523553760, 16033593225279635898,
        14252634232868282405, 8219748254835277737, 7459165569491914711, 15855939513193752003,
        16788866461340278896, 7102224659693946577, 3024718005636976471, 13695468978618890430,
    ],
    [
        8214202050877825436, 2670727992739346204, 16259532062589659211, 11869922396257088411,
        3179482916972760137, 13525476046633427808, 3217337278042947412, 14494689598654046340,
        15837379330312175383, 8029037639801151344, 2153456285263517937, 8301106462311849241,
    ],
    [
        13294194396455217955, 17394768489610594315, 12847609130464867455, 14015739446356528640,
        5879251655839607853, 9747000124977436185, 8950393546890284269, 10765765936405694368,
        14695323910334139959, 16366254691123000864, 15292774414889043182, 10910394433429313384,
    ],
    [
        17253424460214596184, 3442854447664030446, 3005570425335613727, 10859158614900201063,
        9763230642109343539, 6647722546511515039, 909012944955815706, 18101204076790399111,
        11588128829349125809, 15863878496612806566, 5201119062417750399, 176665553780565743,
    ],
];

/// 22 partial-round constants, applied only to element 0.
const INTERNAL_CONSTANTS: [u64; ROUNDS_P] = [
    11921381764981422944, 10318423381711320787, 8291411502347000766, 229948027109387563,
    9152521390190983261, 7129306032690285515, 15395989607365232011, 8641397269074305925,
    17256848792241043600, 6046475228902245682, 12041608676381094092, 12785542378683951657,
    14546032085337914034, 3304199118235116851, 16499627707072547655, 10386478025625759321,
    13475579315436919170, 16042710511297532028, 1411266850385657080, 9024840976168649958,
    14047056970978379368, 838728605080212101,
];

/// Diagonal matrix values for the internal linear layer.
const MATRIX_DIAG_U64: [u64; WIDTH] = [
    0xc3b6c08e23ba9300, 0xd84b5de94a324fb6, 0x0d0c371c5b35b84f, 0x7964f570e7188037,
    0x5daf18bbd996604b, 0x6743bc47b9595257, 0x5528b9362c59bb70, 0xac45e25b7127b68b,
    0xa2077d7dfbb606b5, 0xf3faac6faee378ae, 0x0c6388b51545e883, 0xd27dbb6944917b60,
];

// ------------------------------------------------------------------
// Hash output type: 4 Goldilocks elements (32 bytes)
// ------------------------------------------------------------------

pub type HashOut = [GoldilocksField; 4];

pub fn empty_hash_out() -> HashOut {
    [GoldilocksField::ZERO; 4]
}

pub fn hash_out_to_bytes(h: &HashOut) -> [u8; 32] {
    let mut out = [0u8; 32];
    for i in 0..4 {
        out[i * 8..(i + 1) * 8].copy_from_slice(&h[i].to_bytes_le());
    }
    out
}

pub fn hash_out_from_bytes(bytes: &[u8]) -> Result<HashOut, &'static str> {
    if bytes.len() != 32 {
        return Err("expected 32 bytes");
    }
    let mut h = [GoldilocksField::ZERO; 4];
    for i in 0..4 {
        h[i] = GoldilocksField::from_bytes_le(&bytes[i * 8..(i + 1) * 8])?;
    }
    Ok(h)
}

// ------------------------------------------------------------------
// Core permutation — matches Go Permute in poseidon2_plonky2
// ------------------------------------------------------------------

/// Applies one Poseidon2 permutation to the 12-element state.
pub fn permute(state: &mut [GoldilocksField; WIDTH]) {
    external_linear_layer(state);
    full_rounds(state, 0);
    partial_rounds(state);
    full_rounds(state, ROUNDS_F_HALF);
}

// ------------------------------------------------------------------
// Full rounds
// ------------------------------------------------------------------

fn full_rounds(state: &mut [GoldilocksField; WIDTH], start: usize) {
    for r in start..start + ROUNDS_F_HALF {
        add_external_rc(state, r);
        sbox_all(state);
        external_linear_layer(state);
    }
}

// ------------------------------------------------------------------
// Partial rounds
// ------------------------------------------------------------------

fn partial_rounds(state: &mut [GoldilocksField; WIDTH]) {
    for r in 0..ROUNDS_P {
        add_internal_rc(state, r);
        sbox_one(state, 0);
        internal_linear_layer(state);
    }
}

// ------------------------------------------------------------------
// External linear layer — Go: externalLinearLayer + externalLinearLayer128
//
// Operates in 3 windows of 4. Uses u128 to avoid overflow during the
// MDS multiplication and column-sum redistribution.
// ------------------------------------------------------------------

fn external_linear_layer(s: &mut [GoldilocksField; WIDTH]) {
    let mut vals: [u128; WIDTH] = [0; WIDTH];
    for i in 0..WIDTH {
        vals[i] = s[i].0 as u128;
    }

    // Process 3 windows of 4 elements each (12 / 4 = 3)
    for i in (0..WIDTH).step_by(4) {
        let t01 = vals[i] + vals[i + 1];
        let t23 = vals[i + 2] + vals[i + 3];
        let t0123 = t01 + t23;

        let x0 = vals[i];
        let x1 = vals[i + 1];
        let x2 = vals[i + 2];
        let x3 = vals[i + 3];

        vals[i]     = t0123 + t01 + x1;
        vals[i + 1] = t0123 + x1 + x2 + x2;
        vals[i + 2] = t0123 + t23 + x3;
        vals[i + 3] = t0123 + x3 + x0 + x0;
    }

    // Column-sum redistribution
    let sum0 = vals[0] + vals[4] + vals[8];
    let sum1 = vals[1] + vals[5] + vals[9];
    let sum2 = vals[2] + vals[6] + vals[10];
    let sum3 = vals[3] + vals[7] + vals[11];

    for (i, v) in vals.iter_mut().enumerate() {
        *v += match i % 4 {
            0 => sum0,
            1 => sum1,
            2 => sum2,
            3 => sum3,
            _ => unreachable!(),
        };
    }

    // Reduce back from u128 to GoldilocksField.
    // Each value is at most about 6·2^64 < 2^67, so hi < 8.
    // Reduce96Bit: hi * EPSILON + lo
    for i in 0..WIDTH {
        let hi = (vals[i] >> 64) as u64;
        let lo = vals[i] as u64;
        let t1 = hi.wrapping_mul(GoldilocksField::EPSILON);
        let (res, carry) = lo.overflowing_add(t1);
        s[i] = GoldilocksField(res.wrapping_add(GoldilocksField::EPSILON & (carry as u64).wrapping_neg()));
    }
}

// ------------------------------------------------------------------
// Internal linear layer
//   sum = Σ state[i]
//   state[i] = state[i] * DIAG[i] + sum
// ------------------------------------------------------------------

fn internal_linear_layer(state: &mut [GoldilocksField; WIDTH]) {
    // Compute sum as u128 to avoid overflow
    let mut sum128: u128 = state[0].0 as u128;
    for s in &state[1..] {
        sum128 += s.0 as u128;
    }
    // Reduce sum to GoldilocksField (max 12·2^64 < 2^68)
    let sum_hi = (sum128 >> 64) as u64;
    let sum_lo = sum128 as u64;
    let t1 = sum_hi.wrapping_mul(GoldilocksField::EPSILON);
    let (sum_reduced, carry) = sum_lo.overflowing_add(t1);
    let sum_f = GoldilocksField(sum_reduced.wrapping_add(
        GoldilocksField::EPSILON & (carry as u64).wrapping_neg(),
    ));

    // state[i] = sum_f + state[i] * DIAG[i]   (via MulAccF)
    for (s, &diag_val) in state.iter_mut().zip(MATRIX_DIAG_U64.iter()) {
        let diag = GoldilocksField(diag_val);
        let prod = (s.0 as u128) * (diag.0 as u128);
        let acc = (sum_f.0 as u128) + prod;
        let hi = (acc >> 64) as u64;
        let lo = acc as u64;
        let hi_hi = hi >> 32;
        let hi_lo = hi & GoldilocksField::EPSILON;

        let (t0, borrow) = lo.overflowing_sub(hi_hi);
        let t0 = t0.wrapping_sub(GoldilocksField::EPSILON & (borrow as u64).wrapping_neg());
        let t1 = hi_lo.wrapping_mul(GoldilocksField::EPSILON);
        let (sum, over) = t0.overflowing_add(t1);
        *s = GoldilocksField(sum.wrapping_add(
            GoldilocksField::EPSILON & (over as u64).wrapping_neg(),
        ));
    }
}

// ------------------------------------------------------------------
// S-box: x^7 = ((x^2 * x)^2) * x
// ------------------------------------------------------------------

fn sbox_p(state: &mut [GoldilocksField; WIDTH], idx: usize) {
    let tmp = state[idx];
    let tmp_sq = tmp.square();
    let tmp_6th = tmp_sq.mul(&tmp).square(); // (x²·x)² = x⁶
    state[idx] = tmp_6th.mul(&tmp);           // x⁶·x = x⁷
}

fn sbox_all(state: &mut [GoldilocksField; WIDTH]) {
    for i in 0..WIDTH {
        sbox_p(state, i);
    }
}

fn sbox_one(state: &mut [GoldilocksField; WIDTH], idx: usize) {
    sbox_p(state, idx);
}

// ------------------------------------------------------------------
// Round constant additions
// ------------------------------------------------------------------

fn add_external_rc(state: &mut [GoldilocksField; WIDTH], round: usize) {
    for i in 0..WIDTH {
        state[i] = state[i].add_canonical_u64(EXTERNAL_CONSTANTS[round][i]);
    }
}

fn add_internal_rc(state: &mut [GoldilocksField; WIDTH], round: usize) {
    state[0] = state[0].add_canonical_u64(INTERNAL_CONSTANTS[round]);
}

// ------------------------------------------------------------------
// Hash functions
// ------------------------------------------------------------------

/// Hashes a slice of Goldilocks elements to an Fp5 quintic extension element.
pub fn hash_to_quintic_extension(input: &[GoldilocksField]) -> Fp5 {
    let out = hash_n_to_m(input, 5);
    Fp5::from_array([out[0], out[1], out[2], out[3], out[4]])
}

/// Hashes to exactly 4 output elements (HashOut).
pub fn hash_no_pad(input: &[GoldilocksField]) -> HashOut {
    let out = hash_n_to_m(input, 4);
    [out[0], out[1], out[2], out[3]]
}

/// Combines multiple HashOut values into one.
pub fn hash_n_to_one(hashes: &[HashOut]) -> HashOut {
    match hashes.len() {
        0 => empty_hash_out(),
        1 => hashes[0],
        _ => {
            let mut res = hash_two_to_one(hashes[0], hashes[1]);
            for h in &hashes[2..] {
                res = hash_two_to_one(res, *h);
            }
            res
        }
    }
}

/// Hashes two HashOut values into one.
fn hash_two_to_one(a: HashOut, b: HashOut) -> HashOut {
    hash_no_pad(&[a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3]])
}

/// Core sponge: absorbs input in RATE-size chunks, squeezes `num_outputs` elements.
fn hash_n_to_m(input: &[GoldilocksField], num_outputs: usize) -> Vec<GoldilocksField> {
    let mut perm = [GoldilocksField::ZERO; WIDTH];

    for chunk in input.chunks(RATE) {
        for (j, val) in chunk.iter().enumerate() {
            perm[j] = *val;
        }
        permute(&mut perm);
    }

    let mut outputs = Vec::with_capacity(num_outputs);
    loop {
        for &val in perm.iter().take(RATE) {
            outputs.push(val);
            if outputs.len() == num_outputs {
                return outputs;
            }
        }
        permute(&mut perm);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::goldilocks::GoldilocksField;

    fn gf(x: u64) -> GoldilocksField {
        GoldilocksField::from_canonical_u64(x)
    }

    #[test]
    fn empty_hash_is_zero() {
        let h = empty_hash_out();
        for v in &h {
            assert!(v.is_zero());
        }
    }

    #[test]
    fn hash_no_pad_single_element() {
        let input = [gf(1)];
        let out = hash_no_pad(&input);
        assert_eq!(out.len(), 4);
        // Output should be deterministic
        let out2 = hash_no_pad(&input);
        assert_eq!(out, out2);
    }

    #[test]
    fn hash_to_quintic_ext_basic() {
        let input = [gf(1), gf(2), gf(3)];
        let fp5 = hash_to_quintic_extension(&input);
        // Output should be deterministic
        let fp5_2 = hash_to_quintic_extension(&input);
        assert_eq!(fp5, fp5_2);
    }

    #[test]
    fn hash_no_pad_different_inputs() {
        let out1 = hash_no_pad(&[gf(1), gf(2)]);
        let out2 = hash_no_pad(&[gf(1), gf(3)]);
        assert_ne!(out1, out2, "different inputs should produce different hash");
    }

    #[test]
    fn hash_no_pad_multi_chunk() {
        // 9 elements > RATE(8), triggers two absorption phases
        let input: Vec<_> = (0..9).map(gf).collect();
        let out1 = hash_no_pad(&input);
        let out2 = hash_no_pad(&input);
        assert_eq!(out1, out2);
    }

    #[test]
    fn hash_n_to_one_single() {
        let h = hash_no_pad(&[gf(1)]);
        assert_eq!(hash_n_to_one(&[h]), h);
    }

    #[test]
    fn hash_n_to_one_two() {
        let h1 = hash_no_pad(&[gf(1)]);
        let h2 = hash_no_pad(&[gf(2)]);
        let combined = hash_n_to_one(&[h1, h2]);
        let expected = hash_two_to_one(h1, h2);
        assert_eq!(combined, expected);
    }

    #[test]
    fn serde_hash_out_roundtrip() {
        let h = hash_no_pad(&[gf(42)]);
        let bytes = hash_out_to_bytes(&h);
        assert_eq!(bytes.len(), 32);
        let restored = hash_out_from_bytes(&bytes).unwrap();
        assert_eq!(h, restored);
    }

    #[test]
    fn permute_leaves_state_consistent() {
        let mut state = [gf(0); WIDTH];
        state[0] = gf(1);
        permute(&mut state);
        // State should be non-zero after permutation
        let all_zero = state.iter().all(|s| s.is_zero());
        assert!(!all_zero, "permute should mix state");
    }
}
