use std::collections::VecDeque;

/// A message displayed in the chat
#[derive(Debug, Clone)]
pub enum ChatMessage {
    User(String),
    Assistant(String),
    Tool(String, String),     // (tool_name, args_preview)
    ToolResult(String, bool), // (content, is_error)
    System(String),
    Error(String),
}

/// Input mode
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
}

/// Application state
pub struct App {
    /// Messages in the chat
    pub messages: VecDeque<ChatMessage>,
    /// Current input buffer
    pub input: String,
    /// Cursor position in input
    pub cursor_position: usize,
    /// Input mode
    pub input_mode: InputMode,
    /// Scroll offset for message history
    pub scroll_offset: usize,
    /// Whether the app should quit
    pub should_quit: bool,
    /// Current model name
    pub model_name: String,
    /// Current cost
    pub cost_usd: f64,
    /// Iteration count
    pub iterations: u32,
    /// Status message
    pub status: String,
    /// Whether agent is processing
    pub is_processing: bool,
    /// Maximum messages to keep in view
    max_messages: usize,
}

impl App {
    pub fn new(model_name: String) -> Self {
        Self {
            messages: VecDeque::new(),
            input: String::new(),
            cursor_position: 0,
            input_mode: InputMode::Editing,
            scroll_offset: 0,
            should_quit: false,
            model_name,
            cost_usd: 0.0,
            iterations: 0,
            status: "Ready".to_string(),
            is_processing: false,
            max_messages: 500,
        }
    }

    /// Add a message
    pub fn push_message(&mut self, msg: ChatMessage) {
        self.messages.push_back(msg);
        if self.messages.len() > self.max_messages {
            self.messages.pop_front();
        }
        // Auto-scroll to bottom
        self.scroll_offset = 0;
    }

    /// Handle character input
    pub fn handle_char(&mut self, c: char) {
        self.input.insert(self.cursor_position, c);
        self.cursor_position += c.len_utf8();
    }

    /// Handle backspace
    pub fn handle_backspace(&mut self) {
        if self.cursor_position > 0 {
            // Find the previous character boundary
            let prev = self.input[..self.cursor_position]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.input.drain(prev..self.cursor_position);
            self.cursor_position = prev;
        }
    }

    /// Handle delete
    pub fn handle_delete(&mut self) {
        if self.cursor_position < self.input.len() {
            let next = self.input[self.cursor_position..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor_position + i)
                .unwrap_or(self.input.len());
            self.input.drain(self.cursor_position..next);
        }
    }

    /// Move cursor left
    pub fn cursor_left(&mut self) {
        if self.cursor_position > 0 {
            let prev = self.input[..self.cursor_position]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.cursor_position = prev;
        }
    }

    /// Move cursor right
    pub fn cursor_right(&mut self) {
        if self.cursor_position < self.input.len() {
            let next = self.input[self.cursor_position..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor_position + i)
                .unwrap_or(self.input.len());
            self.cursor_position = next;
        }
    }

    /// Submit the current input
    pub fn submit_input(&mut self) -> String {
        let input = self.input.clone();
        self.input.clear();
        self.cursor_position = 0;
        input
    }

    /// Scroll up
    pub fn scroll_up(&mut self) {
        if self.scroll_offset < self.messages.len().saturating_sub(1) {
            self.scroll_offset += 1;
        }
    }

    /// Scroll down
    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    /// Clear input
    pub fn clear_input(&mut self) {
        self.input.clear();
        self.cursor_position = 0;
    }

    /// Get visible messages (accounting for scroll)
    pub fn visible_messages(&self) -> Vec<&ChatMessage> {
        let total = self.messages.len();
        let end = total.saturating_sub(self.scroll_offset);
        let start = end.saturating_sub(50); // Show up to 50 messages at once
        self.messages.range(start..end).collect()
    }
}
