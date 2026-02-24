//! Bridge from Bevy's `tracing` spans to Micromegas thread-local spans.
//!
//! Bevy (with the `trace` feature) emits `tracing` spans for every schedule
//! run and every system execution.  This module installs a
//! `tracing_subscriber::Layer` that listens for "schedule" spans and forwards
//! them as Micromegas named-scope events, giving full schedule-level visibility
//! in the trace timeline without any Bevy internals surgery.

use micromegas_tracing::dispatch::{on_begin_named_scope, on_end_named_scope};
use micromegas_tracing::intern_string::intern_string;
use tracing::Subscriber;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

// All bridged schedule spans share a single static source location.
micromegas_tracing::static_span_location!(BRIDGE_LOCATION);

/// Data stored in each schedule span's extensions.
struct ScheduleSpanData {
    name: &'static str,
}

/// Field visitor that extracts the `name` field from a tracing span.
struct NameVisitor {
    name: Option<String>,
}

impl NameVisitor {
    fn new() -> Self {
        Self { name: None }
    }
}

impl Visit for NameVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "name" {
            self.name = Some(format!("{:?}", value));
        }
    }
}

/// A `tracing_subscriber::Layer` that bridges Bevy schedule spans into
/// Micromegas thread-local named-scope events.
pub struct MicromegasBridgeLayer;

impl<S> Layer<S> for MicromegasBridgeLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        if attrs.metadata().name() != "schedule" {
            return;
        }

        let mut visitor = NameVisitor::new();
        attrs.record(&mut visitor);

        let label = visitor.name.unwrap_or_default();
        let interned = intern_string(&label);

        if let Some(span) = ctx.span(id) {
            span.extensions_mut()
                .insert(ScheduleSpanData { name: interned });
        }
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            let extensions = span.extensions();
            if let Some(data) = extensions.get::<ScheduleSpanData>() {
                on_begin_named_scope(&BRIDGE_LOCATION, data.name);
            }
        }
    }

    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            let extensions = span.extensions();
            if let Some(data) = extensions.get::<ScheduleSpanData>() {
                on_end_named_scope(&BRIDGE_LOCATION, data.name);
            }
        }
    }
}
