// canvas_textarea.rs
use iced::{
    Color, Event, Font, Point, Rectangle, Size,
    advanced::graphics::geometry::{self, Frame},
    alignment::{Horizontal, Vertical},
    keyboard::{Event as KeyEvent, Modifiers, key::Named},
    mouse::{Cursor, Event as MouseEvent},
    widget::{
        canvas::{self, Geometry, Path, Stroke},
        text::{LineHeight, Shaping},
    },
};
use std::cell::RefCell;
use std::cmp::Ordering;

// === Text Editor Message Types ===

#[derive(Debug, Clone)]
pub enum TextEditorMessage {
    CharacterInput(char),
    Backspace,
    Delete,
    CursorChanged(usize, usize, i32), // (hpos, vpos, scrolldir)
    ContentChanged(usize, usize, i32),
}

// === Line Struct (Text Storage & Styling) ===

#[derive(Debug, Default, Clone)]
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

    // Ensure fonts and font_sizes are properly sized.
    pub fn ensure_styles_match(&mut self) {
        let content_len = self.content.len();
        match self.fonts.len().cmp(&content_len) {
            Ordering::Less => self.fonts.resize(content_len, Font::default()),
            Ordering::Greater => self.fonts.truncate(content_len),
            Ordering::Equal => {}
        }
        match self.font_sizes.len().cmp(&content_len) {
            Ordering::Less => self.font_sizes.resize(content_len, 12.0),
            Ordering::Greater => self.font_sizes.truncate(content_len),
            Ordering::Equal => {}
        }
    }

    // Insert a character at a specific position with a given style.
    pub fn insert_char(&mut self, pos: usize, c: char, font: Font, font_size: f32) {
        self.content.insert(pos, c);
        self.fonts.insert(pos, font);
        self.font_sizes.insert(pos, font_size);
    }

    // Remove a character at a specific position.
    pub fn remove_char(&mut self, pos: usize) -> Option<char> {
        if pos < self.content.len() {
            let c = self.content.remove(pos);
            if pos < self.fonts.len() {
                self.fonts.remove(pos);
            }
            if pos < self.font_sizes.len() {
                self.font_sizes.remove(pos);
            }
            Some(c)
        } else {
            None
        }
    }

    // Drain characters in a range.
    pub fn drain_chars(&mut self, range: std::ops::Range<usize>) -> Vec<char> {
        let chars: Vec<char> = self.content.drain(range.clone()).collect();
        if !range.is_empty() && range.start < self.fonts.len() {
            let end = range.end.min(self.fonts.len());
            self.fonts.drain(range.start..end);
        }
        if !range.is_empty() && range.start < self.font_sizes.len() {
            let end = range.end.min(self.font_sizes.len());
            self.font_sizes.drain(range.start..end);
        }
        chars
    }

    // Append another line to this one.
    pub fn append(&mut self, other: &Line) {
        self.content.extend_from_slice(&other.content);
        self.fonts.extend_from_slice(&other.fonts);
        self.font_sizes.extend_from_slice(&other.font_sizes);
    }

    #[allow(dead_code)]
    pub fn font(&self, n: usize) -> Option<Font> {
        // check if exists otherwise return default
        if n < self.fonts.len() {
            Some(self.fonts[n])
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn font_size(&self, n: usize) -> Option<f32> {
        // check if exists otherwise return default
        if n < self.font_sizes.len() {
            Some(self.font_sizes[n])
        } else {
            None
        }
    }
}

// === Text Editor State with Interior Mutability ===

// We wrap all mutable fields in an inner state which is stored in a RefCell.
#[derive(Debug, Clone)]
pub struct TextEditorState {
    inner: RefCell<TextEditorStateInner>,
}

#[derive(Debug, Clone)]
struct TextEditorStateInner {
    lines: Vec<Line>,
    cursor_hpos: usize,
    cursor_vpos: usize,
    cursor_visible: bool,
    default_font: Font,
    default_font_size: f32,
    char_width: f32,
    line_height: f32,
    scroll_offset_y: f32,
    viewport_height: f32,
    viewport_width: f32,
    last_click_position: Option<Point>,

    // TODO: Maybe we could render those values only when needed, so that
    // the count functions only get called when text changes, instead of when something
    // rerenders like the blinking cursor. Cursor blinking now updates this inner state
    // causing everything to rerender, that shouldnt really by the case. So perhaps
    // we could seperate the cursor blinking from the text editor state. cursor_visible to
    // be precise.
    cached_word_count: usize,
    cached_char_count: usize,
    max_chars_per_visual_line: usize,
}

impl Default for TextEditorState {
    fn default() -> Self {
        Self {
            inner: RefCell::new(TextEditorStateInner {
                lines: vec![Line::new()],
                cursor_hpos: 0,
                cursor_vpos: 0,
                cursor_visible: true,
                default_font: Font::with_name("Courier New"),
                default_font_size: 16.0,
                char_width: 9.6,
                line_height: 20.0,
                scroll_offset_y: 0.0,
                viewport_height: 0.0,
                viewport_width: 0.0,
                last_click_position: None,
                cached_word_count: 0,
                cached_char_count: 0,
                max_chars_per_visual_line: 120,
            }),
        }
    }
}

// === Implementing the Canvas Program Trait ===
//
// Note: The trait’s update and draw methods now take &self. We use interior mutability
// (via RefCell) to allow modifying our inner state.

impl canvas::Program<TextEditorMessage> for TextEditorState {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let inner = self.inner.borrow();
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::TRANSPARENT);

        let max_chars = inner.max_chars_per_visual_line;
        let line_height = inner.line_height;
        let char_width = inner.char_width;

        let mut current_visual_line = 0;

        for (logical_idx, line) in inner.lines.iter().enumerate() {
            let mut pos = 0;

            while pos < line.content.len() {
                let wrap_pos = inner.find_wrap_position(line, pos, max_chars);
                let line_y = current_visual_line as f32 * line_height - inner.scroll_offset_y;

                if line_y + line_height >= 0.0 && line_y <= bounds.height {
                    let text = line.content[pos..wrap_pos].iter().collect::<String>();

                    frame.fill_text(canvas::Text {
                        content: text,
                        position: Point::new(10.0, line_y + line_height),
                        color: Color::WHITE,
                        size: iced::Pixels(inner.default_font_size),
                        line_height: LineHeight::Relative(1.0),
                        font: inner.default_font,
                        horizontal_alignment: Horizontal::Left,
                        vertical_alignment: Vertical::Top,
                        shaping: Shaping::Basic,
                    });

                    if inner.cursor_visible && logical_idx == inner.cursor_vpos {
                        let (cursor_visual_line, cursor_visual_column) =
                            inner.logical_to_visual_position(logical_idx, inner.cursor_hpos);

                        if cursor_visual_line
                            == current_visual_line - inner.get_visual_line_offset(logical_idx)
                        {
                            let cursor_x = 10.0 + cursor_visual_column as f32 * char_width;

                            let cursor_path = Path::line(
                                Point::new(cursor_x, line_y + line_height),
                                Point::new(
                                    cursor_x,
                                    line_y + line_height + inner.default_font_size,
                                ),
                            );
                            frame.stroke(
                                &cursor_path,
                                Stroke {
                                    width: 1.0,
                                    style: geometry::Style::Solid(Color::WHITE),
                                    ..Stroke::default()
                                },
                            );
                        }
                    }
                }

                pos = wrap_pos;
                current_visual_line += 1;
            }

            // Handle empty lines to ensure cursor visibility and line height
            if line.content.is_empty() {
                let line_y = current_visual_line as f32 * line_height - inner.scroll_offset_y;

                if line_y + line_height >= 0.0 && line_y <= bounds.height {
                    // Draw empty line placeholder to maintain line height
                    frame.fill_text(canvas::Text {
                        content: " ".to_string(),
                        position: Point::new(10.0, line_y + line_height),
                        color: Color::WHITE,
                        size: iced::Pixels(inner.default_font_size),
                        line_height: LineHeight::Relative(1.0),
                        font: inner.default_font,
                        horizontal_alignment: Horizontal::Left,
                        vertical_alignment: Vertical::Top,
                        shaping: Shaping::Basic,
                    });

                    // Draw cursor if on this empty line
                    if inner.cursor_visible && logical_idx == inner.cursor_vpos {
                        let cursor_x = 10.0;
                        let cursor_path = Path::line(
                            Point::new(cursor_x, line_y + line_height),
                            Point::new(cursor_x, line_y + line_height + inner.default_font_size),
                        );
                        frame.stroke(
                            &cursor_path,
                            Stroke {
                                width: 1.0,
                                style: geometry::Style::Solid(Color::WHITE),
                                ..Stroke::default()
                            },
                        );
                    }
                }

                current_visual_line += 1;
            }
        }

        vec![frame.into_geometry()]
    }

    fn update(
        &self,
        _state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> (canvas::event::Status, Option<TextEditorMessage>) {
        let mut inner = self.inner.borrow_mut();
        match event {
            canvas::Event::Mouse(mouse_event) => inner.handle_mouse_event(mouse_event, bounds),
            canvas::Event::Keyboard(keyboard_event) => inner.handle_keyboard_event(keyboard_event),
            _ => (canvas::event::Status::Ignored, None),
        }
    }
}

