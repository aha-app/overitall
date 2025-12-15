use crate::command::{Command, parse_command, CommandExecutor};
use crate::config::Config;
use crate::log;
use crate::operations::{batch, batch_window, clipboard, navigation};
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
            KeyCode::Esc if self.app.show_help => {
                self.handle_help_toggle();
                Ok(false)
            }
            // Expanded line view
            KeyCode::Esc if self.app.expanded_line_view => {
                self.app.close_expanded_view();
                Ok(false)
            }
            KeyCode::Enter if self.app.expanded_line_view => {
                self.handle_show_context();
                Ok(false)
            }
            KeyCode::Enter if !self.app.command_mode && !self.app.search_mode && !self.app.expanded_line_view => {
                self.handle_toggle_expanded_view();
                Ok(false)
            }
            // Command mode
            KeyCode::Char(':') if !self.app.command_mode && !self.app.search_mode && !self.app.show_help => {
                self.app.enter_command_mode();
                Ok(false)
            }
            KeyCode::Esc if self.app.command_mode => {
                self.app.exit_command_mode();
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
            // Esc in selection mode with active search pattern - return to search typing
            KeyCode::Esc if !self.app.search_mode && !self.app.search_pattern.is_empty() && self.app.selected_line_index.is_some() => {
                // Return to search typing mode
                self.app.selected_line_index = None;
                self.app.unfreeze_display();
                self.app.discard_snapshot();
                // Re-enter search mode with the saved pattern
                self.app.search_mode = true;
                self.app.input = self.app.search_pattern.clone();
                Ok(false)
            }
            // Esc in search typing mode - exit search completely
            KeyCode::Esc if self.app.search_mode => {
                self.app.exit_search_mode();
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
            // Reset to latest logs with Esc
            KeyCode::Esc if !self.app.command_mode && !self.app.search_mode => {
                self.handle_reset_to_latest();
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
        if self.app.selected_line_index.is_some() {
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
        if search_text.is_empty() {
            return;
        }

        // Save the search pattern
        self.app.perform_search(search_text.clone());

        // Get filtered logs (after persistent filters AND search filter)
        let logs = self.manager.get_all_logs();
        let filtered_logs = apply_filters(logs, &self.app.filters);

        // Apply search filter
        let search_filtered: Vec<_> = filtered_logs
            .into_iter()
            .filter(|log| {
                log.line
                    .to_lowercase()
                    .contains(&search_text.to_lowercase())
            })
            .collect();

        if search_filtered.is_empty() {
            self.app.set_status_error("No matches found".to_string());
            return;
        }

        // Create snapshot
        self.app.create_snapshot(search_filtered.clone());

        // Freeze display
        self.app.freeze_display();

        // Select the last (bottom) entry
        let last_index = search_filtered.len().saturating_sub(1);
        self.app.selected_line_index = Some(last_index);

        // Exit search_mode so user can't type (but keep search_pattern)
        self.app.search_mode = false;
        self.app.input.clear();
    }


    fn handle_show_context(&mut self) {
        // Close expanded view
        self.app.close_expanded_view();

        // Get the currently selected log line (before we change anything)
        let selected_log = if let Some(idx) = self.app.selected_line_index {
            if let Some(snapshot) = &self.app.snapshot {
                snapshot.get(idx).cloned()
            } else {
                None
            }
        } else {
            None
        };

        if selected_log.is_none() {
            return;
        }
        let selected_log = selected_log.unwrap();

        // Clear search pattern to show all logs
        self.app.search_pattern.clear();

        // Get ALL filtered logs (persistent filters only, no search)
        let logs = self.manager.get_all_logs();
        let filtered_logs = apply_filters(logs, &self.app.filters);

        // Find the index of the selected log in the full filtered set
        // Match by timestamp and line content for uniqueness
        let new_index = filtered_logs.iter().position(|log| {
            log.timestamp == selected_log.timestamp && log.line == selected_log.line
        });

        if new_index.is_none() {
            self.app.set_status_error("Could not find log in context".to_string());
            return;
        }

        // Create new snapshot with all logs
        self.app.create_snapshot(filtered_logs);

        // Update selection to point to the same log in the full context
        self.app.selected_line_index = new_index;

        // Display is already frozen, keep it that way
        self.app.set_status_info("Showing context around selected log".to_string());
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
        if let Some(line_idx) = self.app.selected_line_index {
            // Get all logs and apply filters
            let logs = self.manager.get_all_logs();
            let filtered_logs = apply_filters(logs, &self.app.filters);

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

    fn handle_reset_to_latest(&mut self) {
        // Two-stage Esc behavior when frozen:
        // 1. First Esc: clear selection, stay frozen at current position
        // 2. Second Esc: unfreeze and resume tailing
        //
        // Also handle batch view mode exit

        if self.app.frozen {
            if self.app.selected_line_index.is_some() {
                // First Esc: clear selection but stay frozen
                self.app.selected_line_index = None;
                self.app.set_status_info("Selection cleared. Press Esc again to resume tailing.".to_string());
            } else {
                // Second Esc: unfreeze and resume tailing
                self.app.unfreeze_display();
                self.app.discard_snapshot();
                self.app.clear_search();
                self.app.scroll_to_bottom();
                self.app.set_status_info("Resumed tailing".to_string());
            }
        } else if self.app.batch_view_mode {
            // Exit batch view mode and discard snapshot
            self.app.batch_view_mode = false;
            self.app.current_batch = None;
            self.app.discard_snapshot();
            self.app.clear_search();
            self.app.scroll_to_bottom();
            self.app.set_status_info("Exited batch view, resumed tailing".to_string());
        } else {
            // Not frozen or in batch view, just jump to latest
            self.app.clear_search();
            self.app.scroll_to_bottom();
            self.app.selected_line_index = None;
            self.app.set_status_info("Jumped to latest logs".to_string());
        }
    }
}
