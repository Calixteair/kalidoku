use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn init() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,kalidoku=debug"));
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().json().with_target(true).with_current_span(false))
        .init();
}
