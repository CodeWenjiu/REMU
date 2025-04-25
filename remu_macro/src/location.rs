#[macro_export]
macro_rules! location {
    () => {
        format!("File, Line: {}, {}", file!(), line!())
    };
}

#[macro_export]
macro_rules! log_trace {
    ($fmt:expr) => {
        format!("{} at {}", $fmt, $crate::location!())
    };
    ($fmt:expr, $($arg:expr),*) => {
        format!("{} at {}", format!($fmt, $($arg),*), $crate::location!())
    };
}

#[macro_export]
macro_rules! log_level {
    ($level:expr) => {
        Logger::show(&$crate::log_trace!("Debug"), $level)
    };
    ($msg:expr, $level:expr) => {
        Logger::show(&$crate::log_trace!($msg), $level)
    };
    ($fmt:expr, $($arg:expr),+, $level:expr) => {
        Logger::show(&$crate::log_trace!($fmt, $($arg),+), $level)
    };
}

#[macro_export]
macro_rules! log_debug {
    () => {
        $crate::log_level!(Logger::DEBUG)
    };
    ($msg:expr) => {
        $crate::log_level!($msg, Logger::DEBUG)
    };
    ($fmt:expr, $($arg:expr),+) => {
        $crate::log_level!($fmt, $($arg),+, Logger::DEBUG)
    };
}

#[macro_export]
macro_rules! log_error {
    () => {
        $crate::log_level!(Logger::ERROR)
    };
    ($msg:expr) => {
        $crate::log_level!($msg, Logger::ERROR)
    };
    ($fmt:expr, $($arg:expr),+) => {
        $crate::log_level!($fmt, $($arg),+, Logger::ERROR)
    };
}

#[macro_export]
macro_rules! log_err {
    ($expr:expr) => {
        $expr.map_err(|e| {
            $crate::log_error!(e.to_string())
        })
    };
    ($expr:expr, $err:expr) => {
        $expr.map_err(|e| {
            $crate::log_error!(e.to_string());
            $err
        })
    };
}

#[macro_export]
macro_rules! log_todo {
    () => {
        $crate::log_error!("TODO")
    };
}