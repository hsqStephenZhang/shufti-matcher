
cfg_if::cfg_if! {
    if #[cfg(target_feature="ssse3")] {
        mod ssse3;
        pub use ssse3::*;
    } else {
        mod fallback;
        pub use fallback::*;
    }
}
