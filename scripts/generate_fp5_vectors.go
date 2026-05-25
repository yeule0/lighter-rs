package main

import (
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"

	g "github.com/elliottech/poseidon_crypto/field/goldilocks"
	gFp5 "github.com/elliottech/poseidon_crypto/field/goldilocks_quintic_extension"
)

type TestVector struct {
	Name   string `json:"name"`
	A      string `json:"a"`      // hex LE 40 bytes
	B      string `json:"b"`      // hex LE 40 bytes (empty for unary ops)
	Add    string `json:"add"`    // hex LE 40 bytes
	Sub    string `json:"sub"`    // hex LE 40 bytes
	Mul    string `json:"mul"`    // hex LE 40 bytes
	Square string `json:"square"` // hex LE 40 bytes
	Inv    string `json:"inv"`    // hex LE 40 bytes
}

func fp5ToHex(e gFp5.Element) string {
	return hex.EncodeToString(e.ToLittleEndianBytes())
}

func main() {
	vectors := []TestVector{}

	// Test cases covering edge cases
	inputs := [][2]gFp5.Element{
		// Basic: ones and zeros
		{gFp5.FP5_ZERO, gFp5.FP5_ONE},
		{gFp5.FP5_ONE, gFp5.FP5_ONE},
		{gFp5.FP5_TWO, gFp5.FP5_TWO},

		// Powers of x
		{{0, 1, 0, 0, 0}, {0, 1, 0, 0, 0}}, // x * x = x^2
		{{0, 0, 1, 0, 0}, {0, 1, 0, 0, 0}}, // x^2 * x = x^3

		// Random-ish values
		{{1, 2, 3, 4, 5}, {10, 20, 30, 40, 50}},
		{{42, 0, 0, 0, 0}, {0, 99, 0, 0, 0}},
		{{100, 200, 300, 400, 500}, {1, 2, 3, 4, 5}},

		// Near modulus boundary
		{{
			g.GoldilocksField(g.ORDER - 1),
			g.GoldilocksField(g.ORDER - 2),
			g.GoldilocksField(g.ORDER - 3),
			g.GoldilocksField(g.ORDER - 4),
			g.GoldilocksField(g.ORDER - 5),
		}, {
			g.GoldilocksField(2),
			g.GoldilocksField(3),
			g.GoldilocksField(4),
			g.GoldilocksField(5),
			g.GoldilocksField(6),
		}},

		// Same element for square verification
		{{2, 3, 5, 7, 11}, {2, 3, 5, 7, 11}},

		// All high bits
		{{
			g.GoldilocksField(0xFFFFFFFF00000000),
			g.GoldilocksField(0xFFFFFFFF00000000),
			g.GoldilocksField(0xFFFFFFFF00000000),
			g.GoldilocksField(0xFFFFFFFF00000000),
			g.GoldilocksField(0xFFFFFFFF00000000),
		}, {
			g.GoldilocksField(1),
			g.GoldilocksField(1),
			g.GoldilocksField(1),
			g.GoldilocksField(1),
			g.GoldilocksField(1),
		}},

		// Zero and non-zero combos
		{{0, 1, 2, 0, 3}, {4, 0, 5, 6, 0}},
	}

	for i, pair := range inputs {
		a := pair[0]
		b := pair[1]

		tv := TestVector{
			Name: fmt.Sprintf("vector_%d", i),
			A:    fp5ToHex(a),
			B:    fp5ToHex(b),
			Add:  fp5ToHex(gFp5.Add(a, b)),
			Sub:  fp5ToHex(gFp5.Sub(a, b)),
			Mul:  fp5ToHex(gFp5.Mul(a, b)),
		}

		// Square: square of 'a'
		tv.Square = fp5ToHex(gFp5.Square(a))

		// Inverse: inverse of 'a' (skip zero)
		if !gFp5.IsZero(a) {
			tv.Inv = fp5ToHex(gFp5.InverseOrZero(a))
		}

		vectors = append(vectors, tv)
	}

	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	enc.Encode(vectors)
}
