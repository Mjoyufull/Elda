use clap::builder::styling::{Color, Effects, RgbColor, Style, Styles};

pub(super) fn clap_styles() -> Styles {
    Styles::styled()
        .header(
            Style::new()
                .fg_color(Some(Color::Rgb(RgbColor(171, 178, 191))))
                .effects(Effects::BOLD),
        )
        .usage(
            Style::new()
                .fg_color(Some(Color::Rgb(RgbColor(171, 178, 191))))
                .effects(Effects::BOLD),
        )
        .literal(
            Style::new()
                .fg_color(Some(Color::Rgb(RgbColor(152, 195, 121))))
                .effects(Effects::BOLD),
        )
        .placeholder(Style::new().fg_color(Some(Color::Rgb(RgbColor(229, 192, 123)))))
        .valid(Style::new().fg_color(Some(Color::Rgb(RgbColor(152, 195, 121)))))
        .invalid(
            Style::new()
                .fg_color(Some(Color::Rgb(RgbColor(171, 178, 191))))
                .effects(Effects::BOLD),
        )
        .error(
            Style::new()
                .fg_color(Some(Color::Rgb(RgbColor(171, 178, 191))))
                .effects(Effects::BOLD),
        )
}
