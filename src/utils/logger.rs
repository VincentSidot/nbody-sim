use colored::Colorize;
use log::{Level, LevelFilter, Log};

use super::config::Config;

struct Logger;

impl Logger {
    fn level_allowed(level: Level, filter: LevelFilter) -> bool {
        match filter {
            LevelFilter::Off => false,
            LevelFilter::Error => level <= Level::Error,
            LevelFilter::Warn => level <= Level::Warn,
            LevelFilter::Info => level <= Level::Info,
            LevelFilter::Debug => level <= Level::Debug,
            LevelFilter::Trace => true,
        }
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        let target = metadata.target();
        let filt = Config::get_log_level(target);

        Self::level_allowed(metadata.level(), filt)
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let file = record.file().unwrap_or("unknown");
            let line = record.line().unwrap_or(0);
            let target = record.target();

            let text = match record.level() {
                log::Level::Error => {
                    format!(
                        "{} ({} -> {}:{}) - {}",
                        "[ERRO]".red(),
                        target,
                        file,
                        line,
                        record.args()
                    )
                }
                log::Level::Warn => format!(
                    "{} ({} -> {}:{}) - {}",
                    "[WARN]".yellow(),
                    target,
                    file,
                    line,
                    record.args()
                ),
                log::Level::Info => {
                    format!(
                        "{} ({} -> {}:{}) - {}",
                        "[INFO]".green(),
                        target,
                        file,
                        line,
                        record.args()
                    )
                }
                log::Level::Debug => {
                    format!(
                        "{} ({} -> {}:{}) - {}",
                        "[DBUG]".blue(),
                        target,
                        file,
                        line,
                        record.args()
                    )
                }
                log::Level::Trace => format!(
                    "{} ({} -> {}:{}) - {}",
                    "[TRCE]".purple(),
                    target,
                    file,
                    line,
                    record.args()
                ),
            };
            // Print to stdout for simplicity; in a real application, consider using a more robust logging solution
            println!("{}", text);
        }
    }

    fn flush(&self) {}
}

/// Initializes the logger
///
/// This function sets up the logger with a default level of INFO.
/// The log level can be controlled via the LOG_LEVEL environment variable.
/// For example: `LOG_LEVEL=debug` or `LOG_LEVEL=websearch=trace`
pub fn init_logger() {
    static LOGGER: Logger = Logger;

    // Set global max to the highest level we might emit (so overrides can work).
    let global_max = Config::get_max_log_level();

    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(global_max))
        .expect("Failed to set logger");

    log::info!("Logger initialized");
}
