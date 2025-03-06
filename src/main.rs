// main.rs
use std::time::Duration;

use iced::{
    Element, Event, Font, Length, Subscription, Task,
    widget::{Canvas, row, text},
};

use widgets::textarea::{TextEditorMessage, TextEditorWidget};

mod widgets;

struct Blackscript {
    text_editor: TextEditorWidget,
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
}

impl Blackscript {
    pub fn new() -> Self {
        Self {
            text_editor: TextEditorWidget::new()
                .with_font(Font::with_name("Courier New"))
                .with_font_size(16.0),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let editor_view = Canvas::new(self.text_editor.state())
            .width(Length::Fill)
            .height(Length::Fill);

        let editor_element: Element<'_, Message> =
            Element::from(editor_view).map(Message::EditorEvent);

        let content =
            iced::widget::Column::with_children(vec![editor_element, self.render_status_bar()])
                .spacing(10);

        content.into()
    }

    fn render_status_bar(&self) -> Element<'_, Message> {
        let word_count = self.text_editor.word_count();
        let char_count = self.text_editor.char_count();
        let counts = text(format!("Words: {}, Characters: {}", word_count, char_count));

        let (hpos, vpos) = self.text_editor.cursor_position();
        let line_number = vpos + 1;
        let column_number = hpos + 1;
        let total_lines = self.text_editor.line_count();
        let positions = text(format!(
            "Line: {}/{}, Column: {}",
            line_number, total_lines, column_number
        ));

        row![counts, iced::widget::horizontal_space(), positions].into()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::EventOccurred(event) => {
                match event {
                    // Forward keyboard events explicitly.
                    Event::Keyboard(key_event) => {
                        if let Some(editor_msg) = self.text_editor.process_keyboard_event(key_event)
                        {
                            return Task::perform(async { editor_msg }, Message::EditorEvent);
                        }
                    }
                    // Handle other events (like resizing) as before.
                    _ => {
                        if let Some(editor_msg) = self.text_editor.handle_event(&event) {
                            return Task::perform(async { editor_msg }, Message::EditorEvent);
                        }
                    }
                }
                Task::none()
            }
            Message::EditorEvent(editor_msg) => {
                // For now, reset the cursor visibility on any cursor change.
                if let TextEditorMessage::ContentChanged(_, _, _) = editor_msg {
                    // This now toggles the internal state.
                    self.text_editor.toggle_cursor_visibility();
                } else if let TextEditorMessage::CursorChanged(_, _, _) = editor_msg {
                    // This now toggles the internal state.
                    self.text_editor.toggle_cursor_visibility();
                }
                Task::none()
            }
            Message::CursorBlink => {
                // Instead of toggling a separate field, toggle the canvas’ internal cursor.
                self.text_editor.toggle_cursor_visibility();
                Task::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        // Listen for application events and periodically toggle the cursor blink.
        Subscription::batch(vec![
            iced::event::listen().map(Message::EventOccurred),
            iced::time::every(Duration::from_millis(500)).map(|_| Message::CursorBlink),
        ])
    }
}

fn main() -> iced::Result {
    iced::application("Blackscript", Blackscript::update, Blackscript::view)
        .subscription(Blackscript::subscription)
        .run()
}
