use tabled::settings::object::Rows;
use tabled::settings::peaker::PriorityMax;
use tabled::settings::themes::Colorization;
use tabled::settings::{Color, Style, Width};
use tabled::{Table, Tabled};
use terminal_size;

pub fn tableify<I, T>(iter: I) -> Table
where
    I: IntoIterator<Item = T>,
    T: Tabled,
{
    let mut table = Table::new(iter);
    let width = term_size().0 as usize;

    table
        .with(Style::empty())
        .with(
            Width::truncate(width)
                .priority::<PriorityMax>()
                .suffix("..."),
        )
        .with(Colorization::exact([Color::FG_GREEN], Rows::first()));

    table
}

pub fn term_size() -> (u16, u16) {
    match terminal_size::terminal_size() {
        Some((w, h)) => (w.0, h.0),
        // Ol' reliable
        None => (80, 24),
    }
}
