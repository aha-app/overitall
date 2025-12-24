/// Input and command state for the TUI
#[derive(Debug, Default)]
pub struct InputState {
    /// Current command/search input text
    pub input: String,
    /// Whether we're in command mode (user is typing a command)
    pub command_mode: bool,
    /// Whether we're in search mode (user is typing a search)
    pub search_mode: bool,
    /// Current search pattern
    pub search_pattern: String,
    /// Command history for Up/Down navigation
    pub command_history: Vec<String>,
    /// Current position in history (None = not navigating)
    pub history_index: Option<usize>,
}

impl InputState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a character to the command or search input
    pub fn add_char(&mut self, c: char) {
        if self.command_mode {
            self.reset_history_nav();
            self.input.push(c);
        } else if self.search_mode {
            self.input.push(c);
        }
    }

    /// Delete the last character from the command or search input
    pub fn delete_char(&mut self) {
        if self.command_mode || self.search_mode {
            self.input.pop();
        }
    }

    pub fn enter_command_mode(&mut self) {
        self.command_mode = true;
        self.input.clear();
        self.history_index = None;
    }

    pub fn exit_command_mode(&mut self) {
        self.command_mode = false;
        self.input.clear();
    }

    pub fn enter_search_mode(&mut self) {
        self.search_mode = true;
        self.input.clear();
    }

    pub fn exit_search_mode(&mut self) {
        self.search_mode = false;
        self.input.clear();
        self.search_pattern.clear();
    }

    pub fn perform_search(&mut self, pattern: String) {
        self.search_pattern = pattern;
    }

    pub fn clear_search(&mut self) {
        self.search_pattern.clear();
    }

    pub fn save_to_history(&mut self, command: String) {
        if !command.is_empty() {
            self.command_history.push(command);
        }
    }

    /// Navigate backward in history (Up arrow)
    pub fn history_prev(&mut self) {
        if self.command_history.is_empty() {
            return;
        }

        let new_index = match self.history_index {
            None => self.command_history.len() - 1,
            Some(0) => 0,
            Some(i) => i - 1,
        };

        self.history_index = Some(new_index);
        self.input = self.command_history[new_index].clone();
    }

    /// Navigate forward in history (Down arrow)
    pub fn history_next(&mut self) {
        if self.command_history.is_empty() {
            return;
        }

        match self.history_index {
            None => {}
            Some(i) if i >= self.command_history.len() - 1 => {
                self.history_index = None;
                self.input.clear();
            }
            Some(i) => {
                let new_index = i + 1;
                self.history_index = Some(new_index);
                self.input = self.command_history[new_index].clone();
            }
        }
    }

    /// Reset history navigation (call when user starts typing)
    pub fn reset_history_nav(&mut self) {
        self.history_index = None;
    }
}
