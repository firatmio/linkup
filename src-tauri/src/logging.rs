//! Loglama kurulumu (PLAN.md §2.14).
//!
//! Log hem konsola hem de veri dizinindeki rotasyonlu dosyaya yazılır.
//! Loglara mesaj içeriği, dosya içeriği, private key veya SAS kodu ASLA yazılmaz.

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::paths::AppPaths;

/// Log yazıcısını canlı tutan guard. `main` boyunca yaşamalıdır; düşerse
/// arka plandaki yazma thread'i kapanır ve loglar kaybolur.
pub struct LogGuard(#[allow(dead_code)] WorkerGuard);

pub fn init(paths: &AppPaths, level_override: Option<&str>) -> LogGuard {
    let default_level = if cfg!(debug_assertions) {
        "debug"
    } else {
        "info"
    };
    let level = level_override.unwrap_or(default_level);

    // Öncelik: RUST_LOG > --log-level > yapı tipine göre varsayılan.
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("linkup_lib={level},linkup={level},warn")));

    let file_appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("linkup")
        .filename_suffix("log")
        .max_log_files(7)
        .build(&paths.log_dir)
        .expect("log dosyası oluşturulamadı");

    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true).with_ansi(true))
        .with(
            fmt::layer()
                .with_target(true)
                .with_ansi(false)
                .with_writer(non_blocking),
        )
        .init();

    tracing::info!(
        profile = paths.profile_label(),
        data_dir = %paths.data_dir.display(),
        quic_port = paths.quic_port,
        version = env!("CARGO_PKG_VERSION"),
        "LinkUp başlatılıyor"
    );

    LogGuard(guard)
}
