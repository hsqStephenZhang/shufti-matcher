use core::arch::aarch64::*;

pub unsafe fn bitmask_16b(low_tab: &[u8; 16], high_tab: &[u8; 16], bit_mask: u8, data: &[u8; 16]) -> u16 {
    unsafe {
        let l_tab = vld1q_u8(low_tab.as_ptr());
        let h_tab = vld1q_u8(high_tab.as_ptr());
        let input = vld1q_u8(data.as_ptr());

        let low_mask = vmovq_n_u8(0x0f);
        let lo = vandq_u8(input, low_mask);
        let hi = vshrq_n_u8(input, 4);

        let lo_sf = vqtbl1q_u8(l_tab, lo);
        let hi_sf = vqtbl1q_u8(h_tab, hi);
        let v = vandq_u8(lo_sf, hi_sf);

        // vtstq_u8: sets byte to 0xFF if (v & bit_mask) != 0
        let matches = vtstq_u8(v, vmovq_n_u8(bit_mask));

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
