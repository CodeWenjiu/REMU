#[macro_export]
macro_rules! log_err {
    ($expr:expr) => {
        $expr.map_err(|e| {
            Logger::show(&e.to_string(), Logger::ERROR);
        })
    };
    ($expr:expr, $err:expr) => {
        $expr.map_err(|e| {
            Logger::show(&e.to_string(), Logger::ERROR);
            $err
        })
    };
}
