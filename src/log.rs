use {
    crate::{
        draw::{DrawEvent, MAX_LINES, draw_thread},
        inputs::inputs_thread,
    },
    std::{
        collections::VecDeque,
        io::{self, Write},
        marker::PhantomData,
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

pub struct LogTerminal<N, E, V, S> {
    rl: RedirectLayer<V, S>,
    fmt_layer: FmtLayer<Layered<RedirectLayer<V, S>, Registry>, N, E, ChannelWriter>,
}

impl<V, S> LogTerminal<DefaultFields, Format, V, S>
where
    S: PartialEq<String>,
    V: AsRef<[S]>,
{
    pub fn new(split_by: SplitBy<V, S>) -> LogTerminal<DefaultFields, Format, V, S> {
        let (rl, cw) = RedirectLayer::new(split_by);
        let fmt_layer = FmtLayer::new().with_writer(cw);
        LogTerminal { rl, fmt_layer }
    }
}

impl<N, E, V, S> LogTerminal<N, E, V, S>
where
    N: Send + Sync + 'static,
    E: Send + Sync + 'static,
    S: PartialEq<String>,
    V: AsRef<[S]>,
    RedirectLayer<V, S>: Send + Sync + 'static,
    FmtLayer<Layered<RedirectLayer<V, S>, Registry>, N, E, ChannelWriter>:
        Layer<Layered<RedirectLayer<V, S>, Registry>>,
{
    pub fn customize_fmt_layer<N1, E1>(
        self,
        closure: impl FnOnce(
            FmtLayer<Layered<RedirectLayer<V, S>, Registry>, N, E, ChannelWriter>,
        )
            -> FmtLayer<Layered<RedirectLayer<V, S>, Registry>, N1, E1, ChannelWriter>,
    ) -> LogTerminal<N1, E1, V, S> {
        LogTerminal {
            rl: self.rl,
            fmt_layer: closure(self.fmt_layer),
        }
    }

    pub fn with_max_level(mut self, level: tracing::Level) -> LogTerminal<N, E, V, S> {
        self.rl.max_level = level;
        self
    }

    pub fn with_max_lines(self, lines: usize) -> LogTerminal<N, E, V, S> {
        *MAX_LINES.lock().unwrap() = lines;
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

pub enum SplitBy<V, S> {
    /// Tabs are splitted by the `target`
    Target(SplitFilter<V, S>),
    /// Tabs are splitted by the `target` prefix
    TargetPrefix(SplitFilter<V, S>),
    /// Tabs are splitted by the `span` prefix
    SpanPrefix(SplitFilter<V, S>),
}

#[non_exhaustive]
pub enum SplitFilter<V, S> {
    /// Based on [`SplitBy`], show only the tabs that are in the whitelist
    WhiteList(V, PhantomData<S>),
    /// Based on [`SplitBy`], show only the tabs that are not in the blacklist
    BlackList(V, PhantomData<S>),
    /// Show all the tabs
    None,
}

impl<V, S> SplitFilter<V, S>
where
    S: PartialEq<String>,
    V: AsRef<[S]>,
{
    /// Based on [`SplitBy`], show only the tabs that are in the whitelist
    pub fn whitelist(items: V) -> Self {
        Self::WhiteList(items, PhantomData)
    }

    /// Based on [`SplitBy`], show only the tabs that are not in the blacklist
    pub fn blacklist(items: V) -> Self {
        Self::BlackList(items, PhantomData)
    }
}

impl SplitFilter<Vec<String>, String> {
    /// Show all the tabs
    pub fn none() -> Self {
        Self::None
    }
}

impl<V, S> SplitFilter<V, S>
where
    S: PartialEq<String>,
    V: AsRef<[S]>,
{
    pub fn filter<'a>(&self, target: String) -> Option<String> {
        match self {
            SplitFilter::WhiteList(items, _) => {
                if items.as_ref().iter().any(|item| item == &target) {
                    Some(target)
                } else {
                    None
                }
            },
            SplitFilter::BlackList(items, _) => {
                if items.as_ref().iter().any(|item| item == &target) {
                    None
                } else {
                    Some(target)
                }
            },
            SplitFilter::None => Some(target),
        }
    }
}

pub struct RedirectLayer<V, S> {
    max_level: tracing::Level,
    split_by: SplitBy<V, S>,
    events: Arc<Mutex<VecDeque<Option<String>>>>,
}

impl<V, S> RedirectLayer<V, S>
where
    S: PartialEq<String>,
    V: AsRef<[S]>,
{
    pub fn new(split_by: SplitBy<V, S>) -> (Self, ChannelWriter) {
        let (tx, rx) = mpsc::channel();

        let events: Arc<Mutex<VecDeque<Option<String>>>> = Default::default();

        let _events = events.clone();
        let _tx = tx.clone();

        thread::spawn(move || inputs_thread(_tx));
        thread::spawn(move || draw_thread(_events, rx));

        (
            Self {
                max_level: tracing::Level::DEBUG,
                events,
                split_by,
            },
            ChannelWriter { tx },
        )
    }

    fn filter<Sub>(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, Sub>,
    ) -> Option<String>
    where
        Sub: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
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
            SplitBy::SpanPrefix(filter) => {
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

impl<S, V, Sub> Layer<Sub> for RedirectLayer<V, S>
where
    Self: 'static,
    S: PartialEq<String>,
    V: AsRef<[S]>,
    Sub: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn enabled(
        &self,
        metadata: &tracing::Metadata<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, Sub>,
    ) -> bool {
        metadata.level() <= &self.max_level
    }

    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, Sub>,
    ) {
        let name = self.filter(event, _ctx);

        self.events.lock().unwrap().push_back(name);
    }
}

pub struct ChannelWriter {
    tx: mpsc::Sender<DrawEvent>,
}

impl<'a> MakeWriter<'a> for ChannelWriter {
    type Writer = &'a Self;

    fn make_writer(&'a self) -> Self::Writer {
        self
    }
}

impl<'a> Write for &'a ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.tx.send(DrawEvent::Trace(buf.to_vec())).unwrap();
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
