use iced::keyboard::key::Named;
use iced::{Event, Font, keyboard};

// Message type for text editor interactions
#[derive(Debug, Clone)]
pub enum TextEditorMessage {
    CharacterInput(char),
    Backspace,
    Delete,
    CursorLeft,
    CursorRight,
    CursorChanged(usize, usize, i32), // (hpos, vpos, scrolldir)
}

#[derive(Debug, Default, Clone)] // Added Clone for convenience
pub struct Line {
    pub content: Vec<char>,
    pub fonts: Vec<Font>,
    pub font_sizes: Vec<f32>,
}

impl Line {
    pub fn new() -> Self {
        Self {
            content: Vec::new(),
            fonts: Vec::new(),
            font_sizes: Vec::new(),
        }
    }

    // Ensure fonts and font_sizes are properly sized
    pub fn ensure_styles_match(&mut self) {
        // Resize fonts array using match statement
        match self.fonts.len().cmp(&self.content.len()) {
            std::cmp::Ordering::Less => {
                self.fonts.resize(self.content.len(), Font::default());
            }
            std::cmp::Ordering::Greater => {
                self.fonts.truncate(self.content.len());
            }
            std::cmp::Ordering::Equal => {} // No action needed
        }

        // Resize font_sizes array using match statement
        match self.font_sizes.len().cmp(&self.content.len()) {
            std::cmp::Ordering::Less => {
                self.font_sizes.resize(self.content.len(), 12.0);
            }
            std::cmp::Ordering::Greater => {
                self.font_sizes.truncate(self.content.len());
            }
            std::cmp::Ordering::Equal => {} // No action needed
        }
    }
}

// Implementation of the widget
#[allow(dead_code)]
pub struct TextEditorWidget {
    lines: Vec<Line>,
    cursor_hpos: usize,
    cursor_vpos: usize,
    cursor_visible: bool,
    default_font: Font,
    default_font_size: f32,
}

impl TextEditorWidget {
    pub fn new() -> Self {
        Self {
            lines: vec![Line::new()], // Start with one empty line
            cursor_hpos: 0,
            cursor_vpos: 0,
            cursor_visible: true,
            default_font: Font::with_name("Courier New"),
            default_font_size: 16.0,
        }
    }

    pub fn with_font(mut self, font: Font) -> Self {
        self.default_font = font;
        self
    }

    pub fn default_font(&self) -> Font {
        self.default_font
    }

    pub fn with_font_size(mut self, size: f32) -> Self {
        self.default_font_size = size;
        self
    }

    pub fn default_font_size(&self) -> f32 {
        self.default_font_size
    }

