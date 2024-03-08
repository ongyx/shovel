use tabled::settings::object::Rows;
use tabled::settings::themes::Colorization;
use tabled::settings::{Color, Style};
use tabled::{Table, Tabled};

pub fn tableify<I, T>(iter: I) -> Table
where
    I: IntoIterator<Item = T>,
    T: Tabled,
{
    let mut table = Table::new(iter);

    table
        .with(Style::empty())
        .with(Colorization::exact([Color::FG_GREEN], Rows::first()));

    table
}
