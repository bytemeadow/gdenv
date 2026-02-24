use tracing_indicatif::style::ProgressStyle;

use anyhow::Result;
use tracing::field::{Field, Visit};
use tracing::level_filters::LevelFilter;
use tracing::span::Record;
use tracing::{Event, Subscriber};
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::field::RecordFields;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields, FormattedFields};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;

pub fn progress_bar_style() -> Result<ProgressStyle> {
    let style = ProgressStyle::with_template(
        "{spinner:.green} [{elapsed_precise}] {msg} [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})",
    )?;
    let style = style
        .progress_chars("#>-")
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "⠿"]);
    Ok(style)
}

pub fn spinner_style(template: &str) -> Result<ProgressStyle> {
    let style = ProgressStyle::with_template(&format!(
        "{{spinner:.green}} [{{elapsed_precise}}] {}",
        template
    ))?;
    let style = style.tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "⠿"]);
    Ok(style)
}

pub fn initialize_logging() {
    let progress_bar_layer = IndicatifLayer::new();
    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_level(false)
        .with_thread_names(false)
        .with_line_number(false)
        .without_time()
        .event_format(NoSpanFormat)
        .fmt_fields(OnlyMessageField);
    tracing_subscriber::registry()
        .with(LevelFilter::INFO)
        .with(fmt_layer)
        .with(progress_bar_layer)
        .init();
}

pub struct OnlyMessageField;

impl<'writer> FormatFields<'writer> for OnlyMessageField {
    fn format_fields<R: RecordFields>(
        &self,
        mut writer: Writer<'writer>,
        fields: R,
    ) -> std::fmt::Result {
        struct Visitor<'a> {
            writer: &'a mut dyn std::fmt::Write,
        }

        impl<'a> Visit for Visitor<'a> {
            fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
                if field.name() == "message" {
                    // Just write the message, no "message=" prefix
                    let _ = write!(self.writer, "{value:?}");
                }
            }
        }

        let mut visitor = Visitor {
            writer: &mut writer,
        };
        fields.record(&mut visitor);
        Ok(())
    }

    fn add_fields(
        &self,
        _current: &'writer mut FormattedFields<Self>,
        _fields: &Record<'_>,
    ) -> std::fmt::Result {
        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
pub struct NoSpanFormat;

impl<S, N> FormatEvent<S, N> for NoSpanFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        // 1. Format event fields to the writer
        ctx.field_format().format_fields(writer.by_ref(), event)?;

        // 2. Write a newline
        writeln!(writer)
    }
}