// === Methods for the Inner State ===

impl TextEditorStateInner {
    fn handle_mouse_event(
        &mut self,
        event: MouseEvent,
        bounds: Rectangle,
    ) -> (canvas::event::Status, Option<TextEditorMessage>) {
        match event {
            MouseEvent::ButtonPressed(iced::mouse::Button::Left) => {
                if let Some(position) = self.last_click_position {
                    let click_y = position.y + self.scroll_offset_y;
                    let mut visual_line = (click_y / self.line_height) as usize;

                    let mut logical_vpos = 0;
                    for (idx, line) in self.lines.iter().enumerate() {
                        let num_visual =
                            line.content.len().div_ceil(self.max_chars_per_visual_line);

                        if visual_line < num_visual {
                            logical_vpos = idx;
                            break;
                        }
                        visual_line -= num_visual;
                    }

                    let line = &self.lines[logical_vpos];
                    let hpos = (visual_line * self.max_chars_per_visual_line)
                        + ((position.x - 10.0) / self.char_width).floor().max(0.0) as usize;
                    self.cursor_vpos = logical_vpos;
                    self.cursor_hpos = hpos.min(line.content.len());
                    self.cursor_visible = true;
                    return (canvas::event::Status::Captured, None);
                }
                (canvas::event::Status::Captured, None)
            }
            MouseEvent::CursorMoved { position } => {
                if bounds.contains(position) {
                    self.last_click_position = Some(position);
                } else {
                    self.last_click_position = None;
                }
                (canvas::event::Status::Captured, None)
            }
            MouseEvent::WheelScrolled { delta } => {
                match delta {
                    iced::mouse::ScrollDelta::Lines { y, .. } => {
                        self.scroll_offset_y -= y * self.line_height;
                    }
                    iced::mouse::ScrollDelta::Pixels { y, .. } => {
                        self.scroll_offset_y -= y;
                    }
                }
                self.scroll_offset_y = self.scroll_offset_y.max(0.0);
                (canvas::event::Status::Captured, None)
            }
            _ => (canvas::event::Status::Ignored, None),
        }
    }

