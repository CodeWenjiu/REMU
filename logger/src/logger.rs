use log::{debug, info, trace, warn};
use owo_colors::OwoColorize;
use tracing::error;
use tracing_appender::{non_blocking, rolling};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, Registry};

pub enum Logger{
    TRACE,
    DEBUG,
    INFO ,
    WARN ,
    ERROR,
    SUCCESS,
    IMPORTANT,
    CONGRATULATIONS,
}

impl From<tracing::Level> for Logger {
    fn from(level: tracing::Level) -> Self {
        match level {
            tracing::Level::TRACE => Logger::TRACE,
            tracing::Level::DEBUG => Logger::DEBUG,
            tracing::Level::INFO => Logger::INFO,
            tracing::Level::WARN => Logger::WARN,
            tracing::Level::ERROR => Logger::ERROR,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FeatureState {
    /// 功能在编译时被禁用，相关代码不会参与构建
    Disabled,    // 或 Off
    /// 功能在编译时被启用，但在运行时默认关闭
    Inactive,    // 或 Standby
    /// 功能在编译时被启用，且在运行时默认开启
    Active,      // 或 On
}

impl FeatureState {
    pub fn to_display(&self) -> String {
        match self {
            Self::Disabled => format!("[{}]", "OFF".red()),
            Self::Inactive => format!("[{}]", "STANDBY".yellow()),
            Self::Active => format!("[{}]", "ON".green()),
        }
    }
}

impl From<bool> for FeatureState {
    fn from(value: bool) -> Self {
        if value { Self::Active } else { Self::Inactive }
    }
}

impl Logger {
    pub fn new() -> Result<(), ()> {
        let file_appender = rolling::never("target/logs", ".log");
        let (non_blocking_appender, _guard) = non_blocking(file_appender);
        let file_layer = fmt::layer()
            .with_ansi(false)
            .with_writer(non_blocking_appender);
            
        Registry::default()
            .with(file_layer)
            .init();

        Ok(())
    }

    pub fn format(message: &str, level: Logger) -> String {
        match level {
            Logger::TRACE           => format!("⏭️  {}", message.white()),
            Logger::INFO            => format!("ℹ️  {}", message.blue()),
            Logger::WARN            => format!("⚠️  {}", message.yellow()),
            Logger::DEBUG           => format!("🐞 {}", message.magenta()),
            Logger::ERROR           => format!("❌ {}", message.red()),
            Logger::SUCCESS         => format!("✅ {}", message.green()),
            Logger::IMPORTANT       => format!("✨ {}", message.purple()),
            Logger::CONGRATULATIONS => format!("🎉 {}", message.purple()),
        }
    }

    pub fn show(message: &str, level: Logger) {
        let formatted = Logger::format(message, level);

        println!("{}", formatted);
    }

    pub fn function(function_name: &str, state: FeatureState) {
        println!("🔧 {}{}{}", "function ".blue(), function_name.magenta(), state.to_display());
    }

    pub fn log(message: &str, level: tracing::Level) {
        match level {
            tracing::Level::TRACE => trace!("{}", message),
            tracing::Level::DEBUG => debug!("{}", message),
            tracing::Level::INFO => info!("{}", message),
            tracing::Level::WARN => warn!("{}", message),
            tracing::Level::ERROR => error!("{}", message),
        }

        Logger::show(message, level.into());
    }
}
