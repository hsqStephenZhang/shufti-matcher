use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{DeriveInput, Lit, parse_macro_input};

/// Derive macro that reads `#[shufti(set = "...")]` on a struct and implements
/// `ShuftiMatcher` for it with tables computed at compile time.
///
/// # Example
/// ```rust,ignore
/// #[derive(ShuftiMatcher)]
/// #[shufti(set = "[]{}<>()")]
/// pub struct BracketMatcher;
/// ```
#[proc_macro_derive(ShuftiMatcher, attributes(shufti))]
pub fn derive_shufti_matcher(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match impl_shufti_matcher(&input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn impl_shufti_matcher(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Parse #[shufti(set = "...")] attribute
    let set_str = extract_set_attr(&input.attrs)?;
    let needles: Vec<u8> = set_str.bytes().collect();

    if needles.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "shufti set must have >=1 bytes",
        ));
    }

    // Check uniqueness
    for i in 0..needles.len() {
        for j in (i + 1)..needles.len() {
            if needles[i] == needles[j] {
                return Err(syn::Error::new_spanned(
                    &input.ident,
                    format!("shufti set contains duplicate byte 0x{:02x}", needles[i]),
                ));
            }
        }
    }

    // Compute tables at compile time (same logic as build_shufti_fast)
    let res = if needles.len() <= 8 {
        Some(build_shufti_tables(&needles))
    } else {
        build_shufti_table_slow(&needles)
    };

    let Some((low_tab, high_tab, bit_mask)) = res else {
        return Err(syn::Error::new_spanned(
            &input.ident,
            format!("failed to build shufti table"),
        ));
    };

    let low_tab_tokens = low_tab.iter().map(|b| quote! { #b});
    let high_tab_tokens = high_tab.iter().map(|b| quote! { #b});

    let needle_len = needles.len();
    let set_repr = set_str.clone();

    Ok(quote! {
        impl #impl_generics ::shufti_matcher::ShuftiMatch for #name #ty_generics #where_clause {
            const SET: &'static str = #set_repr;
            const NEEDLE_COUNT: usize = #needle_len;

            #[inline(always)]
            fn table() -> ::shufti_matcher::ShuftiTable {
                ::shufti_matcher::ShuftiTable {
                    low_tab:  [#(#low_tab_tokens),*],
                    high_tab: [#(#high_tab_tokens),*],
                    bit_mask: #bit_mask,
                }
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Attribute parsing
// ---------------------------------------------------------------------------

fn extract_set_attr(attrs: &[syn::Attribute]) -> syn::Result<String> {
    for attr in attrs {
        if !attr.path().is_ident("shufti") {
            continue;
        }

        let mut found: Option<String> = None;
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("set") {
                let value = meta.value()?; // consumes `=`
                let lit: Lit = value.parse()?;
                if let Lit::Str(ls) = lit {
                    found = Some(ls.value());
                    Ok(())
                } else {
                    Err(meta.error("expected string literal for `set`"))
                }
            } else {
                Err(meta.error("unknown shufti attribute key"))
            }
        })?;

        if let Some(s) = found {
            return Ok(s);
        }
    }
    Err(syn::Error::new(
        proc_macro2::Span::call_site(),
        "missing #[shufti(set = \"...\")] attribute",
    ))
}

// ---------------------------------------------------------------------------
// Compile-time table construction (mirrors build_shufti_fast)
// ---------------------------------------------------------------------------

fn build_shufti_tables(needles: &[u8]) -> ([u8; 16], [u8; 16], u8) {
    let mut low_tab = [0u8; 16];
    let mut high_tab = [0u8; 16];

    for (i, &byte) in needles.iter().enumerate() {
        let bit = 1u8 << i;
        low_tab[(byte & 0x0f) as usize] |= bit;
        high_tab[(byte >> 4) as usize] |= bit;
    }

    let bit_mask = (1u32 << needles.len()).wrapping_sub(1) as u8;
    (low_tab, high_tab, bit_mask)
}

fn build_shufti_table_slow(targets: &[u8]) -> Option<([u8; 16], [u8; 16], u8)> {
    let mut low_tab = [0u8; 16];
    let mut high_tab = [0u8; 16];
    let mut current_bit = 0;
    let mut assigned_mask = 0u8;

    for &c in targets {
        if current_bit >= 8 {
            return None;
        }

        let hi = (c >> 4) as usize;
        let lo = (c & 0x0F) as usize;

        let mut placed = false;
        for b in 0..current_bit {
            if is_safe(b, c, targets, &low_tab, &high_tab) {
                low_tab[lo] |= 1 << b;
                high_tab[hi] |= 1 << b;
                placed = true;
                break;
            }
        }

        if !placed && current_bit < 8 {
            low_tab[lo] |= 1 << current_bit;
            high_tab[hi] |= 1 << current_bit;
            assigned_mask |= 1 << current_bit;
            current_bit += 1;
        }
    }
    Some((low_tab, high_tab, assigned_mask))
}

/// for a bit(bucket), suppose there are chars [s1, s2, s3]
/// we have the low parts and high parts:
///     [s1_low, s2_low, s3_low]
///     [s1_high, s2_high, s3_high]
/// we have to make sure: for any c = high<<4 + s1_low, c must belongs to this bucket
/// in other words, if it's well-formed after adding candidate to the `bit_index`,
/// then it's ok to do so.
fn is_safe(
    bit_index: u8,
    candidate: u8,
    targets: &[u8],
    current_low: &[u8; 16],
    current_high: &[u8; 16],
) -> bool {
    let c_hi = (candidate >> 4) as usize;
    let c_lo = (candidate & 0x0F) as usize;
    let bit = 1 << bit_index;

    for other_hi in 0..16 {
        for other_lo in 0..16 {
            if (current_high[other_hi] & bit != 0) && (current_low[other_lo] & bit != 0) {
                let ghost1 = ((c_hi << 4) | other_lo) as u8;
                let ghost2 = ((other_hi << 4) | c_lo) as u8;

                if !targets.contains(&ghost1) || !targets.contains(&ghost2) {
                    return false;
                }
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test() {
        let input = syn::parse2(quote! {
            #[derive(ShuftiMatcher)]
            #[shufti(set = "abc")]
            pub struct MyMatcher;
        })
        .unwrap();

        let ts = impl_shufti_matcher(&input).unwrap();
        assert_eq!(
            ts.to_string(),
            r#"impl :: shufti_matcher :: ShuftiMatch for MyMatcher { const SET : & 'static str = "abc" ; const NEEDLE_COUNT : usize = 3usize ; # [inline (always)] fn table () -> :: shufti_matcher :: ShuftiTable { :: shufti_matcher :: ShuftiTable { low_tab : [0u8 , 1u8 , 2u8 , 4u8 , 0u8 , 0u8 , 0u8 , 0u8 , 0u8 , 0u8 , 0u8 , 0u8 , 0u8 , 0u8 , 0u8 , 0u8] , high_tab : [0u8 , 0u8 , 0u8 , 0u8 , 0u8 , 0u8 , 7u8 , 0u8 , 0u8 , 0u8 , 0u8 , 0u8 , 0u8 , 0u8 , 0u8 , 0u8] , bit_mask : 7u8 , } } }"#
        );
    }
}