    fn handle_keyboard_event(
        &mut self,
        event: KeyEvent,
    ) -> (canvas::event::Status, Option<TextEditorMessage>) {
        match event {
            KeyEvent::KeyPressed {
                key: iced::keyboard::Key::Named(Named::Enter),
                ..
            } => {
                let sd = self.handle_enter();
                (
                    canvas::event::Status::Captured,
                    Some(TextEditorMessage::ContentChanged(
                        self.cursor_hpos,
                        self.cursor_vpos,
                        sd,
                    )),
                )
            }
            KeyEvent::KeyPressed {
                key: iced::keyboard::Key::Named(Named::Backspace),
                modifiers,
                ..
            } => {
                self.handle_backspace(modifiers);
                (
                    canvas::event::Status::Captured,
                    Some(TextEditorMessage::ContentChanged(
                        self.cursor_hpos,
                        self.cursor_vpos,
                        0,
                    )),
                )
            }
            KeyEvent::KeyPressed {
                key: iced::keyboard::Key::Named(Named::Delete),
                modifiers,
                ..
            } => {
                self.handle_delete(modifiers);
                (
                    canvas::event::Status::Captured,
                    Some(TextEditorMessage::ContentChanged(
                        self.cursor_hpos,
                        self.cursor_vpos,
                        0,
                    )),
                )
            }
            KeyEvent::KeyPressed {
                key: iced::keyboard::Key::Named(Named::ArrowLeft),
                ..
            } => {
                self.handle_arrow_left();
                (
                    canvas::event::Status::Captured,
                    Some(TextEditorMessage::CursorChanged(
                        self.cursor_hpos,
                        self.cursor_vpos,
                        0,
                    )),
                )
            }
            KeyEvent::KeyPressed {
                key: iced::keyboard::Key::Named(Named::ArrowRight),
                ..
            } => {
                self.handle_arrow_right();
                (
                    canvas::event::Status::Captured,
                    Some(TextEditorMessage::CursorChanged(
                        self.cursor_hpos,
                        self.cursor_vpos,
                        0,
                    )),
                )
            }
            KeyEvent::KeyPressed {
                key: iced::keyboard::Key::Named(Named::ArrowUp),
                ..
            } => {
                self.handle_arrow_up();
                (
                    canvas::event::Status::Captured,
                    Some(TextEditorMessage::CursorChanged(
                        self.cursor_hpos,
                        self.cursor_vpos,
                        -1,
                    )),
                )
            }
            KeyEvent::KeyPressed {
                key: iced::keyboard::Key::Named(Named::ArrowDown),
                ..
            } => {
                self.handle_arrow_down();
                (
                    canvas::event::Status::Captured,
                    Some(TextEditorMessage::CursorChanged(
                        self.cursor_hpos,
                        self.cursor_vpos,
                        1,
                    )),
                )
            }
            KeyEvent::KeyPressed {
                text: Some(text), ..
            } => {
                self.handle_text_input(text.as_str());
                self.update_cached_counts();
                (
                    canvas::event::Status::Captured,
                    Some(TextEditorMessage::ContentChanged(
                        self.cursor_hpos,
                        self.cursor_vpos,
                        0,
                    )),
                )
            }
            _ => (canvas::event::Status::Ignored, None),
        }
    }

