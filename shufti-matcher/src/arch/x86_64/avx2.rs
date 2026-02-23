// #[inline(always)]
// pub unsafe fn bitmask_16b(
//     low_tab: &[u8; 16],
//     high_tab: &[u8; 16],
//     bit_mask: u8,
//     data: &[u8; 32],
// ) -> u32 {
//     use core::arch::x86_64::*;

//     let l_tab = _mm256_broadcastsi128_si256(_mm_loadu_si128(low_tab.as_ptr() as *const __m128i));
//     let h_tab = _mm256_broadcastsi128_si256(_mm_loadu_si128(high_tab.as_ptr() as *const __m128i));
//     let input = _mm256_loadu_si256(data.as_ptr() as *const __m256i);

//     let nib_mask = _mm256_set1_epi8(0x0f_u8 as i8);
//     let lo = _mm256_and_si256(input, nib_mask);
//     // logical >>4, then mask — isolates high nibble as 0..15
//     let hi = _mm256_and_si256(_mm256_srli_epi16(input, 4), nib_mask);

//     let lo_sf = _mm256_shuffle_epi8(l_tab, lo);
//     let hi_sf = _mm256_shuffle_epi8(h_tab, hi);
//     let v = _mm256_and_si256(lo_sf, hi_sf);

//     let bm = _mm256_set1_epi8(bit_mask as i8);
//     let masked = _mm256_and_si256(v, bm);
//     let nonzero = _mm256_cmpgt_epi8(masked, _mm256_setzero_si256());

//     _mm256_movemask_epi8(nonzero) as u32
// }