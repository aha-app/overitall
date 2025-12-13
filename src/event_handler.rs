use crate::command::{Command, parse_command, CommandExecutor};
use crate::config::Config;
use crate::log;
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
                self.app.close_expanded_view();
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
        if !search_text.is_empty() {
            self.app.perform_search(search_text);
        }
        self.app.exit_search_mode();
    }


    fn handle_increase_batch_window(&mut self) {
        let new_window = self.app.batch_window_ms + 100;
        self.app.set_batch_window(new_window);
        // Count batches with the new window to show in status
        let logs = self.manager.get_all_logs();
        let filtered_logs = apply_filters(logs, &self.app.filters);
        let filtered_refs: Vec<&log::LogLine> = filtered_logs.iter().collect();
        let batches = ui::detect_batches_from_logs(&filtered_refs, new_window);
        self.app.set_status_success(format!("Batch window increased to {}ms ({} batches)", new_window, batches.len()));

        // Save to config
        self.config.batch_window_ms = Some(new_window);
        if let Some(config_path) = &self.config.config_path {
            if let Err(e) = self.config.save_to_file(config_path) {
            }
        }
    }

    fn handle_decrease_batch_window(&mut self) {
        let new_window = (self.app.batch_window_ms - 100).max(1);
        self.app.set_batch_window(new_window);
        // Count batches with the new window to show in status
        let logs = self.manager.get_all_logs();
        let filtered_logs = apply_filters(logs, &self.app.filters);
        let filtered_refs: Vec<&log::LogLine> = filtered_logs.iter().collect();
        let batches = ui::detect_batches_from_logs(&filtered_refs, new_window);
        self.app.set_status_success(format!("Batch window decreased to {}ms ({} batches)", new_window, batches.len()));

        // Save to config
        self.config.batch_window_ms = Some(new_window);
        if let Some(config_path) = &self.config.config_path {
            if let Err(e) = self.config.save_to_file(config_path) {
            }
        }
    }

    fn handle_copy_line(&mut self) {
        if let Some(line_idx) = self.app.selected_line_index {
            // Get all logs and apply filters (same logic as in draw_expanded_line_overlay)
            let logs = self.manager.get_all_logs();
            let filtered_logs = apply_filters(logs, &self.app.filters);

            // Apply batch view mode filtering if enabled
            let filtered_refs: Vec<&log::LogLine> = filtered_logs.iter().collect();
            let batches = ui::detect_batches_from_logs(&filtered_refs, self.app.batch_window_ms);
            let display_logs: Vec<_> = if self.app.batch_view_mode {
                if let Some(batch_idx) = self.app.current_batch {
                    if !batches.is_empty() && batch_idx < batches.len() {
                        let (start, end) = batches[batch_idx];
                        filtered_logs[start..=end].to_vec()
                    } else {
                        filtered_logs
                    }
                } else {
                    filtered_logs
                }
            } else {
                filtered_logs
            };

            if line_idx < display_logs.len() {
                let log = &display_logs[line_idx];
                let formatted = format!(
                    "[{}] {}: {}",
                    log.timestamp.format("%Y-%m-%d %H:%M:%S"),
                    log.source.process_name(),
                    log.line
                );

                match overitall::clipboard::copy_to_clipboard(&formatted) {
                    Ok(_) => self.app.set_status_success("Copied line to clipboard".to_string()),
                    Err(e) => self.app.set_status_error(format!("Failed to copy: {}", e)),
                }
            }
        }
    }

    fn handle_copy_batch(&mut self) {
        if let Some(line_idx) = self.app.selected_line_index {
            // Get all logs and apply filters
            let logs = self.manager.get_all_logs();
            let filtered_logs = apply_filters(logs, &self.app.filters);

            // Detect batches
            let filtered_refs: Vec<&log::LogLine> = filtered_logs.iter().collect();
            let batches = ui::detect_batches_from_logs(&filtered_refs, self.app.batch_window_ms);

            // Find which batch contains the selected line
            if let Some((batch_idx, (start, end))) = batches.iter().enumerate().find(|(_, (start, end))| {
                line_idx >= *start && line_idx <= *end
            }) {
                // Format the entire batch
                let mut batch_text = format!("=== Batch {} ({} lines) ===\n", batch_idx + 1, end - start + 1);

                for log in &filtered_logs[*start..=*end] {
                    batch_text.push_str(&format!(
                        "[{}] {}: {}\n",
                        log.timestamp.format("%Y-%m-%d %H:%M:%S"),
                        log.source.process_name(),
                        log.line
                    ));
                }

                match overitall::clipboard::copy_to_clipboard(&batch_text) {
                    Ok(_) => self.app.set_status_success(format!("Copied batch to clipboard ({} lines)", end - start + 1)),
                    Err(e) => self.app.set_status_error(format!("Failed to copy: {}", e)),
                }
            } else {
                self.app.set_status_error("No batch found for selected line".to_string());
            }
        }
    }

    fn handle_next_batch(&mut self) {
        // Get filtered logs and create snapshot on first entry to batch view mode
        let logs = self.manager.get_all_logs();
        let filtered_logs = apply_filters(logs, &self.app.filters);

        // Create snapshot if entering batch view for the first time
        let was_none = !self.app.batch_view_mode;
        if was_none {
            self.app.create_snapshot(filtered_logs);
        }

        self.app.next_batch();
    }

    fn handle_prev_batch(&mut self) {
        // Get filtered logs and create snapshot on first entry to batch view mode
        let logs = self.manager.get_all_logs();
        let filtered_logs = apply_filters(logs, &self.app.filters);

        // Create snapshot if entering batch view for the first time
        let was_none = !self.app.batch_view_mode;
        if was_none {
            self.app.create_snapshot(filtered_logs);
        }

        self.app.prev_batch();
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
        // Line selection: select previous line with wrap-around
        // Calculate the correct max based on filtered logs and batch view mode
        let logs = self.manager.get_all_logs();
        let filtered_logs = apply_filters(logs, &self.app.filters);

        // If in batch view mode, limit to current batch
        let filtered_refs: Vec<&log::LogLine> = filtered_logs.iter().collect();
        let total_logs = if self.app.batch_view_mode {
            if let Some(batch_idx) = self.app.current_batch {
                let batches = ui::detect_batches_from_logs(&filtered_refs, self.app.batch_window_ms);
                if !batches.is_empty() && batch_idx < batches.len() {
                    let (start, end) = batches[batch_idx];
                    end - start + 1
                } else {
                    filtered_logs.len()
                }
            } else {
                filtered_logs.len()
            }
        } else {
            filtered_logs.len()
        };

        // Create snapshot on first selection
        let was_none = self.app.selected_line_index.is_none();
        if was_none {
            self.app.create_snapshot(filtered_logs.clone());
        }

        self.app.select_prev_line(total_logs);
    }

    fn handle_select_next_line(&mut self) {
        // Line selection: select next line
        // Calculate the correct max based on filtered logs and batch view mode
        let logs = self.manager.get_all_logs();
        let filtered_logs = apply_filters(logs, &self.app.filters);

        // If in batch view mode, limit to current batch
        let filtered_refs: Vec<&log::LogLine> = filtered_logs.iter().collect();
        let total_logs = if self.app.batch_view_mode {
            if let Some(batch_idx) = self.app.current_batch {
                let batches = ui::detect_batches_from_logs(&filtered_refs, self.app.batch_window_ms);
                if !batches.is_empty() && batch_idx < batches.len() {
                    let (start, end) = batches[batch_idx];
                    end - start + 1
                } else {
                    filtered_logs.len()
                }
            } else {
                filtered_logs.len()
            }
        } else {
            filtered_logs.len()
        };

        // Create snapshot on first selection
        let was_none = self.app.selected_line_index.is_none();
        if was_none {
            self.app.create_snapshot(filtered_logs.clone());
        }

        self.app.select_next_line(total_logs);
    }

    fn handle_page_up(&mut self) {
        // If a line is selected, move the selection by a page
        if self.app.selected_line_index.is_some() {
            // Move selection up by a page (20 lines)
            let page_size = 20;
            if let Some(current_idx) = self.app.selected_line_index {
                let new_idx = current_idx.saturating_sub(page_size);
                self.app.selected_line_index = Some(new_idx);
                self.app.auto_scroll = false;
            }
        } else {
            // No line selected, just scroll the view
            self.app.scroll_up(20);
        }
    }

    fn handle_page_down(&mut self) {
        // If a line is selected, move the selection by a page
        if self.app.selected_line_index.is_some() {
            // Calculate the correct max based on filtered logs and batch view mode
            let logs = self.manager.get_all_logs();
            let filtered_logs = apply_filters(logs, &self.app.filters);

            let filtered_refs: Vec<&log::LogLine> = filtered_logs.iter().collect();
            let total_logs = if self.app.batch_view_mode {
                if let Some(batch_idx) = self.app.current_batch {
                    let batches = ui::detect_batches_from_logs(&filtered_refs, self.app.batch_window_ms);
                    if !batches.is_empty() && batch_idx < batches.len() {
                        let (start, end) = batches[batch_idx];
                        end - start + 1
                    } else {
                        filtered_logs.len()
                    }
                } else {
                    filtered_logs.len()
                }
            } else {
                filtered_logs.len()
            };

            // Move selection down by a page (20 lines)
            let page_size = 20;
            if let Some(current_idx) = self.app.selected_line_index {
                let new_idx = (current_idx + page_size).min(total_logs.saturating_sub(1));
                self.app.selected_line_index = Some(new_idx);
                self.app.auto_scroll = false;
            }
        } else {
            // No line selected, just scroll the view
            let total_logs = self.manager.get_all_logs().len();
            let max_offset = total_logs.saturating_sub(1);
            self.app.scroll_down(20, max_offset);
        }
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
