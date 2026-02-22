# shufti-macro + shufti-matcher

A proc-macro that generates SIMD shufti lookup tables **at compile time** and
implements a `ShuftiMatcher` trait on any marker struct.

## Workspace layout

```txt
shufti_macro/
├── shufti-macro/      # proc-macro crate (the derive)
├── shufti-matcher/    # trait + ShuftiTable + SIMD/scalar impl
└── example/           # usage demo
```

## Usage

Add to `Cargo.toml`:

```toml
[dependencies]
shufti-matcher = { git = "..." } 
```

```rust
use shufti_matcher::ShuftiMatcher;

/// Matches whitespace characters – tables computed at compile time.
#[derive(ShuftiMatcher)]
#[shufti(set = "\t\r\n")]
pub struct WhitespaceMatcher;

/// Matches bracket characters.
#[derive(ShuftiMatcher)]
#[shufti(set = "[]{}<>()")]
pub struct BracketMatcher;

fn main() {
    // find_first: searches an arbitrary byte slice
    assert_eq!(WhitespaceMatcher::find_first(b"hello\nworld"), Some(5));
    assert_eq!(WhitespaceMatcher::find_first(b"no match"),    None);

    // match_16b: returns a u16 bitmask for exactly 16 bytes
    let chunk = *b"aaa[aaa]aaaa{aaa";
    let mask = BracketMatcher::match_16b(&chunk);
    // bit i set  ⟺  chunk[i] is in the set
    // positions 3, 7, 12 → 0b0001_0000_1000_1000
    assert_eq!(mask, (1 << 3) | (1 << 7) | (1 << 12));

    // Compile-time constants
    println!("{}", WhitespaceMatcher::SET);          // "\t\r\n"
    println!("{}", BracketMatcher::NEEDLE_COUNT);    // 8
}
```

## Rules for the `set`

| Rule                        | Reason                                                                                                          |
| --------------------------- | --------------------------------------------------------------------------------------------------------------- |
| 1 – 8 bytes                 | One bit per needle; 8 bits fit in a `u8` mask                                                                   |
| All bytes distinct          | Sharing a bit-bucket requires `build_shufti` sharing logic; `build_shufti_fast` (used here) requires uniqueness |
| Compile-time string literal | Table generation happens inside the proc-macro, producing `const` arrays                                        |

## The `ShuftiMatcher` trait

```rust
pub trait ShuftiMatcher {
    const SET: &'static str;          // the literal passed to #[shufti(set = "...")]
    const NEEDLE_COUNT: usize;        // len of SET

    fn table() -> ShuftiTable;        // returns the embedded const tables

    fn match_16b(chunk: &[u8; 16]) -> u16;   // SIMD test of one chunk → bitmask
    fn find_first(haystack: &[u8]) -> Option<usize>; // full search
}
```

### `match_16b` 16-byte SIMD chunk

Returns a `u16` where **bit *i* is set iff `chunk[i]` is in the needle set**.
On AArch64 this uses NEON `vqtbl1q_u8` shuffle + `vtstq_u8` + pairwise adds.
On other targets the scalar fallback is selected automatically via `cfg`.

### `find_first` arbitrary-length search

```txt
[chunk 0..16] → match_16b → any hit? → trailing_zeros + offset
[chunk 16..32] → …
…
[epilogue <16 bytes] → zero-padded buf → match_16b → clamp to remainder
```

The epilogue re-uses `match_16b` on a zero-padded buffer; positions ≥ `remainder`
are discarded, so a `\0` needle in the set is handled correctly.

## What the macro generates

Given:

```rust
#[derive(ShuftiMatcher)]
#[shufti(set = "\t\r\n")]
pub struct WhitespaceMatcher;
```

The macro emits (simplified):

```rust
impl ::shufti_matcher::ShuftiMatcher for WhitespaceMatcher {
    const SET: &'static str = "\t\r\n";
    const NEEDLE_COUNT: usize = 3;

    fn table() -> ::shufti_matcher::ShuftiTable {
        ::shufti_matcher::ShuftiTable {
            low_tab:  [0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 3u8, 2u8, 0u8, 0u8, 4u8, 0u8, 0u8],
            high_tab: [4u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8],
            bit_mask: 7u8,
        }
    }
}
```

No runtime computation; the arrays are literal constants baked into the binary.

## Extending to `build_shufti` (>8 or duplicate bytes)

The macro currently mirrors `build_shufti_fast` (unique needles, ≤8).
To support the sharing strategy of `build_shufti`, replace `build_shufti_tables`
in `shufti-macro/src/lib.rs` with the same algorithm and update the uniqueness
assertion accordingly.