    fn get_visual_line_offset(&self, logical_line_idx: usize) -> usize {
        let mut offset = 0;
        for idx in 0..logical_line_idx {
            offset += self.calculate_visual_lines(&self.lines[idx]);
        }
        offset
    }

    fn handle_enter(&mut self) -> i32 {
        let mut scroll_direction = 0;
        self.ensure_line_exists(self.cursor_vpos);
        if self.cursor_vpos + 1 == self.lines.len() {
            scroll_direction = 1;
        }
        let mut new_line = Line::new();
        if self.cursor_hpos < self.lines[self.cursor_vpos].content.len() {
            let content_range = self.cursor_hpos..self.lines[self.cursor_vpos].content.len();
            let content_to_move = self.lines[self.cursor_vpos].drain_chars(content_range);
            for c in content_to_move {
                new_line.insert_char(
                    new_line.content.len(),
                    c,
                    self.default_font,
                    self.default_font_size,
                );
            }
            self.lines[self.cursor_vpos].ensure_styles_match();
        }
        self.lines.insert(self.cursor_vpos + 1, new_line);
        self.cursor_vpos += 1;
        self.cursor_hpos = 0;
        self.ensure_cursor_visible();
        scroll_direction
    }

    fn handle_backspace(&mut self, modifiers: Modifiers) {
        self.ensure_line_exists(self.cursor_vpos);
        if self.cursor_hpos > 0 {
            if modifiers.control() {
                self.handle_ctrl_backspace();
            } else {
                self.lines[self.cursor_vpos].remove_char(self.cursor_hpos - 1);
                self.cursor_hpos -= 1;
            }
        } else if self.cursor_vpos > 0 {
            self.join_with_previous_line();
        }
        if self.cursor_vpos < self.lines.len() {
            self.lines[self.cursor_vpos].ensure_styles_match();
        }
        self.ensure_cursor_visible();
        self.update_cached_counts();
    }

