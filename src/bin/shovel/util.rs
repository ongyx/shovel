use tabled;
use tabled::settings;
use tabled::settings::{object, peaker, themes};
use terminal_size;

/// Returns a formatted table for an iterator over tabular items.
pub fn tableify<I, T>(iter: I, transpose: bool) -> tabled::Table
where
    I: IntoIterator<Item = T>,
    T: tabled::Tabled,
{
    use object::{Columns, Rows};
    use peaker::PriorityMax;
    use settings::{Color, Style, Width};
    use themes::Colorization;

    let mut table = if transpose {
        let builder = tabled::Table::builder(iter).index().column(0).transpose();
        let mut table = builder.build();

        // Colour the first column.
        table.with(Colorization::exact([Color::FG_GREEN], Columns::first()));

        table
    } else {
        let mut table = tabled::Table::new(iter);

        // Colour the first row.
        table.with(Colorization::exact([Color::FG_GREEN], Rows::first()));

        table
    };

    let width = term_size().0 as usize;

    table.with(Style::empty()).with(
        Width::truncate(width)
            .priority::<PriorityMax>()
            .suffix("..."),
    );

    table
}

/// Returns the current terminal size.
pub fn term_size() -> (u16, u16) {
    match terminal_size::terminal_size() {
        Some((w, h)) => (w.0, h.0),
        // Ol' reliable
        None => (80, 24),
    }
}

/// Parses and returns a two-tuple (bucket, app) for a slash-seperated name.
pub fn parse_app(name: &str) -> (&str, &str) {
    name.split_once('/').unwrap_or(("", name))
}
