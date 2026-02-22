//! # shufti-runtime
//!
//! Runtime support for the `ShuftiMatcher` derive macro.
//!
//! Provides:
//! - [`ShuftiTable`] – the precomputed SIMD lookup tables (AArch64).
//! - [`ShuftiMatcher`] – trait implemented by the derive macro; exposes
//!   [`match_16b`](ShuftiMatcher::match_16b) and [`find_first`](ShuftiMatcher::find_first).
//!
//! ## Usage
//!
//! ```rust,ignore
//! use shufti_matcher::{ShuftiMatcher};
//! use shufti_macro::ShuftiMatcher;
//!
//! #[derive(ShuftiMatcher)]
//! #[shufti(set = "\t\r\n")]
//! pub struct WhitespaceMatcher;
//!
//! let pos = WhitespaceMatcher::find_first(b"hello\nworld");
//! assert_eq!(pos, Some(5));
//! ```

// Re-export the derive macro for convenience (one `use` covers both).
pub use shufti_macro::ShuftiMatcher;

// ---------------------------------------------------------------------------
// ShuftiTable
// ---------------------------------------------------------------------------

/// Pre-computed shufti lookup tables. Produced by the derive macro at compile
/// time; never constructed at runtime.
#[derive(Debug, Clone, Copy)]
pub struct ShuftiTable {
    pub low_tab: [u8; 16],
    pub high_tab: [u8; 16],
    pub bit_mask: u8,
}

impl ShuftiTable {
    /// Test a 16-byte chunk. Returns a bitmask where bit *i* is set when
    /// `data[i]` is a member of the needle set.
    ///
    /// # Safety
    /// Must be called on an AArch64 target with NEON available.
    #[cfg(target_arch = "aarch64")]
    #[inline(always)]
    pub unsafe fn bitmask_16b(&self, data: &[u8; 16]) -> u16 {
        use core::arch::aarch64::*;

        unsafe {
            let l_tab = vld1q_u8(self.low_tab.as_ptr());
            let h_tab = vld1q_u8(self.high_tab.as_ptr());
            let input = vld1q_u8(data.as_ptr());

            let low_mask = vmovq_n_u8(0x0f);
            let lo = vandq_u8(input, low_mask);
            let hi = vshrq_n_u8(input, 4);

            let lo_sf = vqtbl1q_u8(l_tab, lo);
            let hi_sf = vqtbl1q_u8(h_tab, hi);
            let v = vandq_u8(lo_sf, hi_sf);

            // vtstq_u8: sets byte to 0xFF if (v & bit_mask) != 0
            let matches = vtstq_u8(v, vmovq_n_u8(self.bit_mask));

            // movemask: pack one bit per lane into a u16
            let masked = vandq_u8(
                matches,
                vld1q_u8([1u8, 2, 4, 8, 16, 32, 64, 128, 1, 2, 4, 8, 16, 32, 64, 128].as_ptr()),
            );
            let res64 = vpaddlq_u8(masked);
            let res32 = vpaddlq_u16(res64);
            let res16 = vpaddlq_u32(res32);

            let mask64: u64 = vgetq_lane_u64(res16, 0) | (vgetq_lane_u64(res16, 1) << 8);

            mask64 as u16
        }
    }

    /// Scalar fallback for non-NEON targets (used in cfg tests / CI).
    #[cfg(not(target_arch = "aarch64"))]
    #[inline(always)]
    pub unsafe fn bitmask_16b(&self, data: &[u8; 16]) -> u16 {
        let mut mask = 0u16;
        for (i, &b) in data.iter().enumerate() {
            let lo = (b & 0x0f) as usize;
            let hi = (b >> 4) as usize;
            if (self.low_tab[lo] & self.high_tab[hi] & self.bit_mask) != 0 {
                mask |= 1 << i;
            }
        }
        mask
    }
}

// ---------------------------------------------------------------------------
// ShuftiMatcher trait
// ---------------------------------------------------------------------------

/// Trait implemented automatically by `#[derive(ShuftiMatcher)]`.
///
/// Implementors expose two search entry points:
/// - [`match_16b`](Self::match_16b) – single 16-byte chunk, returns position bitmask.
/// - [`find_first`](Self::find_first) – searches an arbitrary byte slice.
pub trait ShuftiMatcher {
    /// The literal set string provided to `#[shufti(set = "...")]`.
    const SET: &'static str;
    /// Number of distinct bytes in the set.
    const NEEDLE_COUNT: usize;

    /// Return the precomputed lookup tables for this matcher.
    fn table() -> ShuftiTable;

