use crate::command::{Command, parse_command, CommandExecutor};
use crate::config::Config;
use crate::operations::{batch, batch_window, clipboard, display, manual_trace, navigation, search, traces};
use crate::process::ProcessManager;
use crate::ui::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind, MouseButton};
use anyhow::Result;

pub struct EventHandler<'a> {
    app: &'a mut App,
    manager: &'a mut ProcessManager,
    config: &'a mut Config,
}

impl<'a> EventHandler<'a> {
    pub fn new(app: &'a mut App, manager: &'a mut ProcessManager, config: &'a mut Config) -> Self {
        Self {
            app,
            manager,
            config,
        }
    }

    pub async fn handle_key_event(&mut self, key: KeyEvent) -> Result<bool> {
        // Returns true if the app should quit, false otherwise
        match key.code {
            // Ctrl-C triggers graceful shutdown
            // In raw mode, Ctrl+C is captured as a keyboard event, not a signal
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) && !self.app.shutting_down => {
                self.app.start_shutdown();
                // Set all processes to Terminating status immediately (UI will show this on next draw)
                self.manager.set_all_terminating();
                Ok(false) // Don't quit immediately - let the loop handle killing
            }
            // Help mode
            KeyCode::Char('?') if !self.app.input.command_mode && !self.app.input.search_mode => {
                self.handle_help_toggle();
                Ok(false)
            }
            // Help overlay scrolling (must come before other navigation handlers)
            KeyCode::Up | KeyCode::Char('k') if self.app.display.show_help => {
                self.app.display.scroll_help_up();
                Ok(false)
            }
            KeyCode::Down | KeyCode::Char('j') if self.app.display.show_help => {
                self.app.display.scroll_help_down();
                Ok(false)
            }
            // All Esc handling in one place
            KeyCode::Esc => {
                self.handle_escape();
                Ok(false)
            }
            // Expanded line view
            KeyCode::Enter if self.app.display.expanded_line_view => {
                self.handle_show_context();
                Ok(false)
            }
            KeyCode::Char('c') if self.app.display.expanded_line_view => {
                self.handle_copy_line();
                Ok(false)
            }
            // Command mode
            KeyCode::Char(':') if !self.app.input.command_mode && !self.app.input.search_mode && !self.app.display.show_help => {
                self.app.input.enter_command_mode();
                self.app.display.status_message = None;
                Ok(false)
            }
            KeyCode::Enter if self.app.input.command_mode => {
                self.handle_command_execute().await
            }
            KeyCode::Backspace if self.app.input.command_mode => {
                self.app.input.delete_char();
                Ok(false)
            }
            KeyCode::Up if self.app.input.command_mode => {
                self.app.input.history_prev();
                Ok(false)
            }
            KeyCode::Down if self.app.input.command_mode => {
                self.app.input.history_next();
                Ok(false)
            }
            KeyCode::Char(c) if self.app.input.command_mode => {
                self.app.input.add_char(c);
                Ok(false)
            }
            // Search mode
            KeyCode::Char('/') if !self.app.input.command_mode && !self.app.input.search_mode && !self.app.display.show_help => {
                self.app.input.enter_search_mode();
                Ok(false)
            }
            KeyCode::Enter if self.app.input.search_mode => {
                self.handle_search_execute();
                Ok(false)
            }
            KeyCode::Backspace if self.app.input.search_mode => {
                self.app.input.delete_char();
                Ok(false)
            }
            KeyCode::Char(c) if self.app.input.search_mode => {
                self.app.input.add_char(c);
                Ok(false)
            }
            // Trace selection mode
            KeyCode::Enter if self.app.trace.trace_selection_mode => {
                traces::select_trace(self.app, self.manager);
                Ok(false)
            }
            KeyCode::Up if self.app.trace.trace_selection_mode => {
                self.app.trace.select_prev_trace();
                Ok(false)
            }
            KeyCode::Down if self.app.trace.trace_selection_mode => {
                self.app.trace.select_next_trace();
                Ok(false)
            }
            KeyCode::Char('k') if self.app.trace.trace_selection_mode => {
                self.app.trace.select_prev_trace();
                Ok(false)
            }
            KeyCode::Char('j') if self.app.trace.trace_selection_mode => {
                self.app.trace.select_next_trace();
                Ok(false)
            }
            // Trace filter mode
            KeyCode::Char('[') if self.app.trace.trace_filter_mode => {
                traces::expand_trace_before(self.app);
                Ok(false)
            }
            KeyCode::Char(']') if self.app.trace.trace_filter_mode => {
                traces::expand_trace_after(self.app);
                Ok(false)
            }
            KeyCode::Enter if !self.app.input.command_mode && !self.app.input.search_mode && !self.app.display.expanded_line_view => {
                self.handle_toggle_expanded_view();
                Ok(false)
            }
            // Batch navigation
            KeyCode::Char('[') if !self.app.input.command_mode && !self.app.input.search_mode => {
                self.handle_prev_batch();
                Ok(false)
            }
            KeyCode::Char(']') if !self.app.input.command_mode && !self.app.input.search_mode => {
                self.handle_next_batch();
                Ok(false)
            }
            // Batch window adjustment
            KeyCode::Char('+') if !self.app.input.command_mode && !self.app.input.search_mode => {
                self.handle_increase_batch_window();
                Ok(false)
            }
            KeyCode::Char('-') if !self.app.input.command_mode && !self.app.input.search_mode => {
                self.handle_decrease_batch_window();
                Ok(false)
            }
            // Clipboard operations
            KeyCode::Char('c') if !self.app.input.command_mode && !self.app.input.search_mode && !self.app.display.expanded_line_view => {
                self.handle_copy_line();
                Ok(false)
            }
            // Contextual copy - same process within time window (Shift+X)
            KeyCode::Char('X') if !self.app.input.command_mode && !self.app.input.search_mode => {
                self.handle_copy_time_context();
                Ok(false)
            }
            KeyCode::Char('C') if !self.app.input.command_mode && !self.app.input.search_mode && !self.app.display.expanded_line_view => {
                self.handle_copy_batch();
                Ok(false)
            }
            // Vim-style page navigation (Ctrl+B = page up, Ctrl+F = page down)
            // IMPORTANT: These must come BEFORE plain 'b' handler to match correctly
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) && !self.app.input.command_mode && !self.app.input.search_mode => {
                self.handle_page_up();
                Ok(false)
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) && !self.app.input.command_mode && !self.app.input.search_mode => {
                self.handle_page_down();
                Ok(false)
            }
            // Batch focus
            KeyCode::Char('b') if !self.app.input.command_mode && !self.app.input.search_mode && !self.app.display.expanded_line_view => {
                self.handle_focus_batch();
                Ok(false)
            }
            // Manual trace capture
            KeyCode::Char('s') if !self.app.input.command_mode && !self.app.input.search_mode
                && !self.app.trace.trace_filter_mode && !self.app.trace.trace_selection_mode
                && !self.app.display.expanded_line_view => {
                self.handle_manual_trace_toggle();
                Ok(false)
            }
            // Cycle display mode (compact/full/wrap)
            KeyCode::Char('w') if !self.app.input.command_mode && !self.app.input.search_mode
                && !self.app.display.show_help && !self.app.display.expanded_line_view => {
                self.handle_cycle_display_mode();
                Ok(false)
            }
            // Cycle timestamp mode (seconds/milliseconds/off)
            KeyCode::Char('t') if !self.app.input.command_mode && !self.app.input.search_mode
                && !self.app.display.show_help && !self.app.display.expanded_line_view => {
                self.handle_cycle_timestamp_mode();
                Ok(false)
            }
            // Line selection and scrolling
            KeyCode::Up if !self.app.input.command_mode && !self.app.input.search_mode => {
                self.handle_select_prev_line();
                Ok(false)
            }
            KeyCode::Down if !self.app.input.command_mode && !self.app.input.search_mode => {
                self.handle_select_next_line();
                Ok(false)
            }
            KeyCode::PageUp if !self.app.input.command_mode && !self.app.input.search_mode => {
                self.handle_page_up();
                Ok(false)
            }
            KeyCode::PageDown if !self.app.input.command_mode && !self.app.input.search_mode => {
                self.handle_page_down();
                Ok(false)
            }
            KeyCode::Home if !self.app.input.command_mode && !self.app.input.search_mode => {
                self.app.navigation.scroll_to_top();
                Ok(false)
            }
            KeyCode::End if !self.app.input.command_mode && !self.app.input.search_mode => {
                self.app.navigation.scroll_to_bottom();
                Ok(false)
            }
            // Quit - initiate graceful shutdown
            // Use Ctrl+Q to quit from any mode (including command/search mode)
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) && !self.app.shutting_down => {
                self.app.start_shutdown();
                // Set all processes to Terminating status immediately (UI will show this on next draw)
                self.manager.set_all_terminating();
                Ok(false) // Don't quit immediately - let the loop handle killing
            }
            // Regular 'q' only works when not in command/search mode
            KeyCode::Char('q') if !self.app.input.command_mode && !self.app.input.search_mode && !self.app.shutting_down => {
                self.app.start_shutdown();
                // Set all processes to Terminating status immediately (UI will show this on next draw)
                self.manager.set_all_terminating();
                Ok(false) // Don't quit immediately - let the loop handle killing
            }
            _ => Ok(false),
        }
    }

    fn handle_help_toggle(&mut self) {
        self.app.display.toggle_help();
    }

    fn handle_toggle_expanded_view(&mut self) {
        if self.app.navigation.selected_line_id.is_some() {
            self.app.display.toggle_expanded_view();
        }
    }

    async fn handle_command_execute(&mut self) -> Result<bool> {
        let cmd_text = self.app.input.input.clone();
        let cmd = parse_command(&cmd_text);

        // Save to history before processing (don't save empty or quit commands)
        if !cmd_text.trim().is_empty() && !matches!(cmd, Command::Quit) {
            self.app.input.save_to_history(cmd_text);
        }

        // Handle quit command specially since it breaks the event loop
        if matches!(cmd, Command::Quit) {
            self.app.quit();
            return Ok(true);
        }

        // Execute all other commands using CommandExecutor
        let mut executor = CommandExecutor::new(self.app, self.manager, self.config);
        if let Err(e) = executor.execute(cmd).await {
            self.app.display.set_status_error(format!("Command error: {}", e));
        }
        self.app.input.exit_command_mode();
        Ok(false)
    }

    fn handle_search_execute(&mut self) {
        let search_text = self.app.input.input.clone();
        if let Err(msg) = search::execute_search(self.app, self.manager, &search_text) {
            if msg != "Empty search" {
                self.app.display.set_status_error(msg);
            }
        }
    }


    fn handle_show_context(&mut self) {
        match search::show_context(self.app, self.manager) {
            Ok(msg) => self.app.display.set_status_info(msg),
            Err(msg) => self.app.display.set_status_error(msg),
        }
    }

    fn handle_increase_batch_window(&mut self) {
        let (new_window, batch_count) = batch_window::increase_batch_window(self.app, self.manager, self.config);
        self.app.display.set_status_success(format!("Batch window increased to {}ms ({} batches)", new_window, batch_count));
    }

    fn handle_decrease_batch_window(&mut self) {
        let (new_window, batch_count) = batch_window::decrease_batch_window(self.app, self.manager, self.config);
        self.app.display.set_status_success(format!("Batch window decreased to {}ms ({} batches)", new_window, batch_count));
    }

    fn handle_copy_line(&mut self) {
        match clipboard::copy_line(self.app, self.manager) {
            Ok(msg) => self.app.display.set_status_success(msg),
            Err(msg) => self.app.display.set_status_error(msg),
        }
    }

    fn handle_copy_batch(&mut self) {
        match clipboard::copy_batch(self.app, self.manager) {
            Ok(msg) => self.app.display.set_status_success(msg),
            Err(msg) => self.app.display.set_status_error(msg),
        }
    }

    fn handle_copy_time_context(&mut self) {
        let time_window = self.config.context_copy_seconds.unwrap_or(1.0);
        match clipboard::copy_time_context(self.app, self.manager, time_window) {
            Ok(msg) => self.app.display.set_status_success(msg),
            Err(msg) => self.app.display.set_status_error(msg),
        }
    }

    fn handle_next_batch(&mut self) {
        batch::next_batch(self.app, self.manager);
    }

    fn handle_prev_batch(&mut self) {
        batch::prev_batch(self.app, self.manager);
    }

    fn handle_focus_batch(&mut self) {
        match batch::focus_batch(self.app, self.manager) {
            Ok(msg) => self.app.display.set_status_info(msg),
            Err(msg) => self.app.display.set_status_error(msg),
        }
    }

    fn handle_select_prev_line(&mut self) {
        navigation::select_prev_line(self.app, self.manager);
    }

    fn handle_select_next_line(&mut self) {
        navigation::select_next_line(self.app, self.manager);
    }

    fn handle_page_up(&mut self) {
        navigation::page_up(self.app, self.manager);
    }

    fn handle_page_down(&mut self) {
        navigation::page_down(self.app, self.manager);
    }

    fn handle_manual_trace_toggle(&mut self) {
        if self.app.trace.manual_trace_recording {
            match manual_trace::stop_recording(self.app, self.manager) {
                Ok(msg) => self.app.display.set_status_success(msg),
                Err(msg) => self.app.display.set_status_error(msg),
            }
        } else {
            manual_trace::start_recording(self.app);
        }
    }

    fn handle_cycle_display_mode(&mut self) {
        let mode = display::cycle_display_mode(self.app, self.config);
        self.app.display.set_status_info(format!("Display mode: {}", mode));
    }

    fn handle_cycle_timestamp_mode(&mut self) {
        let mode = display::cycle_timestamp_mode(self.app);
        self.app.display.set_status_info(format!("Timestamp: {}", mode));
    }

    /// Handle Esc key - all escape logic in one place for clarity.
    /// Priority order (first match wins):
    /// 0. Manual trace recording - cancel recording
    /// 1. Help overlay - close help
    /// 2. Expanded line view - close modal
    /// 3. Command mode - exit command input
    /// 4. Search mode - exit search input
    /// 5. Trace selection mode - cancel trace selection
    /// 6. Search results with selection - return to search input
    /// 7. Trace filter mode - exit trace view
    /// 8. Frozen state - unfreeze and resume tailing
    /// 9. Batch view mode - exit batch view
    /// 10. Default - jump to latest logs
    fn handle_escape(&mut self) {
        // 0. Manual trace recording
        if self.app.trace.manual_trace_recording {
            manual_trace::cancel_recording(self.app);
            self.app.display.set_status_info("Recording cancelled".to_string());
            return;
        }

        // 1. Help overlay
        if self.app.display.show_help {
            self.app.display.toggle_help();
            return;
        }

        // 2. Expanded line view (modal)
        if self.app.display.expanded_line_view {
            self.app.display.close_expanded_view();
            return;
        }

        // 3. Command mode
        if self.app.input.command_mode {
            self.app.input.exit_command_mode();
            return;
        }

        // 4. Search mode (actively typing)
        if self.app.input.search_mode {
            self.app.input.exit_search_mode();
            return;
        }

        // 5. Trace selection mode
        if self.app.trace.trace_selection_mode {
            self.app.trace.exit_trace_selection();
            self.app.display.set_status_info("Trace selection cancelled".to_string());
            return;
        }

        // 6. Search results with selection - return to search input
        if !self.app.input.search_pattern.is_empty() && self.app.navigation.selected_line_id.is_some() {
            self.app.navigation.selected_line_id = None;
            self.app.navigation.unfreeze_display();
            self.app.navigation.discard_snapshot();
            self.app.input.search_mode = true;
            self.app.input.input = self.app.input.search_pattern.clone();
            return;
        }

        // 7. Trace filter mode
        if self.app.trace.trace_filter_mode {
            self.app.trace.exit_trace_filter();
            self.app.navigation.unfreeze_display();
            self.app.navigation.selected_line_id = None;
            self.app.navigation.discard_snapshot();
            self.app.display.set_status_info("Exited trace view".to_string());
            return;
        }

        // 8. Frozen state - single Esc resumes tailing
        if self.app.navigation.frozen {
            self.app.navigation.selected_line_id = None;
            self.app.navigation.unfreeze_display();
            self.app.navigation.discard_snapshot();
            self.app.input.clear_search();
            self.app.navigation.scroll_to_bottom();
            self.app.display.set_status_info("Resumed tailing".to_string());
            return;
        }

        // 9. Batch view mode
        if self.app.batch.batch_view_mode {
            self.app.batch.batch_view_mode = false;
            self.app.batch.current_batch = None;
            self.app.navigation.discard_snapshot();
            self.app.input.clear_search();
            self.app.navigation.scroll_to_bottom();
            self.app.display.set_status_info("Exited batch view, resumed tailing".to_string());
            return;
        }

        // 10. Default - jump to latest
        self.app.input.clear_search();
        self.app.navigation.scroll_to_bottom();
        self.app.navigation.selected_line_id = None;
        self.app.display.set_status_info("Jumped to latest logs".to_string());
    }

    pub fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<bool> {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let col = mouse.column;
                let row = mouse.row;
                let pos = ratatui::layout::Position::new(col, row);

                // Check which region was clicked
                if let Some(area) = self.app.regions.process_list_area {
                    if area.contains(pos) {
                        // Check if click is on a specific process
                        for (name, rect) in &self.app.regions.process_regions.clone() {
                            if rect.contains(pos) {
                                // 3-state cycle: Solo -> Mute -> Normal -> Solo...
                                let all_names: Vec<String> = self
                                    .app
                                    .regions
                                    .process_regions
                                    .iter()
                                    .map(|(n, _)| n.clone())
                                    .collect();
                                let total = all_names.len();
                                let hidden_count = self.app.filters.hidden_processes.len();
                                let is_hidden = self.app.filters.hidden_processes.contains(name);

                                if !is_hidden && hidden_count == total - 1 {
                                    // Solo mode (this is the only visible process) -> Mute mode
                                    self.app.filters.hidden_processes.clear();
                                    self.app.filters.hidden_processes.insert(name.clone());
                                    self.app.display.set_status_info(format!("Muting: {}", name));
                                } else if is_hidden && hidden_count == 1 {
                                    // Mute mode (this is the only hidden process) -> Normal mode
                                    self.app.filters.hidden_processes.clear();
                                    self.app.display.set_status_info("Showing all".to_string());
                                } else {
                                    // Normal or other state -> Solo mode
                                    self.app.filters.hidden_processes.clear();
                                    for n in &all_names {
                                        if n != name {
                                            self.app.filters.hidden_processes.insert(n.clone());
                                        }
                                    }
                                    self.app.display.set_status_info(format!("Solo: {}", name));
                                }
                                return Ok(false);
                            }
                        }
                        // Click was in process list area but not on a specific process
                        return Ok(false);
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                // Use selection navigation (same as keyboard) to properly enter selection mode
                for _ in 0..3 {
                    navigation::select_prev_line(self.app, self.manager);
                }
            }
            MouseEventKind::ScrollDown => {
                // Use selection navigation (same as keyboard) to properly enter selection mode
                for _ in 0..3 {
                    navigation::select_next_line(self.app, self.manager);
                }
            }
            _ => {}
        }
        Ok(false)
    }
}
