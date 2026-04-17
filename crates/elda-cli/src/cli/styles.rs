use clap::builder::styling::{Color, Effects, RgbColor, Style, Styles};

pub(super) fn clap_styles() -> Styles {
    Styles::styled()
        .header(
            Style::new()
                .fg_color(Some(Color::Rgb(RgbColor(255, 255, 255))))
                .effects(Effects::BOLD),
        )
        .usage(
            Style::new()
                .fg_color(Some(Color::Rgb(RgbColor(255, 255, 255))))
                .effects(Effects::BOLD),
        )
        .literal(
            Style::new()
                .fg_color(Some(Color::Rgb(RgbColor(171, 255, 67))))
                .effects(Effects::BOLD),
        )
        .placeholder(Style::new().fg_color(Some(Color::Rgb(RgbColor(255, 255, 0)))))
        .valid(Style::new().fg_color(Some(Color::Rgb(RgbColor(171, 255, 67)))))
        .invalid(
            Style::new()
                .fg_color(Some(Color::Rgb(RgbColor(255, 255, 255))))
                .effects(Effects::BOLD),
        )
        .error(
            Style::new()
                .fg_color(Some(Color::Rgb(RgbColor(255, 255, 255))))
                .effects(Effects::BOLD),
        )
}