    fn handle_ctrl_backspace(&mut self) {
        if self.cursor_hpos == 0 || self.cursor_vpos >= self.lines.len() {
            return;
        }
        let content = &self.lines[self.cursor_vpos].content[..self.cursor_hpos];
        let mut end_pos = self.cursor_hpos;
        while end_pos > 0 && content[end_pos - 1].is_whitespace() {
            end_pos -= 1;
        }
        let start_pos = if end_pos > 0 {
            content[..end_pos]
                .iter()
                .rposition(|c| c.is_whitespace())
                .map_or(0, |pos| pos + 1)
        } else {
            0
        };
        if start_pos < self.cursor_hpos {
            self.lines[self.cursor_vpos].drain_chars(start_pos..self.cursor_hpos);
            self.cursor_hpos = start_pos;
        }
        self.update_cached_counts();
    }

    fn handle_ctrl_delete(&mut self) {
        if self.cursor_vpos >= self.lines.len() {
            return;
        }

        let line = &mut self.lines[self.cursor_vpos];
        let content = &line.content;
        let mut end_pos = self.cursor_hpos;

        // Skip whitespace after cursor
        while end_pos < content.len() && content[end_pos].is_whitespace() {
            end_pos += 1;
        }

        // Find end of next word
        while end_pos < content.len() && !content[end_pos].is_whitespace() {
            end_pos += 1;
        }

        if end_pos > self.cursor_hpos {
            line.drain_chars(self.cursor_hpos..end_pos);
        }
    }

    fn join_with_previous_line(&mut self) {
        if self.cursor_vpos == 0 || self.cursor_vpos >= self.lines.len() {
            return;
        }
        let current_line = self.lines.remove(self.cursor_vpos);
        let prev_line_idx = self.cursor_vpos - 1;
        let new_cursor_pos = self.lines[prev_line_idx].content.len();
        self.lines[prev_line_idx].append(&current_line);
        self.cursor_vpos = prev_line_idx;
        self.cursor_hpos = new_cursor_pos;
    }

    fn handle_delete(&mut self, modifiers: Modifiers) {
        self.ensure_line_exists(self.cursor_vpos);

        if modifiers.control() {
            self.handle_ctrl_delete();
        } else {
            #[allow(clippy::collapsible_if)]
            if self.cursor_hpos < self.lines[self.cursor_vpos].content.len() {
                self.lines[self.cursor_vpos].remove_char(self.cursor_hpos);
            } else if self.cursor_vpos < self.lines.len() - 1 {
                self.join_with_next_line();
            }
        }

        self.update_cached_counts();
    }

    fn join_with_next_line(&mut self) {
        if self.cursor_vpos >= self.lines.len() - 1 {
            return;
        }
        let next_line = self.lines.remove(self.cursor_vpos + 1);
        self.lines[self.cursor_vpos].append(&next_line);
        self.lines[self.cursor_vpos].ensure_styles_match();
    }

    fn handle_arrow_left(&mut self) {
        if self.cursor_hpos > 0 {
            self.cursor_hpos -= 1;
        } else if self.cursor_vpos > 0 {
            self.cursor_vpos -= 1;
            self.ensure_line_exists(self.cursor_vpos);
            self.cursor_hpos = self.lines[self.cursor_vpos].content.len();
        }
        self.ensure_cursor_visible();
    }

