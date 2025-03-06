use std::sync::LazyLock;
use std::time::Duration;

use iced::{
    Element, Event, Font, Length, Subscription, Task,
    widget::{column, row, scrollable, text},
};
use widgets::textarea::TextEditorMessage;
use widgets::textarea::TextEditorWidget;

mod widgets;

static SCROLLABLE_ID: LazyLock<scrollable::Id> = LazyLock::new(scrollable::Id::unique);

struct Blackscript {
    text_editor: TextEditorWidget,
    cursor_visible: bool,
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
    EventOccurred(Event),
    EditorEvent(TextEditorMessage),
    CursorBlink,
    // ScrollToBeginning,
    // ScrollToEnd,
    Scrolled(scrollable::Viewport),
}

impl Blackscript {
    pub fn new() -> Self {
        Self {
            text_editor: TextEditorWidget::new()
                .with_font(Font::with_name("Courier New"))
                .with_font_size(16.0),
            cursor_visible: true,
            current_scroll_offset: scrollable::RelativeOffset::START,
            content_scroll_bound: 0.0,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        // Create a column for all lines
        let mut line_column = column![];

        let (hpos, vpos) = self.text_editor.cursor_position();

        // Loop over each line
        let lines = self.text_editor.lines(1000);
        for (i, line) in lines.iter().enumerate() {
            // Create a row for each line
            let mut line_row = row![];

            // If line is empty, we still need to show cursor and minimum height
            if line.content.is_empty() {
                if i == vpos && self.cursor_visible {
                    line_row = line_row.push(
                        text("|")
                            .font(self.text_editor.default_font())
                            .size(self.text_editor.default_font_size()),
                    );
                } else {
                    line_row = line_row.push(
                        text(" ")
                            .font(self.text_editor.default_font())
                            .size(self.text_editor.default_font_size()),
                    );
                }
            } else {
                // Process each character in the line
                for (j, character) in line.content.iter().enumerate() {
                    // Get font and size
                    let font = if j < line.fonts.len() {
                        line.fonts[j]
                    } else {
                        self.text_editor.default_font()
                    };

                    let font_size = if j < line.font_sizes.len() {
                        line.font_sizes[j]
                    } else {
                        self.text_editor.default_font_size()
                    };

                    // Add character
                    line_row =
                        line_row.push(text(character.to_string()).font(font).size(font_size));

                    // Add cursor if this is the cursor position
                    if i == vpos && j + 1 == hpos && self.cursor_visible {
                        // TODO: Turn into overlay
                        line_row = line_row.push(text("|").font(font).size(font_size));
                    }
                }
            }

            line_column = line_column.push(line_row);
        }

        // Add status bar
        let word_count = self.text_editor.word_count();
        let char_count = self.text_editor.char_count();
        let counts = text(format!("Words: {}, Characters: {}", word_count, char_count));

        let positions = {
            let line_number = vpos + 1;
            let column_number = hpos + 1;
            let total_lines = self.text_editor.line_count();
            text(format!(
                "Line: {}/{}, Column: {}",
                line_number, total_lines, column_number
            ))
        };

        let status_bar = row![counts, iced::widget::horizontal_space(), positions];

        let script = scrollable(line_column.padding(10))
            .id(SCROLLABLE_ID.clone())
            .on_scroll(Message::Scrolled)
            .height(Length::Fill)
            .width(Length::Fill);

        let content = column![script, status_bar].spacing(10);
        content.into()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::EventOccurred(event) => {
                if let Some(editor_msg) = self.text_editor.handle_event(&event) {
                    return Task::perform(async { editor_msg }, Message::EditorEvent);
                }
                Task::none()
            }
            Message::EditorEvent(editor_msg) => {
                #[allow(clippy::single_match)]
                match editor_msg {
                    TextEditorMessage::CursorChanged(_, _, sd) => {
                        // Reset cursor visibility on cursor movement
                        self.cursor_visible = true;

                        // Handle changes in vertical cursor movement
                        return self.handle_vcursor_change(sd);
                    }
                    _ => {}
                }

                Task::none()
            }
            Message::CursorBlink => {
                // Toggle cursor visibility
                self.cursor_visible = !self.cursor_visible;
                Task::none()
            }
            // Message::ScrollToBeginning => {
            //     self.current_scroll_offset = scrollable::RelativeOffset::START;
            //     scrollable::snap_to(SCROLLABLE_ID.clone(), self.current_scroll_offset)
            // }
            // Message::ScrollToEnd => {
            //     self.current_scroll_offset = scrollable::RelativeOffset::END;
            //     scrollable::snap_to(SCROLLABLE_ID.clone(), self.current_scroll_offset)
            // }
            Message::Scrolled(viewport) => {
                self.current_scroll_offset = viewport.relative_offset();
                self.content_scroll_bound = viewport.content_bounds().height;
                Task::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            iced::event::listen().map(Message::EventOccurred),
            iced::time::every(Duration::from_millis(500)).map(|_| Message::CursorBlink),
        ])
    }

    fn handle_vcursor_change(&mut self, sd: i32) -> Task<Message> {
        // Handle vertical scrolling based on cursor position
        if sd != 0 {
            // Get the current cursor position
            let (_, vpos) = self.text_editor.cursor_position();

            // Calculate the viewport height in terms of line count
            // This is an approximation - 16.0 is the font size, add some padding
            // TODO: Make line height dynamic and adjusted for the line that was jumped from
            // or the default if none was set, use a helper function named line_height(n: i32) -> f32 for this
            // this function can be placed in the impl for the TextEditorWidget struct
            let line_height = 16.0 + 4.0; // font size + some padding
            let viewport_height = self.content_scroll_bound;
            let visible_lines = (viewport_height / line_height).floor() as usize;

            // Calculate the scroll positions
            let current_offset = self.current_scroll_offset.y;
            let total_lines = self.text_editor.line_count() as f32;

            // Calculate which lines are currently visible
            let start_line = (current_offset * total_lines) as usize;
            let end_line = start_line + visible_lines.min(self.text_editor.line_count());

            // Determine if scrolling is needed
            if sd > 0 && vpos >= end_line.saturating_sub(2) {
                // Cursor moved down and is near bottom of viewport
                // Calculate new offset to keep cursor visible with some context
                let new_line_pos = (vpos + 2).min(self.text_editor.line_count());
                let new_offset = new_line_pos as f32 / total_lines;
                self.current_scroll_offset = scrollable::RelativeOffset {
                    x: 0.0,
                    y: new_offset.min(1.0),
                };

                return scrollable::snap_to(SCROLLABLE_ID.clone(), self.current_scroll_offset);
            } else if sd < 0 && vpos <= start_line + 2 {
                // Cursor moved up and is near top of viewport
                // Calculate new offset to keep cursor visible with some context
                let new_line_pos = vpos.saturating_sub(2);
                let new_offset = new_line_pos as f32 / total_lines;
                self.current_scroll_offset = scrollable::RelativeOffset {
                    x: 0.0,
                    y: new_offset,
                };

                return scrollable::snap_to(SCROLLABLE_ID.clone(), self.current_scroll_offset);
            }
        }

        Task::none()
    }
}

fn main() -> iced::Result {
    iced::application("Blackscript", Blackscript::update, Blackscript::view)
        .subscription(Blackscript::subscription)
        .run()
}
