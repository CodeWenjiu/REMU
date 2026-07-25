#[macro_export]
macro_rules! mod_pub {
    // Shorthand: mod_pub!(crate, A, B);
    (crate, $($name:ident),+ $(,)?) => {
        $(
            pub(crate) mod $name;
        )+
    };
    // Shorthand: mod_pub!(super, A, B);
    (super, $($name:ident),+ $(,)?) => {
        $(
            pub(super) mod $name;
        )+
    };
    // Explicit visibility: mod_pub!(pub(self), A, B);
    ($vis:vis, $($name:ident),+ $(,)?) => {
        $(
            $vis mod $name;
        )+
    };
    // Default to `pub`: mod_pub!(A, B, C);
    ($($name:ident),+ $(,)?) => {
        $(
            pub mod $name;
        )+
    };
}

/// Declare private modules (crate-internal only).
/// Expands to `mod A; mod B; ...` — no re-exports, no flattening.
#[macro_export]
macro_rules! mod_prv {
    [ $( $name:ident $(,)? )+ ] => {
        $(
            mod $name;
        )+
    };
}
