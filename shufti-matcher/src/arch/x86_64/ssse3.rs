#[inline(always)]
pub unsafe fn bitmask_16b(
    low_tab: &[u8; 16],
    high_tab: &[u8; 16],
    bit_mask: u8,
    data: &[u8; 16],
) -> u16 {
    use core::arch::x86_64::*;

    unsafe {
        let l_tab = _mm_loadu_si128(low_tab.as_ptr() as *const __m128i);
        let h_tab = _mm_loadu_si128(high_tab.as_ptr() as *const __m128i);
        let input = _mm_loadu_si128(data.as_ptr() as *const __m128i);

        let nib_mask = _mm_set1_epi8(0x0f_u8 as i8);
        let lo = _mm_and_si128(input, nib_mask);
        let hi = _mm_and_si128(_mm_srli_epi16(input, 4), nib_mask);

        let lo_sf = _mm_shuffle_epi8(l_tab, lo);
        let hi_sf = _mm_shuffle_epi8(h_tab, hi);
        let v = _mm_and_si128(lo_sf, hi_sf);

        let bm = _mm_set1_epi8(bit_mask as i8);
        let masked = _mm_and_si128(v, bm);
        let nonzero = _mm_andnot_si128(
            _mm_cmpeq_epi8(masked, _mm_setzero_si128()),
            _mm_set1_epi8(-1),
        );

        _mm_movemask_epi8(nonzero) as u16
    }
}
