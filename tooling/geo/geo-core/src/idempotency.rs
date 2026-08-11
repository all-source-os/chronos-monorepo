//! Idempotency-key derivation.
//!
//! Every `geo.*` event has a *natural key* — the tuple of facts that makes it
//! the same observation, not a new one. Re-ingesting the same crawl log window
//! or re-reading the same probe run must not double-count, so the natural key
//! is hashed into a stable token that becomes both the event's `entity_id`
//! suffix and its `metadata.idempotency_key`.
//!
//! Because the token lands in the `entity_id`, a replay appends version 2 to
//! the *same* Core entity rather than creating a second one. Counting distinct
//! entities is therefore replay-safe even though counting raw events is not.

use sha2::{Digest, Sha256};

/// Field separator. Unit Separator (0x1f) cannot appear in any of the inputs
/// we hash (URLs, ids, RFC 3339 timestamps), so `["a", "b"]` can never collide
/// with `["a\u{1f}b"]`-style ambiguity from a naive join.
const SEP: u8 = 0x1f;

/// Number of hex characters kept from the SHA-256 digest. 32 hex chars = 128
/// bits — collision-free for any plausible GEO volume, and short enough that
/// an `entity_id` stays greppable.
const KEY_LEN: usize = 32;

/// Hash a natural key into a stable, URL-safe idempotency token.
///
/// Deterministic across processes and releases: the same parts in the same
/// order always produce the same token.
pub fn derive_key(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            hasher.update([SEP]);
        }
        hasher.update(part.as_bytes());
    }
    let digest = hasher.finalize();
    let mut out = String::with_capacity(KEY_LEN);
    for byte in digest.iter().take(KEY_LEN / 2) {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_is_stable_and_the_right_length() {
        let key = derive_key(&["chatgpt.com", "/pricing"]);
        assert_eq!(key.len(), KEY_LEN);
        assert_eq!(key, derive_key(&["chatgpt.com", "/pricing"]));
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn field_boundaries_are_not_ambiguous() {
        // Without a separator these two would hash identically.
        assert_ne!(derive_key(&["ab", "c"]), derive_key(&["a", "bc"]));
    }

    #[test]
    fn order_matters() {
        assert_ne!(derive_key(&["a", "b"]), derive_key(&["b", "a"]));
    }
}
