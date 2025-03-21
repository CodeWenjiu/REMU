#[macro_export]
macro_rules! location {
    ($($name:ident),+) => {
        $(
            pub mod $name;
        )+
    };
}
