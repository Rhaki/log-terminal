use log_terminal::{SplitBy, SplitFilter, TerminalLog};

#[test]
fn name() {
    TerminalLog::new(SplitBy::Name(SplitFilter::none()))
        // .customize_fmt_layer(|layer| layer.with_target(false).without_time())
        .with_max_level(tracing::Level::INFO)
        .finish();

    let value = 10;

    let parent = tracing::span!(tracing::Level::INFO, "parent");

    for _ in 0..100 {
        tracing::debug!("hello");
        tracing::info!("hello");
        tracing::info!(target: "pippo", parent: &parent, amount = value, "hello");
        std::thread::sleep(std::time::Duration::from_millis(200));
        log2();
        log3();
        log4::log4();
    }

    ratatui::restore();
}

#[test]
fn mouse() {
    TerminalLog::new(SplitBy::Name(SplitFilter::none()))
        // .customize_fmt_layer(|layer| layer.with_target(false).without_time())
        .with_max_level(tracing::Level::INFO)
        .finish();

        let parent = tracing::span!(tracing::Level::INFO, "parent");

    for i in 0..200 {
        tracing::info!(target: "pippo", parent: &parent, amount = i, "hello");
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

#[tracing::instrument("log2", skip_all)]
fn log2() {
    tracing::info!("log2");
}

#[tracing::instrument("log3", skip_all)]
fn log3() {
    log2();
}

pub mod log4 {
    pub fn log4() {
        tracing::info!("log4");
    }
}
