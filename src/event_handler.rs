use crate::command::{Command, parse_command, CommandExecutor};
use crate::config::Config;
use crate::log;
use crate::operations::{batch, batch_window, clipboard, manual_trace, navigation, search, traces};
use crate::process::ProcessManager;
use crate::ui::{self, App, apply_filters};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
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
            KeyCode::Char('?') if !self.app.command_mode && !self.app.search_mode => {
                self.handle_help_toggle();
                Ok(false)
            }
            // Help overlay scrolling (must come before other navigation handlers)
            KeyCode::Up | KeyCode::Char('k') if self.app.show_help => {
                self.app.scroll_help_up();
                Ok(false)
            }
            KeyCode::Down | KeyCode::Char('j') if self.app.show_help => {
                self.app.scroll_help_down();
                Ok(false)
            }
            // All Esc handling in one place
            KeyCode::Esc => {
                self.handle_escape();
                Ok(false)
            }
            // Expanded line view
            KeyCode::Enter if self.app.expanded_line_view => {
                self.handle_show_context();
                Ok(false)
            }
            // Command mode
            KeyCode::Char(':') if !self.app.command_mode && !self.app.search_mode && !self.app.show_help => {
                self.app.enter_command_mode();
                Ok(false)
            }
            KeyCode::Enter if self.app.command_mode => {
                self.handle_command_execute().await
            }
            KeyCode::Backspace if self.app.command_mode => {
                self.app.delete_char();
                Ok(false)
            }
            KeyCode::Up if self.app.command_mode => {
                self.app.history_prev();
                Ok(false)
            }
            KeyCode::Down if self.app.command_mode => {
                self.app.history_next();
                Ok(false)
            }
            KeyCode::Char(c) if self.app.command_mode => {
                self.app.add_char(c);
                Ok(false)
            }
            // Search mode
            KeyCode::Char('/') if !self.app.command_mode && !self.app.search_mode && !self.app.show_help => {
                self.app.enter_search_mode();
                Ok(false)
            }
            KeyCode::Enter if self.app.search_mode => {
                self.handle_search_execute();
                Ok(false)
            }
            KeyCode::Backspace if self.app.search_mode => {
                self.app.delete_char();
                Ok(false)
            }
            KeyCode::Char(c) if self.app.search_mode => {
                self.app.add_char(c);
                Ok(false)
            }
            // Trace selection mode
            KeyCode::Enter if self.app.trace_selection_mode => {
                traces::select_trace(self.app, self.manager);
                Ok(false)
            }
            KeyCode::Up if self.app.trace_selection_mode => {
                self.app.select_prev_trace();
                Ok(false)
            }
            KeyCode::Down if self.app.trace_selection_mode => {
                self.app.select_next_trace();
                Ok(false)
            }
            KeyCode::Char('k') if self.app.trace_selection_mode => {
                self.app.select_prev_trace();
                Ok(false)
            }
            KeyCode::Char('j') if self.app.trace_selection_mode => {
                self.app.select_next_trace();
                Ok(false)
            }
            // Trace filter mode
            KeyCode::Char('[') if self.app.trace_filter_mode => {
                traces::expand_trace_before(self.app);
                Ok(false)
            }
            KeyCode::Char(']') if self.app.trace_filter_mode => {
                traces::expand_trace_after(self.app);
                Ok(false)
            }
            KeyCode::Enter if !self.app.command_mode && !self.app.search_mode && !self.app.expanded_line_view => {
                self.handle_toggle_expanded_view();
                Ok(false)
            }
            // Batch navigation
            KeyCode::Char('[') if !self.app.command_mode && !self.app.search_mode => {
                self.handle_prev_batch();
                Ok(false)
            }
            KeyCode::Char(']') if !self.app.command_mode && !self.app.search_mode => {
                self.handle_next_batch();
                Ok(false)
            }
            // Batch window adjustment
            KeyCode::Char('+') if !self.app.command_mode && !self.app.search_mode => {
                self.handle_increase_batch_window();
                Ok(false)
            }
            KeyCode::Char('-') if !self.app.command_mode && !self.app.search_mode => {
                self.handle_decrease_batch_window();
                Ok(false)
            }
            // Clipboard operations
            KeyCode::Char('c') if !self.app.command_mode && !self.app.search_mode && !self.app.expanded_line_view => {
                self.handle_copy_line();
                Ok(false)
            }
            KeyCode::Char('C') if !self.app.command_mode && !self.app.search_mode && !self.app.expanded_line_view => {
                self.handle_copy_batch();
                Ok(false)
            }
            // Vim-style page navigation (Ctrl+B = page up, Ctrl+F = page down)
            // IMPORTANT: These must come BEFORE plain 'b' handler to match correctly
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) && !self.app.command_mode && !self.app.search_mode => {
                self.handle_page_up();
                Ok(false)
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) && !self.app.command_mode && !self.app.search_mode => {
                self.handle_page_down();
                Ok(false)
            }
            // Batch focus
            KeyCode::Char('b') if !self.app.command_mode && !self.app.search_mode && !self.app.expanded_line_view => {
                self.handle_focus_batch();
                Ok(false)
            }
            // Manual trace capture
            KeyCode::Char('s') if !self.app.command_mode && !self.app.search_mode
                && !self.app.trace_filter_mode && !self.app.trace_selection_mode
                && !self.app.expanded_line_view => {
                self.handle_manual_trace_toggle();
                Ok(false)
            }
            // Toggle compact mode (condense metadata tags)
            KeyCode::Char('w') if !self.app.command_mode && !self.app.search_mode
                && !self.app.show_help && !self.app.expanded_line_view => {
                self.handle_toggle_compact_mode();
                Ok(false)
            }
            // Line selection and scrolling
            KeyCode::Up if !self.app.command_mode && !self.app.search_mode => {
                self.handle_select_prev_line();
                Ok(false)
            }
            KeyCode::Down if !self.app.command_mode && !self.app.search_mode => {
                self.handle_select_next_line();
                Ok(false)
            }
            KeyCode::PageUp if !self.app.command_mode && !self.app.search_mode => {
                self.handle_page_up();
                Ok(false)
            }
            KeyCode::PageDown if !self.app.command_mode && !self.app.search_mode => {
                self.handle_page_down();
                Ok(false)
            }
            KeyCode::Home if !self.app.command_mode && !self.app.search_mode => {
                self.app.scroll_to_top();
                Ok(false)
            }
            KeyCode::End if !self.app.command_mode && !self.app.search_mode => {
                self.app.scroll_to_bottom();
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
            KeyCode::Char('q') if !self.app.command_mode && !self.app.search_mode && !self.app.shutting_down => {
                self.app.start_shutdown();
                // Set all processes to Terminating status immediately (UI will show this on next draw)
                self.manager.set_all_terminating();
                Ok(false) // Don't quit immediately - let the loop handle killing
            }
            _ => Ok(false),
        }
    }

    fn handle_help_toggle(&mut self) {
        self.app.toggle_help();
    }

    fn handle_toggle_expanded_view(&mut self) {
        if self.app.selected_line_id.is_some() {
            self.app.toggle_expanded_view();
        }
    }

    async fn handle_command_execute(&mut self) -> Result<bool> {
        let cmd_text = self.app.input.clone();
        let cmd = parse_command(&cmd_text);

        // Save to history before processing (don't save empty or quit commands)
        if !cmd_text.trim().is_empty() && !matches!(cmd, Command::Quit) {
            self.app.save_to_history(cmd_text);
        }

        // Handle quit command specially since it breaks the event loop
        if matches!(cmd, Command::Quit) {
            self.app.quit();
            return Ok(true);
        }

        // Execute all other commands using CommandExecutor
        let mut executor = CommandExecutor::new(self.app, self.manager, self.config);
        if let Err(e) = executor.execute(cmd).await {
            self.app.set_status_error(format!("Command error: {}", e));
        }
        self.app.exit_command_mode();
        Ok(false)
    }

    fn handle_search_execute(&mut self) {
        let search_text = self.app.input.clone();
        if let Err(msg) = search::execute_search(self.app, self.manager, &search_text) {
            if msg != "Empty search" {
                self.app.set_status_error(msg);
            }
        }
    }


    fn handle_show_context(&mut self) {
        match search::show_context(self.app, self.manager) {
            Ok(msg) => self.app.set_status_info(msg),
            Err(msg) => self.app.set_status_error(msg),
        }
    }

    fn handle_increase_batch_window(&mut self) {
        let (new_window, batch_count) = batch_window::increase_batch_window(self.app, self.manager, self.config);
        self.app.set_status_success(format!("Batch window increased to {}ms ({} batches)", new_window, batch_count));
    }

    fn handle_decrease_batch_window(&mut self) {
        let (new_window, batch_count) = batch_window::decrease_batch_window(self.app, self.manager, self.config);
        self.app.set_status_success(format!("Batch window decreased to {}ms ({} batches)", new_window, batch_count));
    }

    fn handle_copy_line(&mut self) {
        match clipboard::copy_line(self.app, self.manager) {
            Ok(msg) => self.app.set_status_success(msg),
            Err(msg) => self.app.set_status_error(msg),
        }
    }

    fn handle_copy_batch(&mut self) {
        match clipboard::copy_batch(self.app, self.manager) {
            Ok(msg) => self.app.set_status_success(msg),
            Err(msg) => self.app.set_status_error(msg),
        }
    }

    fn handle_next_batch(&mut self) {
        batch::next_batch(self.app, self.manager);
    }

    fn handle_prev_batch(&mut self) {
        batch::prev_batch(self.app, self.manager);
    }

    fn handle_focus_batch(&mut self) {
        if let Some(selected_id) = self.app.selected_line_id {
            // Get all logs and apply filters
            let logs = self.manager.get_all_logs();
            let filtered_logs = apply_filters(logs, &self.app.filters);

            // Find the index of the selected log by ID
            let line_idx = match filtered_logs.iter().position(|log| log.id == selected_id) {
                Some(idx) => idx,
                None => {
                    self.app.set_status_error("Selected line not found".to_string());
                    return;
                }
            };

            // Detect batches
            let filtered_refs: Vec<&log::LogLine> = filtered_logs.iter().collect();
            let batches = ui::detect_batches_from_logs(&filtered_refs, self.app.batch_window_ms);

            // Find which batch contains the selected line
            if let Some((batch_idx, _)) = batches.iter().enumerate().find(|(_, (start, end))| {
                line_idx >= *start && line_idx <= *end
            }) {
                // Create snapshot if entering batch view for the first time
                let was_none = !self.app.batch_view_mode;
                if was_none {
                    self.app.create_snapshot(filtered_logs);
                }

                self.app.current_batch = Some(batch_idx);
                self.app.batch_view_mode = true;
                self.app.scroll_offset = 0;
                self.app.set_status_info(format!("Focused on batch {}", batch_idx + 1));
            } else {
                self.app.set_status_error("No batch found for selected line".to_string());
            }
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
        if self.app.manual_trace_recording {
            match manual_trace::stop_recording(self.app, self.manager) {
                Ok(msg) => self.app.set_status_success(msg),
                Err(msg) => self.app.set_status_error(msg),
            }
        } else {
            manual_trace::start_recording(self.app);
        }
    }

    fn handle_toggle_compact_mode(&mut self) {
        self.app.toggle_compact_mode();
        // Persist to config
        self.config.compact_mode = Some(self.app.compact_mode);
        crate::operations::config::save_config_with_error(self.config, self.app);
        let mode = if self.app.compact_mode { "compact" } else { "full" };
        self.app.set_status_info(format!("Display mode: {}", mode));
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
    /// 8. Frozen with selection - clear selection
    /// 9. Frozen without selection - unfreeze and resume tailing
    /// 10. Batch view mode - exit batch view
    /// 11. Default - jump to latest logs
    fn handle_escape(&mut self) {
        // 0. Manual trace recording
        if self.app.manual_trace_recording {
            manual_trace::cancel_recording(self.app);
            self.app.set_status_info("Recording cancelled".to_string());
            return;
        }

        // 1. Help overlay
        if self.app.show_help {
            self.app.toggle_help();
            return;
        }

        // 2. Expanded line view (modal)
        if self.app.expanded_line_view {
            self.app.close_expanded_view();
            return;
        }

        // 3. Command mode
        if self.app.command_mode {
            self.app.exit_command_mode();
            return;
        }

        // 4. Search mode (actively typing)
        if self.app.search_mode {
            self.app.exit_search_mode();
            return;
        }

        // 5. Trace selection mode
        if self.app.trace_selection_mode {
            self.app.exit_trace_selection();
            self.app.set_status_info("Trace selection cancelled".to_string());
            return;
        }

        // 6. Search results with selection - return to search input
        if !self.app.search_pattern.is_empty() && self.app.selected_line_id.is_some() {
            self.app.selected_line_id = None;
            self.app.unfreeze_display();
            self.app.discard_snapshot();
            self.app.search_mode = true;
            self.app.input = self.app.search_pattern.clone();
            return;
        }

        // 7. Trace filter mode
        if self.app.trace_filter_mode {
            self.app.exit_trace_filter();
            self.app.discard_snapshot();
            self.app.set_status_info("Exited trace view".to_string());
            return;
        }

        // 8-9. Frozen state (two-stage Esc)
        if self.app.frozen {
            if self.app.selected_line_id.is_some() {
                // First Esc: clear selection but stay frozen
                self.app.selected_line_id = None;
                self.app.set_status_info("Selection cleared. Press Esc again to resume tailing.".to_string());
            } else {
                // Second Esc: unfreeze and resume tailing
                self.app.unfreeze_display();
                self.app.discard_snapshot();
                self.app.clear_search();
                self.app.scroll_to_bottom();
                self.app.set_status_info("Resumed tailing".to_string());
            }
            return;
        }

        // 10. Batch view mode
        if self.app.batch_view_mode {
            self.app.batch_view_mode = false;
            self.app.current_batch = None;
            self.app.discard_snapshot();
            self.app.clear_search();
            self.app.scroll_to_bottom();
            self.app.set_status_info("Exited batch view, resumed tailing".to_string());
            return;
        }

        // 11. Default - jump to latest
        self.app.clear_search();
        self.app.scroll_to_bottom();
        self.app.selected_line_id = None;
        self.app.set_status_info("Jumped to latest logs".to_string());
    }
}