    fn handle_arrow_right(&mut self) {
        self.ensure_line_exists(self.cursor_vpos);
        if self.cursor_hpos < self.lines[self.cursor_vpos].content.len() {
            self.cursor_hpos += 1;
        } else if self.cursor_vpos < self.lines.len() - 1 {
            self.cursor_vpos += 1;
            self.cursor_hpos = 0;
        }
        self.ensure_cursor_visible();
    }

    fn handle_arrow_up(&mut self) {
        if self.cursor_vpos == 0 && self.cursor_hpos == 0 {
            return; // Already at the start
        }

        let (visual_line, visual_column) =
            self.logical_to_visual_position(self.cursor_vpos, self.cursor_hpos);

        if visual_line > 0 {
            // Move to previous visual line within the same logical line
            let mut pos = 0;
            let line = &self.lines[self.cursor_vpos];
            let mut current_visual = 0;

            while current_visual < visual_line - 1 && pos < line.content.len() {
                pos = self.find_wrap_position(line, pos, self.max_chars_per_visual_line);
                current_visual += 1;
            }

            // Try to maintain the same visual column
            self.cursor_hpos = (pos + visual_column)
                .min(self.find_wrap_position(line, pos, self.max_chars_per_visual_line) - 1);
        } else if self.cursor_vpos > 0 {
            // Move to the previous logical line
            self.cursor_vpos -= 1;

            // Find the last visual line in the previous logical line
            let prev_line = &self.lines[self.cursor_vpos];
            let prev_visual_lines = self.calculate_visual_lines(prev_line);

            if prev_visual_lines > 0 {
                let mut pos = 0;
                let mut current_visual = 0;

                // Move to the last visual line of the previous logical line
                while current_visual < prev_visual_lines - 1 && pos < prev_line.content.len() {
                    pos = self.find_wrap_position(prev_line, pos, self.max_chars_per_visual_line);
                    current_visual += 1;
                }

                // Try to maintain the same visual column
                self.cursor_hpos = (pos + visual_column).min(prev_line.content.len());
            } else {
                self.cursor_hpos = 0;
            }
        }

        self.ensure_cursor_visible();
    }

    fn handle_arrow_down(&mut self) {
        if self.cursor_vpos >= self.lines.len() - 1
            && self.cursor_hpos >= self.lines[self.cursor_vpos].content.len()
        {
            return; // Already at the end
        }

        let (visual_line, visual_column) =
            self.logical_to_visual_position(self.cursor_vpos, self.cursor_hpos);
        let current_line = &self.lines[self.cursor_vpos];
        let current_line_visual_lines = self.calculate_visual_lines(current_line);

        if visual_line < current_line_visual_lines - 1 {
            // Move to next visual line within same logical line
            let mut pos = 0;
            let mut current_visual = 0;

            while current_visual <= visual_line && pos < current_line.content.len() {
                pos = self.find_wrap_position(current_line, pos, self.max_chars_per_visual_line);
                current_visual += 1;
            }

            // Try to maintain same visual column
            self.cursor_hpos = (pos + visual_column)
                .min(self.find_wrap_position(current_line, pos, self.max_chars_per_visual_line) - 1)
                .min(current_line.content.len());
        } else if self.cursor_vpos < self.lines.len() - 1 {
            // Move to the next logical line
            self.cursor_vpos += 1;

            // Position cursor at the same visual column on the first visual line
            self.cursor_hpos = visual_column.min(self.lines[self.cursor_vpos].content.len());
        }

        self.ensure_cursor_visible();
    }

    fn handle_text_input(&mut self, text: &str) {
        self.ensure_line_exists(self.cursor_vpos);
        let chars: Vec<char> = text.chars().collect();
        for (i, ch) in chars.iter().enumerate() {
            self.lines[self.cursor_vpos].insert_char(
                self.cursor_hpos + i,
                *ch,
                self.default_font,
                self.default_font_size,
            );
        }
        self.cursor_hpos += chars.len();
        self.lines[self.cursor_vpos].ensure_styles_match();
        self.ensure_cursor_visible();
    }

