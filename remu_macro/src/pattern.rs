#[macro_export]
macro_rules! generate_mask_and_value {
    ($s:expr) => {{
        let s = $s;
        let mut mask: u32 = 0;
        let mut value: u32 = 0;
        let mut i = 0;
        while i < s.len() {
            let c = s.as_bytes()[i] as char;
            mask <<= 1;
            value <<= 1;
            match c {
                '1' => {
                    mask |= 1;
                    value |= 1;
                }
                '0' => {
                    mask |= 1;
                }
                '?' => {
                    mask |= 1;
                }
                _ => {
                    mask >>= 1;
                    value >>= 1;
                }
            }
            i += 1;
        }
        (mask, value)
    }};
}

#[macro_export]
macro_rules! mask_and_value {
    ($bits:expr) => {{
        use remu_macro::generate_mask_and_value;
        let (mask, value) = generate_mask_and_value!($bits);
        (mask, value)
    }};
}
