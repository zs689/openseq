//! Block identity.
//!
//! Logseq identifies a block by a uuid, and that uuid is what `((block ref))`
//! points at — so it has to be *unique*, but it does **not** have to be
//! cryptographically random. An id only ever needs to be unique inside one
//! user's graph; there is no adversary choosing inputs.
//!
//! So instead of depending on `getrandom` (which on Windows drags in an FFI
//! layer and, for the GNU targets, a C toolchain just to build its import
//! libraries), we derive 128 bits from a process-local seed plus a monotonic
//! counter and format them as a **v4-shaped uuid**, keeping the value
//! byte-compatible with Logseq's `id::` property.
//!
//! The seed mixes the wall clock, a stack address (ASLR) and a heap address,
//! which is enough spread that two processes started in the same nanosecond
//! still diverge.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicU64 = AtomicU64::new(0);
static SEED: OnceLock<[u64; 2]> = OnceLock::new();

/// Generate a new block id, formatted like a v4 uuid.
pub fn new_block_id() -> String {
    let [s0, s1] = seed();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);

    let hi = splitmix64(s0 ^ n);
    let lo = splitmix64(s1 ^ n.wrapping_mul(0x9E37_79B9_7F4A_7C15));

    // 8-4-4-4-12, with the version nibble forced to 4 and the variant bits to 10xx.
    format!(
        "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        (hi >> 32) as u32,
        (hi >> 16) as u16,
        (hi & 0x0fff) as u16,
        (0x8000 | ((lo >> 48) as u16 & 0x3fff)),
        lo & 0x0000_ffff_ffff_ffff,
    )
}

/// Whether `value` looks like an id this module produced (or a Logseq uuid).
pub fn is_block_id(value: &str) -> bool {
    const GROUPS: [usize; 5] = [8, 4, 4, 4, 12];
    let mut parts = value.split('-');
    for expected in GROUPS {
        match parts.next() {
            Some(part) if part.len() == expected && part.chars().all(|c| c.is_ascii_hexdigit()) => {
            }
            _ => return false,
        }
    }
    parts.next().is_none()
}

fn splitmix64(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn seed() -> [u64; 2] {
    *SEED.get_or_init(|| {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        // Stack and heap addresses both move with ASLR, which gives us
        // per-process entropy without asking the OS for a random number.
        let on_stack = 0u8;
        let stack = (&on_stack as *const u8) as u64;
        let on_heap: Box<u8> = Box::new(0);
        let heap = (&*on_heap as *const u8) as u64;

        [
            nanos ^ stack.rotate_left(17),
            heap ^ nanos.rotate_right(29) ^ (std::process::id() as u64).rotate_left(41),
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ids_look_like_v4_uuids() {
        let id = new_block_id();
        assert!(is_block_id(&id), "not uuid shaped: {id}");
        assert_eq!(id.len(), 36);
        assert_eq!(id.chars().nth(14), Some('4'), "version nibble: {id}");
        assert!(
            matches!(id.chars().nth(19), Some('8' | '9' | 'a' | 'b')),
            "variant: {id}"
        );
    }

    #[test]
    fn ids_do_not_repeat() {
        let ids: HashSet<String> = (0..20_000).map(|_| new_block_id()).collect();
        assert_eq!(ids.len(), 20_000, "the generator produced a collision");
    }

    #[test]
    fn rejects_non_ids() {
        for bad in [
            "",
            "hello",
            "3f2504e0-4f89-41d3-9a0c",
            "3f2504e0-4f89-41d3-9a0c-0305e82c33011",
        ] {
            assert!(!is_block_id(bad), "should have rejected {bad:?}");
        }
    }
}