    fn ensure_line_exists(&mut self, index: usize) {
        if self.lines.is_empty() {
            self.lines.push(Line::new());
        }
        if index >= self.lines.len() {
            self.lines.resize_with(index + 1, Line::new);
        }
    }

    fn update_max_chars(&mut self) {
        let padding = 20.0; // 10px on each side
        let available_width = (self.viewport_width - padding).max(0.0);
        self.max_chars_per_visual_line =
            ((available_width / self.char_width).floor() as usize).max(1);
    }

    fn calculate_visual_lines(&self, line: &Line) -> usize {
        let content_len = line.content.len();
        if content_len == 0 {
            return 1;
        }

        let max_chars = self.max_chars_per_visual_line;
        let mut visual_lines = 0;
        let mut pos = 0;

        while pos < content_len {
            let wrap_pos = self.find_wrap_position(line, pos, max_chars);
            pos = wrap_pos;
            visual_lines += 1;
        }

        visual_lines
    }

    fn visual_line_count(&self) -> usize {
        self.lines
            .iter()
            .map(|line| self.calculate_visual_lines(line))
            .sum()
    }

    fn logical_to_visual_position(&self, logical_line_idx: usize, hpos: usize) -> (usize, usize) {
        if logical_line_idx >= self.lines.len() {
            return (0, 0);
        }

        let line = &self.lines[logical_line_idx];

        if line.content.is_empty() {
            return (0, 0);
        }

        let max_chars = self.max_chars_per_visual_line;

        let mut visual_line = 0;
        let mut pos = 0;

        while pos < hpos {
            let wrap_pos = self.find_wrap_position(line, pos, max_chars);
            if wrap_pos >= hpos || wrap_pos <= pos {
                break;
            }
            pos = wrap_pos;
            visual_line += 1;
        }

        let visual_column = hpos - pos;
        (visual_line, visual_column)
    }

    fn ensure_cursor_visible(&mut self) {
        let total_visual_lines = self.visual_line_count();
        let cursor_visual_line = self
            .logical_to_visual_position(self.cursor_vpos, self.cursor_hpos)
            .0;
        let cursor_y = cursor_visual_line as f32 * self.line_height;

        if cursor_y < self.scroll_offset_y {
            self.scroll_offset_y = cursor_y;
        } else if cursor_y + self.line_height > self.scroll_offset_y + self.viewport_height {
            self.scroll_offset_y = cursor_y + self.line_height - self.viewport_height;
        }

        self.scroll_offset_y = self
            .scroll_offset_y
            .max(0.0)
            .min((total_visual_lines as f32 * self.line_height - self.viewport_height).max(0.0));
    }

    fn find_wrap_position(&self, line: &Line, start: usize, max_chars: usize) -> usize {
        let content = &line.content;
        let end = (start + max_chars).min(content.len());

        // If we can't fit at least one character or we fit the whole content, return as is
        if start >= end || end == content.len() {
            return end;
        }

        // Look for a space to break at
        for i in (start..end).rev() {
            if content[i].is_whitespace() {
                return i + 1; // Break after the whitespace
            }
        }

        // If no space was found, we have to break in the middle of a word
        end
    }

    // Get the current cursor position.
    fn cursor_position(&self) -> (usize, usize) {
        (self.cursor_hpos, self.cursor_vpos)
    }

    // Get text content as a string.
    fn text(&self) -> String {
        let mut result = String::with_capacity(self.estimate_text_capacity());
        for (i, line) in self.lines.iter().enumerate() {
            if i > 0 {
                result.push('\n');
            }
            result.extend(line.content.iter());
        }
        result
    }

    fn estimate_text_capacity(&self) -> usize {
        let mut capacity = 0;
        for (i, line) in self.lines.iter().enumerate() {
            if i > 0 {
                capacity += 1;
            }
            capacity += line.content.len();
        }
        capacity
    }

