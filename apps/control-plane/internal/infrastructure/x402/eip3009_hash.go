package x402

import (
	"math/big"

	"golang.org/x/crypto/sha3"
)

// EIP3009Hash computes the EIP-712 hash for a TransferWithAuthorization that
// must be signed to produce a valid EIP-3009 payment. The hash is chain-aware
// (uses chainID and the USDC contract address in the domain separator).
//
// This is a pure Go implementation with no go-ethereum dependency.
// The resulting hash is passed to the CDP sign_payload endpoint.
func EIP3009Hash(p EIP3009HashParams) []byte {
	domainSep := eip712DomainSeparator(p.ChainID, p.USDCAddress)
	structHash := eip3009StructHash(p)
	return keccak256([]byte("\x19\x01"), domainSep, structHash)
}

// EIP3009HashParams holds the inputs needed to compute an EIP-3009 hash.
// This mirrors clients.EIP3009SignParams but lives in the x402 package
// so middleware can call it without importing clients.
type EIP3009HashParams struct {
	From        string
	To          string
	Value       *big.Int
	ValidAfter  *big.Int
	ValidBefore *big.Int
	Nonce       [32]byte
	USDCAddress string // USDC contract address on the target network
	ChainID     int64
}

// eip712DomainSeparator computes the EIP-712 domain separator for USDC on a given chain.
// USDC v2 domain: name="USD Coin", version="2".
func eip712DomainSeparator(chainID int64, usdcContract string) []byte {
	domainTypeHash := keccak256([]byte(
		"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
	))
	nameHash := keccak256([]byte("USD Coin"))
	versionHash := keccak256([]byte("2"))

	encoded := make([]byte, 5*32)
	copy(encoded[0:32], domainTypeHash)
	copy(encoded[32:64], nameHash)
	copy(encoded[64:96], versionHash)
	copy(encoded[96:128], abiEncodeUint256(big.NewInt(chainID)))
	copy(encoded[128:160], abiEncodeAddress(usdcContract))

	return keccak256(encoded)
}

// transferWithAuthTypeHash is the keccak256 of the EIP-3009 type string.
// Computed once at init time.
var transferWithAuthTypeHash = keccak256([]byte(
	"TransferWithAuthorization(address from,address to,uint256 value,uint256 validAfter,uint256 validBefore,bytes32 nonce)",
))

// eip3009StructHash computes the struct hash for TransferWithAuthorization.
func eip3009StructHash(p EIP3009HashParams) []byte {
	encoded := make([]byte, 7*32)
	copy(encoded[0:32], transferWithAuthTypeHash)
	copy(encoded[32:64], abiEncodeAddress(p.From))
	copy(encoded[64:96], abiEncodeAddress(p.To))
	copy(encoded[96:128], abiEncodeUint256(p.Value))
	copy(encoded[128:160], abiEncodeUint256(p.ValidAfter))
	copy(encoded[160:192], abiEncodeUint256(p.ValidBefore))
	copy(encoded[192:224], p.Nonce[:])
	return keccak256(encoded)
}

// keccak256 returns the Keccak-256 hash (Ethereum's hash, not NIST SHA3-256).
func keccak256(data ...[]byte) []byte {
	h := sha3.NewLegacyKeccak256()
	for _, d := range data {
		h.Write(d)
	}
	return h.Sum(nil)
}

// abiEncodeUint256 encodes a big.Int as a 32-byte big-endian ABI value.
func abiEncodeUint256(n *big.Int) []byte {
	b := make([]byte, 32)
	if n != nil {
		raw := n.Bytes()
		if len(raw) > 32 {
			raw = raw[len(raw)-32:]
		}
		copy(b[32-len(raw):], raw)
	}
	return b
}

// abiEncodeAddress encodes a hex address string as a 32-byte ABI value (left-padded).
func abiEncodeAddress(addr string) []byte {
	b := make([]byte, 32)
	addr = stripHexPrefix(addr)
	if len(addr) > 40 {
		addr = addr[len(addr)-40:]
	}
	// Decode hex into the last 20 bytes of the 32-byte slot
	raw := make([]byte, 20)
	for i := 0; i < len(addr)/2; i++ {
		hi := hexNibble(addr[2*i])
		lo := hexNibble(addr[2*i+1])
		raw[i] = (hi << 4) | lo
	}
	copy(b[12:], raw)
	return b
}

func stripHexPrefix(s string) string {
	if len(s) >= 2 && (s[:2] == "0x" || s[:2] == "0X") {
		return s[2:]
	}
	return s
}

func hexNibble(c byte) byte {
	switch {
	case c >= '0' && c <= '9':
		return c - '0'
	case c >= 'a' && c <= 'f':
		return c - 'a' + 10
	case c >= 'A' && c <= 'F':
		return c - 'A' + 10
	default:
		return 0
	}
}
