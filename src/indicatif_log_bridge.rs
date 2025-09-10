// Copied and adapted from https://crates.io/crates/indicatif-log-bridge
// Licensed under MIT license
// Copied because of dependency mismatch, they used old version of indicatif while we
// use the up-to-date one

use indicatif::MultiProgress;
use log::Log;

/// Wraps a `MultiProgress` and a Log implementor,
/// calling .suspend on the `MultiProgress` while writing the log message,
/// thereby preventing progress bars and logs from getting mixed up.
///
/// You simply have to add every `ProgressBar` to the passed `MultiProgress`.
pub struct LogWrapper<L: Log> {
    bar: MultiProgress,
    log: L,
}

impl<L: Log + 'static> LogWrapper<L> {
    pub fn new(bar: MultiProgress, log: L) -> Self {
        Self { bar, log }
    }

    /// Installs this as the global logger.
    ///
    /// Tries to find the correct argument to `log::set_max_level`
    /// by reading the logger configuration,
    /// you may want to set it manually though.
    /// For more details read the [known issues](index.html#wrong-global-log-level).
    pub fn try_init(self) -> Result<(), log::SetLoggerError> {
        use log::LevelFilter::{Debug, Error, Info, Off, Trace, Warn};
        let levels = [Off, Error, Warn, Info, Debug, Trace];

        for level_filter in levels.iter().rev() {
            let Some(level) = level_filter.to_level() else {
                // off is the last level, just do nothing in that case
                continue;
            };

            let meta = log::Metadata::builder().level(level).build();
            if self.enabled(&meta) {
                log::set_max_level(*level_filter);
                break;
            }
        }

        log::set_boxed_logger(Box::new(self))
    }
    #[allow(dead_code)]
    pub fn multi(&self) -> MultiProgress {
        self.bar.clone()
    }
}

impl<L: Log> Log for LogWrapper<L> {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.log.enabled(metadata)
    }

    fn log(&self, record: &log::Record) {
        // do an early check for enabled to not cause unnescesary suspends
        if self.log.enabled(record.metadata()) {
            self.bar.suspend(|| self.log.log(record));
        }
    }

    fn flush(&self) {
        self.log.flush();
    }
}
