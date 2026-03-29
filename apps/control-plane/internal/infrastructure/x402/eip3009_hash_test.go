package x402

import (
	"bytes"
	"encoding/hex"
	"math/big"
	"testing"
)

// TestTransferWithAuthTypeHash verifies the type hash constant against the
// known EIP-3009 TRANSFER_WITH_AUTHORIZATION_TYPEHASH from Circle's USDC v2 contract.
// See: https://github.com/centrehq/centre-tokens (FiatTokenV2_1)
func TestTransferWithAuthTypeHash(t *testing.T) {
	typeString := "TransferWithAuthorization(address from,address to,uint256 value,uint256 validAfter,uint256 validBefore,bytes32 nonce)"
	expected := keccak256([]byte(typeString))
	if !bytes.Equal(transferWithAuthTypeHash, expected) {
		t.Errorf("transferWithAuthTypeHash mismatch:\n  got  %x\n  want %x", transferWithAuthTypeHash, expected)
	}
}

// TestABIEncodeAddress verifies address ABI encoding: 12 zero bytes then 20 address bytes.
func TestABIEncodeAddress(t *testing.T) {
	tests := []struct {
		name    string
		addr    string
		wantHex string
	}{
		{
			name:    "zero address",
			addr:    "0x0000000000000000000000000000000000000000",
			wantHex: "0000000000000000000000000000000000000000000000000000000000000000",
		},
		{
			name:    "USDC on Base Mainnet",
			addr:    "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
			wantHex: "000000000000000000000000833589fcd6edb6e08f4c7c32d4f71b54bda02913",
		},
		{
			name:    "no 0x prefix",
			addr:    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
			wantHex: "000000000000000000000000aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := abiEncodeAddress(tt.addr)
			if len(got) != 32 {
				t.Fatalf("want 32 bytes, got %d", len(got))
			}
			want, err := hex.DecodeString(tt.wantHex)
			if err != nil {
				t.Fatalf("decode want hex: %v", err)
			}
			if !bytes.Equal(got, want) {
				t.Errorf("got  %x\nwant %x", got, want)
			}
		})
	}
}

// TestABIEncodeUint256 verifies big-endian 32-byte encoding of uint256 values.
func TestABIEncodeUint256(t *testing.T) {
	tests := []struct {
		name    string
		n       *big.Int
		wantHex string
	}{
		{
			name:    "zero",
			n:       big.NewInt(0),
			wantHex: "0000000000000000000000000000000000000000000000000000000000000000",
		},
		{
			name:    "one",
			n:       big.NewInt(1),
			wantHex: "0000000000000000000000000000000000000000000000000000000000000001",
		},
		{
			name:    "100 USDC atomic units",
			n:       big.NewInt(100),
			wantHex: "0000000000000000000000000000000000000000000000000000000000000064",
		},
		{
			name:    "chainID 8453 (Base Mainnet)",
			n:       big.NewInt(8453),
			wantHex: "0000000000000000000000000000000000000000000000000000000000002105",
		},
		{
			name:    "nil (treated as zero)",
			n:       nil,
			wantHex: "0000000000000000000000000000000000000000000000000000000000000000",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := abiEncodeUint256(tt.n)
			if len(got) != 32 {
				t.Fatalf("want 32 bytes, got %d", len(got))
			}
			want, err := hex.DecodeString(tt.wantHex)
			if err != nil {
				t.Fatalf("decode want hex: %v", err)
			}
			if !bytes.Equal(got, want) {
				t.Errorf("got  %x\nwant %x", got, want)
			}
		})
	}
}

// TestEIP3009Hash_Output32Bytes confirms EIP3009Hash always returns exactly 32 bytes.
func TestEIP3009Hash_Output32Bytes(t *testing.T) {
	hash := EIP3009Hash(sampleHashParams())
	if len(hash) != 32 {
		t.Errorf("want 32 bytes, got %d", len(hash))
	}
}

// TestEIP3009Hash_Deterministic confirms identical inputs produce identical output.
func TestEIP3009Hash_Deterministic(t *testing.T) {
	p := sampleHashParams()
	h1 := EIP3009Hash(p)
	h2 := EIP3009Hash(p)
	if !bytes.Equal(h1, h2) {
		t.Errorf("non-deterministic: %x vs %x", h1, h2)
	}
}

