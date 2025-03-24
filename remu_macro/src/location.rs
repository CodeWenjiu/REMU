#[macro_export]
macro_rules! location {
    () => {
        format!("File, Line: {}, {}", file!(), line!())
    };
}

#[macro_export]
macro_rules! log_trace {
    ($msg:expr) => {
        format!("{} at {}", $msg, $crate::location!())
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
}

#[macro_export]
macro_rules! log_debug {
    () => {
        $crate::log_level!(Logger::DEBUG)
    };
    ($msg:expr) => {
        $crate::log_level!($msg, Logger::DEBUG)
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
}

#[macro_export]
macro_rules! log_todo {
    () => {
        $crate::log_error!("TODO")
    };
}