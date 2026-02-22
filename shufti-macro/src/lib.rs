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

    if needles.is_empty() || needles.len() > 8 {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "shufti set must have 1–8 bytes",
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
    let (low_tab, high_tab, bit_mask) = build_shufti_tables(&needles);

    let low_tab_tokens = low_tab.iter().map(|b| quote! { #b u8 });
    let high_tab_tokens = high_tab.iter().map(|b| quote! { #b u8 });

    let needle_len = needles.len();
    let set_repr = set_str.clone();

    Ok(quote! {
        impl #impl_generics ::shufti_matcher::ShuftiMatcher for #name #ty_generics #where_clause {
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