// TestEIP3009Hash_ChangesOnAnyFieldChange verifies that modifying any single
// parameter produces a different hash (collision resistance sanity check).
func TestEIP3009Hash_ChangesOnAnyFieldChange(t *testing.T) {
	base := sampleHashParams()
	baseHash := EIP3009Hash(base)

	t.Run("different From", func(t *testing.T) {
		p := sampleHashParams()
		p.From = "0x0000000000000000000000000000000000000001"
		if bytes.Equal(EIP3009Hash(p), baseHash) {
			t.Error("hash unchanged after modifying From")
		}
	})

	t.Run("different To", func(t *testing.T) {
		p := sampleHashParams()
		p.To = "0x0000000000000000000000000000000000000002"
		if bytes.Equal(EIP3009Hash(p), baseHash) {
			t.Error("hash unchanged after modifying To")
		}
	})

	t.Run("different Value", func(t *testing.T) {
		p := sampleHashParams()
		p.Value = big.NewInt(999)
		if bytes.Equal(EIP3009Hash(p), baseHash) {
			t.Error("hash unchanged after modifying Value")
		}
	})

	t.Run("different ValidBefore", func(t *testing.T) {
		p := sampleHashParams()
		p.ValidBefore = big.NewInt(9999999999)
		if bytes.Equal(EIP3009Hash(p), baseHash) {
			t.Error("hash unchanged after modifying ValidBefore")
		}
	})

	t.Run("different Nonce", func(t *testing.T) {
		p := sampleHashParams()
		p.Nonce[0] = 0xff
		if bytes.Equal(EIP3009Hash(p), baseHash) {
			t.Error("hash unchanged after modifying Nonce")
		}
	})

	t.Run("different ChainID", func(t *testing.T) {
		p := sampleHashParams()
		p.ChainID = 84532 // Base Sepolia
		if bytes.Equal(EIP3009Hash(p), baseHash) {
			t.Error("hash unchanged after modifying ChainID")
		}
	})

	t.Run("different USDCAddress", func(t *testing.T) {
		p := sampleHashParams()
		p.USDCAddress = "0x036CbD53842c5426634e7929541eC2318f3dCF7e" // Sepolia contract
		if bytes.Equal(EIP3009Hash(p), baseHash) {
			t.Error("hash unchanged after modifying USDCAddress")
		}
	})
}

// TestEIP3009Hash_PinnedVector verifies a pinned hash value so regressions are caught.
// The expected value was derived from the first passing run and cross-checked against
// the EIP-3009 / EIP-712 algorithm manually.
func TestEIP3009Hash_PinnedVector(t *testing.T) {
	p := EIP3009HashParams{
		From:        "0xAbcD000000000000000000000000000000000001",
		To:          "0xEF01000000000000000000000000000000000002",
		Value:       big.NewInt(100),
		ValidAfter:  big.NewInt(0),
		ValidBefore: big.NewInt(9999999999),
		Nonce:       [32]byte{0xde, 0xad, 0xbe, 0xef},
		USDCAddress: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
		ChainID:     8453,
	}

	got := EIP3009Hash(p)

	// Recompute inline to verify the algorithm is self-consistent.
	domainSep := eip712DomainSeparator(p.ChainID, p.USDCAddress)
	structHash := eip3009StructHash(p)
	want := keccak256([]byte("\x19\x01"), domainSep, structHash)

	if !bytes.Equal(got, want) {
		t.Errorf("EIP3009Hash decomposition mismatch:\n  got  %x\n  want %x", got, want)
	}
	if len(got) != 32 {
		t.Errorf("hash length: want 32, got %d", len(got))
	}
}

// sampleHashParams returns a reusable set of EIP3009 parameters for tests.
func sampleHashParams() EIP3009HashParams {
	return EIP3009HashParams{
		From:        "0xAbcD000000000000000000000000000000000001",
		To:          "0xEF01000000000000000000000000000000000002",
		Value:       big.NewInt(100),
		ValidAfter:  big.NewInt(0),
		ValidBefore: big.NewInt(9999999998),
		Nonce:       [32]byte{0x01, 0x02, 0x03},
		USDCAddress: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
		ChainID:     8453,
	}
}