    pub fn handle_event(&mut self, event: &Event) -> Option<TextEditorMessage> {
        if let Event::Keyboard(keyboard_event) = event {
            match keyboard_event {
                keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(Named::Enter),
                    ..
                } => {
                    let sd = self.handle_enter();
                    return Some(TextEditorMessage::CursorChanged(
                        self.cursor_hpos,
                        self.cursor_vpos,
                        sd,
                    ));
                }
                keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(Named::Backspace),
                    modifiers,
                    ..
                } => {
                    self.handle_backspace(*modifiers);
                    return Some(TextEditorMessage::CursorChanged(
                        self.cursor_hpos,
                        self.cursor_vpos,
                        0,
                    ));
                }
                keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(Named::Delete),
                    ..
                } => {
                    self.handle_delete();
                    return Some(TextEditorMessage::CursorChanged(
                        self.cursor_hpos,
                        self.cursor_vpos,
                        0,
                    ));
                }
                keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(Named::ArrowLeft),
                    ..
                } => {
                    self.handle_arrow_left();
                    return Some(TextEditorMessage::CursorChanged(
                        self.cursor_hpos,
                        self.cursor_vpos,
                        0,
                    ));
                }
                keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(Named::ArrowRight),
                    ..
                } => {
                    self.handle_arrow_right();
                    return Some(TextEditorMessage::CursorChanged(
                        self.cursor_hpos,
                        self.cursor_vpos,
                        0,
                    ));
                }
                keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(Named::ArrowUp),
                    ..
                } => {
                    self.handle_arrow_up();
                    return Some(TextEditorMessage::CursorChanged(
                        self.cursor_hpos,
                        self.cursor_vpos,
                        0,
                    ));
                }
                keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(Named::ArrowDown),
                    ..
                } => {
                    self.handle_arrow_down();
                    return Some(TextEditorMessage::CursorChanged(
                        self.cursor_hpos,
                        self.cursor_vpos,
                        0,
                    ));
                }
                keyboard::Event::KeyPressed {
                    text: Some(text), ..
                } => {
                    // This addresses the clippy suggestion by removing the nested if let
                    self.handle_text_input(text);
                    return Some(TextEditorMessage::CursorChanged(
                        self.cursor_hpos,
                        self.cursor_vpos,
                        0,
                    ));
                }
                _ => {}
            }
        }

        None
    }

    // Helper methods to break down handle_event functionality
    fn handle_enter(&mut self) -> i32 {
        let mut scroll_direction = 0;

        if self.cursor_vpos + 1 == self.lines.len() {
            scroll_direction = 1;
        }

        // Ensure we have enough lines
        if self.cursor_vpos >= self.lines.len() {
            self.lines.resize_with(self.cursor_vpos + 1, Line::new);
        }

        // Get current line's content
        let current_line = &self.lines[self.cursor_vpos];

        // Create a new line
        let mut new_line = Line::new();

        // If cursor is not at the end of the line, move remaining content to new line
        if self.cursor_hpos < current_line.content.len() {
            let content_to_move = current_line.content[self.cursor_hpos..].to_vec();
            let fonts_to_move = if self.cursor_hpos < current_line.fonts.len() {
                current_line.fonts[self.cursor_hpos..].to_vec()
            } else {
                vec![self.default_font; content_to_move.len()]
            };

            let font_sizes_to_move = if self.cursor_hpos < current_line.font_sizes.len() {
                current_line.font_sizes[self.cursor_hpos..].to_vec()
            } else {
                vec![self.default_font_size; content_to_move.len()]
            };

            new_line.content = content_to_move;
            new_line.fonts = fonts_to_move;
            new_line.font_sizes = font_sizes_to_move;

            // Truncate current line
            self.lines[self.cursor_vpos]
                .content
                .truncate(self.cursor_hpos);
            self.lines[self.cursor_vpos]
                .fonts
                .truncate(self.cursor_hpos);
            self.lines[self.cursor_vpos]
                .font_sizes
                .truncate(self.cursor_hpos);
        }

        // Insert the new line after the current one
        self.lines.insert(self.cursor_vpos + 1, new_line);

        // Move cursor to beginning of new line
        self.cursor_vpos += 1;
        self.cursor_hpos = 0;

        scroll_direction
    }

    fn handle_backspace(&mut self, modifiers: keyboard::Modifiers) {
        // Handle backspace
        if self.cursor_hpos > 0 && self.cursor_vpos < self.lines.len() {
            if modifiers.control() {
                self.handle_ctrl_backspace();
            } else {
                // Regular backspace - remove single character
                self.lines[self.cursor_vpos]
                    .content
                    .remove(self.cursor_hpos - 1);
                if self.cursor_hpos - 1 < self.lines[self.cursor_vpos].fonts.len() {
                    self.lines[self.cursor_vpos]
                        .fonts
                        .remove(self.cursor_hpos - 1);
                }
                if self.cursor_hpos - 1 < self.lines[self.cursor_vpos].font_sizes.len() {
                    self.lines[self.cursor_vpos]
                        .font_sizes
                        .remove(self.cursor_hpos - 1);
                }
                self.cursor_hpos -= 1;
            }
        } else if self.cursor_hpos == 0 && self.cursor_vpos > 0 {
            self.join_with_previous_line();
        }

        // Ensure styles match after changes
        if self.cursor_vpos < self.lines.len() {
            self.lines[self.cursor_vpos].ensure_styles_match();
        }
    }

    fn handle_ctrl_backspace(&mut self) {
        // Word deletion (improved to handle trailing spaces)
        let content = &self.lines[self.cursor_vpos].content[..self.cursor_hpos];

        // First, skip any whitespace immediately before the cursor
        let mut end_pos = self.cursor_hpos;
        let mut chars_iter = content.iter().rev();

        // Skip trailing whitespace
        while end_pos > 0 && chars_iter.next().is_some_and(|c| c.is_whitespace()) {
            end_pos -= 1;
        }

        if end_pos < self.cursor_hpos {
            // If we skipped some whitespace, find the word boundary
            if let Some(pos) = content[..end_pos].iter().rposition(|c| c.is_whitespace()) {
                // Delete from after the last whitespace up to the cursor
                self.lines[self.cursor_vpos]
                    .content
                    .drain(pos + 1..self.cursor_hpos);
                self.lines[self.cursor_vpos]
                    .fonts
                    .drain(pos + 1..self.cursor_hpos);
                self.lines[self.cursor_vpos]
                    .font_sizes
                    .drain(pos + 1..self.cursor_hpos);

                self.cursor_hpos = pos + 1;
            } else {
                // No whitespace found, delete from start of line
                self.lines[self.cursor_vpos]
                    .content
                    .drain(..self.cursor_hpos);
                self.lines[self.cursor_vpos].fonts.drain(..self.cursor_hpos);
                self.lines[self.cursor_vpos]
                    .font_sizes
                    .drain(..self.cursor_hpos);
                self.cursor_hpos = 0;
            }
        } else {
            // If there were no trailing spaces, use original logic
            if let Some(pos) = content.iter().rposition(|c| c.is_whitespace()) {
                self.lines[self.cursor_vpos]
                    .content
                    .drain(pos + 1..self.cursor_hpos);
                self.lines[self.cursor_vpos]
                    .fonts
                    .drain(pos + 1..self.cursor_hpos);
                self.lines[self.cursor_vpos]
                    .font_sizes
                    .drain(pos + 1..self.cursor_hpos);

                self.cursor_hpos = pos + 1;
            } else {
                // Delete to start of line
                self.lines[self.cursor_vpos]
                    .content
                    .drain(..self.cursor_hpos);
                self.lines[self.cursor_vpos].fonts.drain(..self.cursor_hpos);
                self.lines[self.cursor_vpos]
                    .font_sizes
                    .drain(..self.cursor_hpos);
                self.cursor_hpos = 0;
            }
        }
    }

    fn join_with_previous_line(&mut self) {
        // Join with previous line on backspace at beginning of line
        let current_line = self.lines.remove(self.cursor_vpos);
        let prev_line_idx = self.cursor_vpos - 1;

        // Store the position to place cursor
        let new_cursor_pos = self.lines[prev_line_idx].content.len();

        // Append current line to previous line
        self.lines[prev_line_idx]
            .content
            .extend(current_line.content);
        self.lines[prev_line_idx].fonts.extend(current_line.fonts);
        self.lines[prev_line_idx]
            .font_sizes
            .extend(current_line.font_sizes);

        // Update cursor position
        self.cursor_vpos = prev_line_idx;
        self.cursor_hpos = new_cursor_pos;
    }

    fn handle_delete(&mut self) {
        // Delete key handling
        if self.cursor_vpos < self.lines.len() {
            if self.cursor_hpos < self.lines[self.cursor_vpos].content.len() {
                // Delete character at cursor
                self.lines[self.cursor_vpos]
                    .content
                    .remove(self.cursor_hpos);

                // Remove corresponding font and font size
                if self.cursor_hpos < self.lines[self.cursor_vpos].fonts.len() {
                    self.lines[self.cursor_vpos].fonts.remove(self.cursor_hpos);
                }
                if self.cursor_hpos < self.lines[self.cursor_vpos].font_sizes.len() {
                    self.lines[self.cursor_vpos]
                        .font_sizes
                        .remove(self.cursor_hpos);
                }

                // Ensure styles match after changes
                self.lines[self.cursor_vpos].ensure_styles_match();
            } else if self.cursor_vpos < self.lines.len() - 1 {
                self.join_with_next_line();
            }
        }
    }

    fn join_with_next_line(&mut self) {
        // At end of line but not last line - join with next line
        let next_line = self.lines.remove(self.cursor_vpos + 1);

        // Append next line to current line
        self.lines[self.cursor_vpos]
            .content
            .extend(next_line.content);
        self.lines[self.cursor_vpos].fonts.extend(next_line.fonts);
        self.lines[self.cursor_vpos]
            .font_sizes
            .extend(next_line.font_sizes);

        // Ensure styles match
        self.lines[self.cursor_vpos].ensure_styles_match();
    }

    fn handle_arrow_left(&mut self) {
        if self.cursor_hpos > 0 {
            self.cursor_hpos -= 1;
        } else if self.cursor_vpos > 0 {
            // Move to end of previous line
            self.cursor_vpos -= 1;
            self.cursor_hpos = self.lines[self.cursor_vpos].content.len();
        }
    }

    fn handle_arrow_right(&mut self) {
        if self.cursor_vpos < self.lines.len()
            && self.cursor_hpos < self.lines[self.cursor_vpos].content.len()
        {
            self.cursor_hpos += 1;
        } else if self.cursor_vpos < self.lines.len() - 1 {
            // Move to start of next line
            self.cursor_vpos += 1;
            self.cursor_hpos = 0;
        }
    }

    fn handle_arrow_up(&mut self) {
        if self.cursor_vpos > 0 {
            self.cursor_vpos -= 1;
            // Adjust horizontal position if needed
            if self.cursor_hpos > self.lines[self.cursor_vpos].content.len() {
                self.cursor_hpos = self.lines[self.cursor_vpos].content.len();
            }
        }
    }

    fn handle_arrow_down(&mut self) {
        if self.cursor_vpos + 1 < self.lines.len() {
            self.cursor_vpos += 1;
            // Adjust horizontal position if needed
            if self.cursor_hpos > self.lines[self.cursor_vpos].content.len() {
                self.cursor_hpos = self.lines[self.cursor_vpos].content.len();
            }
        }
    }

    fn handle_text_input(&mut self, text: &str) {
        // Ensure we have at least one line
        if self.lines.is_empty() {
            self.lines.push(Line::new());
        }

        // Ensure we have enough lines
        if self.cursor_vpos >= self.lines.len() {
            self.lines.resize_with(self.cursor_vpos + 1, Line::new);
        }

        // Get the characters to insert
        let chars: Vec<char> = text.chars().collect();

        // Insert each character at cursor position
        for (i, ch) in chars.iter().enumerate() {
            self.lines[self.cursor_vpos]
                .content
                .insert(self.cursor_hpos + i, *ch);
            self.lines[self.cursor_vpos]
                .fonts
                .insert(self.cursor_hpos + i, self.default_font);
            self.lines[self.cursor_vpos]
                .font_sizes
                .insert(self.cursor_hpos + i, self.default_font_size);
        }

        // Update cursor position
        self.cursor_hpos += chars.len();

        // Ensure styles match after insertion
        self.lines[self.cursor_vpos].ensure_styles_match();
    }

    // Returns a slice of the lines
    pub fn lines(&self, n: usize) -> &[Line] {
        let end = std::cmp::min(n, self.lines.len());
        &self.lines[0..end]
    }

    pub fn cursor_position(&self) -> (usize, usize) {
        (self.cursor_hpos, self.cursor_vpos)
    }

    // Toggle cursor visibility (for blinking)
    #[allow(dead_code)]
    pub fn toggle_cursor_visibility(&mut self) {
        self.cursor_visible = !self.cursor_visible;
    }

    #[allow(dead_code)]
    pub fn is_cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    // Get text content as a string
    pub fn text(&self) -> String {
        let mut result = String::new();
        for (i, line) in self.lines.iter().enumerate() {
            if i > 0 {
                result.push('\n');
            }
            result.extend(line.content.iter());
        }
        result
    }

    // Count words
    pub fn word_count(&self) -> usize {
        self.text().split_whitespace().count()
    }

    // Count characters
    pub fn char_count(&self) -> usize {
        self.text().chars().count()
    }

    // Get line count
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
}

impl Default for TextEditorWidget {
    fn default() -> Self {
        Self::new()
    }
}
