#[inline(always)]
pub unsafe fn bitmask_16b(
    low_tab: &[u8; 16],
    high_tab: &[u8; 16],
    bit_mask: u8,
    data: &[u8; 16],
) -> u16 {
    let mut mask = 0u16;
    for (i, &b) in data.iter().enumerate() {
        let lo = (b & 0x0f) as usize;
        let hi = (b >> 4) as usize;
        if (low_tab[lo] & high_tab[hi] & bit_mask) != 0 {
            mask |= 1 << i;
        }
    }
    mask
}
