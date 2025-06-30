use {
    crate::{draw::draw_thread, inputs::inputs_thread},
    std::{
        collections::VecDeque,
        io::{self, Write},
        sync::{Arc, Mutex, Once, mpsc},
        thread,
    },
    tracing::Subscriber,
    tracing_subscriber::{
        Layer, Registry,
        fmt::{
            Layer as FmtLayer, MakeWriter,
            format::{DefaultFields, Format},
        },
        layer::{Layered, SubscriberExt},
    },
};

static TRACING: Once = Once::new();

const NAME_NOT_FOUND: &str = "undefined";

pub struct TerminalLog<N, E> {
    rl: RedirectLayer,
    fmt_layer: FmtLayer<Layered<RedirectLayer, Registry>, N, E, ChannelWriter>,
}

impl TerminalLog<DefaultFields, Format> {
    pub fn new(split_by: SplitBy) -> TerminalLog<DefaultFields, Format> {
        let (rl, cw) = RedirectLayer::new(split_by);
        let fmt_layer = FmtLayer::new().with_writer(cw);
        TerminalLog { rl, fmt_layer }
    }
}

impl<N, E> TerminalLog<N, E>
where
    N: Send + Sync + 'static,
    E: Send + Sync + 'static,
    FmtLayer<Layered<RedirectLayer, Registry>, N, E, ChannelWriter>:
        Layer<Layered<RedirectLayer, Registry>>,
{
    pub fn customize_fmt_layer<N1, E1>(
        self,
        closure: impl FnOnce(
            FmtLayer<Layered<RedirectLayer, Registry>, N, E, ChannelWriter>,
        )
            -> FmtLayer<Layered<RedirectLayer, Registry>, N1, E1, ChannelWriter>,
    ) -> TerminalLog<N1, E1> {
        TerminalLog {
            rl: self.rl,
            fmt_layer: closure(self.fmt_layer),
        }
    }

    pub fn with_max_level(mut self, level: tracing::Level) -> TerminalLog<N, E> {
        self.rl.max_level = level;
        self
    }

    pub fn finish(self) {
        TRACING.call_once(|| {
            let subscriber = tracing_subscriber::registry()
                .with(self.rl)
                .with(self.fmt_layer);

            tracing::subscriber::set_global_default(subscriber)
                .expect("global subscriber already set");
        });
    }
}

pub enum SplitBy {
    Target(SplitFilter),
    TargetPrefix(SplitFilter),
    Name(SplitFilter),
}

pub enum SplitFilter {
    WhiteList(&'static [&'static str]),
    BlackList(&'static [&'static str]),
    None,
}

impl SplitFilter {
    pub fn filter<'a>(&self, target: String) -> Option<String> {
        match self {
            SplitFilter::WhiteList(items) => {
                if items.contains(&target.as_str()) {
                    Some(target)
                } else {
                    None
                }
            },
            SplitFilter::BlackList(items) => {
                if items.contains(&target.as_str()) {
                    None
                } else {
                    Some(target)
                }
            },
            SplitFilter::None => Some(target),
        }
    }
}

pub struct RedirectLayer {
    max_level: tracing::Level,
    split_by: SplitBy,
    events: Arc<Mutex<VecDeque<Option<String>>>>,
}

impl RedirectLayer {
    pub fn new(split_by: SplitBy) -> (Self, ChannelWriter) {
        let (tx, rx) = mpsc::channel();

        let events: Arc<Mutex<VecDeque<Option<String>>>> = Default::default();

        let _events = events.clone();

        thread::spawn(|| inputs_thread());

        thread::spawn(move || draw_thread(events, rx));

        (
            Self {
                max_level: tracing::Level::DEBUG,
                events: _events,
                split_by,
            },
            ChannelWriter { tx },
        )
    }

    fn filter<S>(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) -> Option<String>
    where
        S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    {
        match &self.split_by {
            SplitBy::Target(filter) => filter.filter(event.metadata().target().to_string()),
            SplitBy::TargetPrefix(filter) => {
                let full_target = event.metadata().target();

                let target = full_target
                    .split("::")
                    .next()
                    .unwrap_or(full_target)
                    .to_string();
                filter.filter(target)
            },
            SplitBy::Name(filter) => {
                let str = if let Some(scope) = _ctx.event_scope(event) {
                    if let Some(span) = scope.from_root().next() {
                        span.name().to_string()
                    } else {
                        NAME_NOT_FOUND.to_string()
                    }
                } else {
                    NAME_NOT_FOUND.to_string()
                };
                filter.filter(str)
            },
        }
    }
}

impl<S> Layer<S> for RedirectLayer
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn enabled(
        &self,
        metadata: &tracing::Metadata<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        metadata.level() <= &self.max_level
    }

    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let name = self.filter(event, _ctx);

        self.events.lock().unwrap().push_back(name);
    }
}

pub struct ChannelWriter {
    tx: mpsc::Sender<Vec<u8>>,
}

impl<'a> MakeWriter<'a> for ChannelWriter {
    type Writer = &'a Self;

    fn make_writer(&'a self) -> Self::Writer {
        self
    }
}

impl<'a> Write for &'a ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.tx.send(buf.to_vec()).unwrap();
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