    fn update_cached_counts(&mut self) {
        let mut char_count = 0;
        for line in &self.lines {
            if !line.content.is_empty() && line.content[0] != '\n' && line.content[0] != ' ' {
                char_count += line.content.len();
            };
        }

        self.cached_char_count = char_count;
        self.cached_word_count = self.text().split_whitespace().count();
    }

    fn word_count(&self) -> usize {
        self.cached_word_count
    }

    fn char_count(&self) -> usize {
        self.cached_char_count
    }

    fn line_count(&self) -> usize {
        self.lines.len()
    }
}

// === Public Methods on TextEditorState ===

impl TextEditorState {
    pub fn set_viewport_size(&self, size: Size) {
        let mut inner = self.inner.borrow_mut();
        inner.viewport_width = size.width;
        inner.viewport_height = size.height;
        inner.update_max_chars();
    }

    pub fn toggle_cursor_visibility(&self) {
        self.inner.borrow_mut().cursor_visible ^= true;
    }

    pub fn cursor_position(&self) -> (usize, usize) {
        self.inner.borrow().cursor_position()
    }

    pub fn word_count(&self) -> usize {
        self.inner.borrow().word_count()
    }

    pub fn char_count(&self) -> usize {
        self.inner.borrow().char_count()
    }

    pub fn line_count(&self) -> usize {
        self.inner.borrow().line_count()
    }

    #[allow(dead_code)]
    pub fn line(&self, n: usize) -> Line {
        self.inner.borrow().lines[n].clone()
    }

    // Since returning a slice from a RefCell is tricky, we return a vector of lines.
    pub fn lines(&self, n: usize) -> Vec<Line> {
        let inner = self.inner.borrow();
        let end = n.min(inner.lines.len());
        inner.lines[0..end].to_vec()
    }
}

// === Text Editor Widget Wrapper Around Canvas ===

pub struct TextEditorWidget {
    state: TextEditorState,
}

impl TextEditorWidget {
    pub fn new() -> Self {
        Self {
            state: TextEditorState::default(),
        }
    }

    pub fn state(&self) -> &TextEditorState {
        &self.state
    }

    pub fn with_font(self, font: Font) -> Self {
        self.state.inner.borrow_mut().default_font = font;
        self
    }

    pub fn with_font_size(self, size: f32) -> Self {
        {
            let mut inner = self.state.inner.borrow_mut();
            inner.default_font_size = size;
            inner.line_height = size * 1.2;
            inner.char_width = size * 0.6;
        }
        self
    }

    #[allow(dead_code)]
    pub fn default_font(&self) -> Font {
        self.state.inner.borrow().default_font
    }

    #[allow(dead_code)]
    pub fn default_font_size(&self) -> f32 {
        self.state.inner.borrow().default_font_size
    }

    /// Process a keyboard event and update the internal state.
    pub fn process_keyboard_event(&mut self, event: KeyEvent) -> Option<TextEditorMessage> {
        // Forward the event to our inner state.
        self.state.inner.borrow_mut().handle_keyboard_event(event).1
    }

    pub fn handle_event(&mut self, event: &Event) -> Option<TextEditorMessage> {
        match event {
            Event::Window(iced::window::Event::Resized(size)) => {
                self.state
                    .set_viewport_size(Size::new(size.width, size.height));
                None
            }
            _ => None, // Other events are handled directly by the canvas.
        }
    }

    pub fn toggle_cursor_visibility(&mut self) {
        self.state.toggle_cursor_visibility();
    }

    // Forward methods to the internal state.
    pub fn cursor_position(&self) -> (usize, usize) {
        self.state.cursor_position()
    }

    pub fn word_count(&self) -> usize {
        self.state.word_count()
    }

    pub fn char_count(&self) -> usize {
        self.state.char_count()
    }

    pub fn line_count(&self) -> usize {
        self.state.line_count()
    }

    #[allow(dead_code)]
    pub fn lines(&self, n: usize) -> Vec<Line> {
        self.state.lines(n)
    }
}

impl Default for TextEditorWidget {
    fn default() -> Self {
        Self::new()
    }
}
