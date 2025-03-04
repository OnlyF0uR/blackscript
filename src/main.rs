use std::sync::LazyLock;

use iced::{
    Element,
    Length::Fill,
    Task,
    widget::{column, container, row, scrollable},
};
use styles::text_window_style;

mod styles;

static SCROLLABLE_ID: LazyLock<scrollable::Id> = LazyLock::new(scrollable::Id::unique);

struct Blackscript {
    content: iced::widget::text_editor::Content,
    current_scroll_offset: scrollable::RelativeOffset,
    content_scroll_bound: f32,
}

impl Default for Blackscript {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Edit(iced::widget::text_editor::Action),

    ScrollToBeginning,
    ScrollToEnd,
    Scrolled(scrollable::Viewport),
}

impl Blackscript {
    pub fn new() -> Self {
        Self {
            content: iced::widget::text_editor::Content::with_text(""),
            current_scroll_offset: scrollable::RelativeOffset::START,
            content_scroll_bound: 0.0,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let input = scrollable(
            iced::widget::text_editor(&self.content)
                .on_action(Message::Edit)
                .font(iced::Font::with_name("Courier New"))
                .style(|theme, _s| text_window_style(theme)),
        )
        .width(Fill)
        .height(Fill)
        .id(SCROLLABLE_ID.clone())
        .on_scroll(Message::Scrolled);

        let counts = {
            let words = self.content.text().split_whitespace().count();
            let characters = self.content.text().chars().count() - 1;

            iced::widget::text(format!("Words: {}, Characters: {}", words, characters))
        };

        let positions = {
            let (line, column) = self.content.cursor_position();
            let all_lines = self.content.line_count();
            iced::widget::text(format!(
                "Line: {}/{}, Column: {}",
                line + 1,
                all_lines,
                column
            ))
        };

        let status_bar = row![counts, iced::widget::horizontal_space(), positions];
        container(column![input, status_bar].spacing(10))
            .padding(10)
            .into()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Edit(action) => {
                self.content.perform(action);
                Task::none()
            }
            Message::ScrollToBeginning => {
                self.current_scroll_offset = scrollable::RelativeOffset::START;

                scrollable::snap_to(SCROLLABLE_ID.clone(), self.current_scroll_offset)
            }
            Message::ScrollToEnd => {
                self.current_scroll_offset = scrollable::RelativeOffset::END;

                scrollable::snap_to(SCROLLABLE_ID.clone(), self.current_scroll_offset)
            }
            Message::Scrolled(viewport) => {
                self.current_scroll_offset = viewport.relative_offset();

                // Scroll to bottom if at the last line and the content bounds height
                // just increased relative to previous content bounds height.
                let cbounds = viewport.content_bounds();
                if cbounds.height > 0.0 && cbounds.height > self.content_scroll_bound {
                    let lc = self.content.line_count();
                    let cp = self.content.cursor_position().0;
                    if lc == cp + 1 && self.current_scroll_offset != scrollable::RelativeOffset::END
                    {
                        self.content_scroll_bound = cbounds.height;

                        self.current_scroll_offset = scrollable::RelativeOffset::END;
                        return scrollable::snap_to(
                            SCROLLABLE_ID.clone(),
                            self.current_scroll_offset,
                        );
                    }
                }

                self.content_scroll_bound = cbounds.height;
                Task::none()
            }
        }
    }
}

fn main() -> iced::Result {
    iced::application("Blackscript", Blackscript::update, Blackscript::view)
        // .window(iced::window::Settings {
        //     size: Size::new(1024.0, 768.0),
        //     resizable: true,
        //     decorations: true,
        //     transparent: false,
        //     min_size: None,
        //     max_size: None,
        //     visible: true,
        //     exit_on_close_request: true,
        //     platform_specific: PlatformSpecific::default(),
        //     icon: None,
        //     level: iced::window::Level::Normal,
        //     position: iced::window::Position::default(),
        // })
        // .default_font(iced::Font::with_name("Courier New"))
        .run()
}