    /// Test exactly 16 bytes. Returns a `u16` bitmask: bit *i* is 1 when
    /// `chunk[i]` belongs to the set. Wraps [`ShuftiTable::test_chunk`].
    ///
    /// # Safety
    /// On AArch64 the underlying NEON intrinsics are used directly. Callers
    /// on other architectures get the scalar fallback automatically.
    #[inline(always)]
    fn match_16b(chunk: &[u8; 16]) -> u16 {
        // SAFETY: test_chunk is always safe on the current platform;
        // the unsafe block is required only because of the aarch64 intrinsics.
        unsafe { Self::table().bitmask_16b(chunk) }
    }

    /// Search `haystack` for the first byte that belongs to the set.
    ///
    /// Processes the input in 16-byte chunks using [`match_16b`](Self::match_16b),
    /// then handles the remaining epilogue byte-by-byte with a scalar fallback.
    fn find_first(haystack: &[u8]) -> Option<usize> {
        let table = Self::table();
        let mut offset = 0usize;

        // --- 16-byte fast path ---
        while offset + 16 <= haystack.len() {
            // SAFETY: we've checked there are ≥16 bytes available.
            let chunk: &[u8; 16] = unsafe { &*(haystack.as_ptr().add(offset) as *const [u8; 16]) };
            let mask = unsafe { table.bitmask_16b(chunk) } as u32;
            if mask != 0 {
                return Some(offset + mask.trailing_zeros() as usize);
            }
            offset += 16;
        }

        // --- Scalar epilogue ---
        // Load the remaining bytes into a zeroed 16-byte buffer so we can
        // still use the SIMD path (zeros never match real needles unless '\0'
        // is in the set — handled below via position-clamped output).
        let remainder = haystack.len() - offset;
        if remainder > 0 {
            let mut buf = [0u8; 16];
            buf[..remainder].copy_from_slice(&haystack[offset..]);
            let mask = unsafe { table.bitmask_16b(&buf) } as u32;
            if mask != 0 {
                let pos = mask.trailing_zeros() as usize;
                if pos < remainder {
                    return Some(offset + pos);
                }
                // pos >= remainder means only the zero-padding matched '\0'
                // If '\0' is actually a needle we need the scalar check:
                for i in 0..remainder {
                    if mask & (1 << i) != 0 {
                        return Some(offset + i);
                    }
                }
            }
        }

        None
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal hand-rolled implementation for testing without the macro.
    struct WsMatcher;
    impl ShuftiMatcher for WsMatcher {
        const SET: &'static str = "\t\r\n";
        const NEEDLE_COUNT: usize = 3;
        fn table() -> ShuftiTable {
            // build_shufti_fast equivalent for [0x09, 0x0d, 0x0a]
            let needles: &[u8] = b"\t\r\n";
            let mut low_tab = [0u8; 16];
            let mut high_tab = [0u8; 16];
            for (i, &b) in needles.iter().enumerate() {
                let bit = 1u8 << i;
                low_tab[(b & 0x0f) as usize] |= bit;
                high_tab[(b >> 4) as usize] |= bit;
            }
            ShuftiTable {
                low_tab,
                high_tab,
                bit_mask: (1u32 << needles.len()).wrapping_sub(1) as u8,
            }
        }
    }

    #[test]
    fn test_match_16b_none() {
        let chunk = *b"abcdefghijklmnop";
        assert_eq!(WsMatcher::match_16b(&chunk), 0);
    }

    #[test]
    fn test_match_16b_first() {
        let chunk = *b"\tbcdefghijklmnop";
        let mask = WsMatcher::match_16b(&chunk);
        assert_eq!(mask.trailing_zeros(), 0);
    }

    #[test]
    fn test_match_16b_last() {
        let chunk = *b"abcdefghijklmno\n";
        let mask = WsMatcher::match_16b(&chunk);
        assert_eq!(mask.trailing_zeros(), 15);
    }

    #[test]
    fn test_find_first_empty() {
        assert_eq!(WsMatcher::find_first(b""), None);
    }

    #[test]
    fn test_find_first_no_match() {
        assert_eq!(WsMatcher::find_first(b"hello world"), None);
    }

    #[test]
    fn test_find_first_exact_16() {
        let hay = b"abcdefghijklmno\n";
        assert_eq!(WsMatcher::find_first(hay), Some(15));
    }

    #[test]
    fn test_find_first_crosses_chunk_boundary() {
        let hay = b"abcdefghijklmnopqrs\tuvwxyz";
        assert_eq!(WsMatcher::find_first(hay), Some(19));
    }

    #[test]
    fn test_find_first_epilogue_only() {
        let hay = b"abcde\r";
        assert_eq!(WsMatcher::find_first(hay), Some(5));
    }

    #[test]
    fn test_find_first_first_byte() {
        let hay = b"\nhello";
        assert_eq!(WsMatcher::find_first(hay), Some(0));
    }
}
