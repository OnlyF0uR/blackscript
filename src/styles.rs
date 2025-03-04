use iced::{Background, Border, Color, Theme, widget};

pub fn text_window_style(theme: &Theme) -> widget::text_editor::Style {
    let palette = theme.extended_palette();

    widget::text_editor::Style {
        background: Background::Color(palette.background.base.color),
        // background: Background::Color(Color::WHITE),
        border: Border {
            radius: 2.0.into(),
            width: 1.0,
            // color: palette.background.strong.color,
            color: Color::TRANSPARENT,
        },
        icon: palette.background.weak.text,
        placeholder: palette.background.strong.color,
        value: palette.background.base.text,
        selection: palette.primary.weak.color,
    }
}
