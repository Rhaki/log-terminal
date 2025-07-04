use log_terminal::{LogTerminal, SplitBy, SplitFilter};

#[test]
fn name() {
    LogTerminal::new(SplitBy::Name(SplitFilter::none()))
        .with_max_level(tracing::Level::INFO)
        .finish();

    let value = 10;

    let parent = tracing::span!(tracing::Level::INFO, "parent");

    loop {
        tracing::debug!("hello");
        tracing::info!("hello");
        tracing::info!(target: "pippo", parent: &parent, amount = value, "hello");
        std::thread::sleep(std::time::Duration::from_millis(200));
        log2();
        log3();
        log4::log4();
    }
}

#[test]
fn max_lines() {
    LogTerminal::new(SplitBy::Name(SplitFilter::none()))
        .with_max_lines(100)
        .finish();

    let parent = tracing::span!(tracing::Level::INFO, "parent");

    let mut i = 0;
    loop {
        tracing::info!(parent: &parent, amount = i, "hello");
        i += 1;
        std::thread::sleep(std::time::Duration::from_millis(20));
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
