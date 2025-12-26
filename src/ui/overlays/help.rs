use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::ui::utils::centered_rect;

/// Draw the help overlay with scroll support
pub fn draw_help_overlay(f: &mut Frame, scroll_offset: u16) {
    let help_text = vec![
        Line::from(vec![
            Span::styled("Overitall Help", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Navigation:", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  ↑/↓", Style::default().fg(Color::Yellow)),
            Span::raw("     Select previous/next log line"),
        ]),
        Line::from(vec![
            Span::styled("  Shift+↑/↓", Style::default().fg(Color::Yellow)),
            Span::raw(" Extend selection (multi-select)"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+B/F", Style::default().fg(Color::Yellow)),
            Span::raw(" Page up/down (Vim-style)"),
        ]),
        Line::from(vec![
            Span::styled("  Enter", Style::default().fg(Color::Yellow)),
            Span::raw("   Expand selected line (show full content)"),
        ]),
        Line::from(vec![
            Span::styled("  Esc", Style::default().fg(Color::Yellow)),
            Span::raw("     Jump to latest logs (reset view)"),
        ]),
        Line::from(vec![
            Span::styled("  q", Style::default().fg(Color::Yellow)),
            Span::raw("       Quit"),
        ]),
        Line::from(vec![
            Span::styled("  s", Style::default().fg(Color::Yellow)),
            Span::raw("       Start/stop manual trace capture"),
        ]),
        Line::from(vec![
            Span::styled("  w", Style::default().fg(Color::Yellow)),
            Span::raw("       Cycle display: compact → full → wrap"),
        ]),
        Line::from(vec![
            Span::styled("  t", Style::default().fg(Color::Yellow)),
            Span::raw("       Cycle timestamps: seconds → ms → off"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Commands:", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  :", Style::default().fg(Color::Yellow)),
            Span::raw("       Enter command mode"),
        ]),
        Line::from(vec![
            Span::styled("  :s <proc>", Style::default().fg(Color::Yellow)),
            Span::raw(" Start process"),
        ]),
        Line::from(vec![
            Span::styled("  :r [proc]", Style::default().fg(Color::Yellow)),
            Span::raw(" Restart process (or all if no arg)"),
        ]),
        Line::from(vec![
            Span::styled("  :k <proc>", Style::default().fg(Color::Yellow)),
            Span::raw(" Kill process"),
        ]),
        Line::from(vec![
            Span::styled("  :q/:quit/:exit", Style::default().fg(Color::Yellow)),
            Span::raw("  Quit"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Filtering:", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  :f <pat>", Style::default().fg(Color::Yellow)),
            Span::raw("  Include filter (show only matching lines)"),
        ]),
        Line::from(vec![
            Span::styled("  :fn <pat>", Style::default().fg(Color::Yellow)),
            Span::raw(" Exclude filter (hide matching lines)"),
        ]),
        Line::from(vec![
            Span::styled("  :fc", Style::default().fg(Color::Yellow)),
            Span::raw("       Clear all filters"),
        ]),
        Line::from(vec![
            Span::styled("  :fl", Style::default().fg(Color::Yellow)),
            Span::raw("       List active filters"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Search:", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  /", Style::default().fg(Color::Yellow)),
            Span::raw("       Start search (filters as you type)"),
        ]),
        Line::from(vec![
            Span::styled("  Enter", Style::default().fg(Color::Yellow)),
            Span::raw("   In search mode: enter selection mode"),
        ]),
        Line::from(vec![
            Span::styled("  Enter", Style::default().fg(Color::Yellow)),
            Span::raw("   In expanded view: show context around log"),
        ]),
        Line::from(vec![
            Span::styled("  Esc", Style::default().fg(Color::Yellow)),
            Span::raw("     Step back (selection→typing→exit)"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Batch Navigation:", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  [", Style::default().fg(Color::Yellow)),
            Span::raw("       Previous batch"),
        ]),
        Line::from(vec![
            Span::styled("  ]", Style::default().fg(Color::Yellow)),
            Span::raw("       Next batch"),
        ]),
        Line::from(vec![
            Span::styled("  :pb", Style::default().fg(Color::Yellow)),
            Span::raw("      Previous batch (same as [)"),
        ]),
        Line::from(vec![
            Span::styled("  :nb", Style::default().fg(Color::Yellow)),
            Span::raw("      Next batch (same as ])"),
        ]),
        Line::from(vec![
            Span::styled("  :sb", Style::default().fg(Color::Yellow)),
            Span::raw("      Toggle batch view mode"),
        ]),
        Line::from(vec![
            Span::styled("  :bw", Style::default().fg(Color::Yellow)),
            Span::raw("       Show current batch window"),
        ]),
        Line::from(vec![
            Span::styled("  :bw <ms>", Style::default().fg(Color::Yellow)),
            Span::raw("  Set batch window (milliseconds)"),
        ]),
        Line::from(vec![
            Span::styled("  :bw fast/medium/slow", Style::default().fg(Color::Yellow)),
            Span::raw("  Presets: 100ms/1000ms/5000ms"),
        ]),
        Line::from(vec![
            Span::styled("  +/-", Style::default().fg(Color::Yellow)),
            Span::raw("     Increase/decrease batch window by 100ms"),
        ]),
        Line::from(vec![
            Span::styled("  :g/:goto <time>", Style::default().fg(Color::Yellow)),
            Span::raw("  Jump to time (HH:MM, -5m, +30s)"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Clipboard & Batch:", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  c", Style::default().fg(Color::Yellow)),
            Span::raw("       Copy selected line(s) (also in expanded view)"),
        ]),
        Line::from(vec![
            Span::styled("  Shift+C", Style::default().fg(Color::Yellow)),
            Span::raw(" Copy entire batch to clipboard"),
        ]),
        Line::from(vec![
            Span::styled("  x", Style::default().fg(Color::Yellow)),
            Span::raw("       Contextual copy (same process ±1s)"),
        ]),
        Line::from(vec![
            Span::styled("  b", Style::default().fg(Color::Yellow)),
            Span::raw("       Focus on batch containing selected line"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Process Visibility:", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  :hide <proc>", Style::default().fg(Color::Yellow)),
            Span::raw(" Hide logs from a specific process"),
        ]),
        Line::from(vec![
            Span::styled("  :show <proc>", Style::default().fg(Color::Yellow)),
            Span::raw(" Show logs from a specific process"),
        ]),
        Line::from(vec![
            Span::styled("  :hide all", Style::default().fg(Color::Yellow)),
            Span::raw("    Hide all process logs"),
        ]),
        Line::from(vec![
            Span::styled("  :show all", Style::default().fg(Color::Yellow)),
            Span::raw("    Show all process logs"),
        ]),
        Line::from(vec![
            Span::styled("  :only <proc>", Style::default().fg(Color::Yellow)),
            Span::raw(" Show only one process, hide all others"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Display:", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  :color", Style::default().fg(Color::Yellow)),
            Span::raw("      Toggle process coloring on/off"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Trace Detection:", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  :traces", Style::default().fg(Color::Yellow)),
            Span::raw("     Detect correlation IDs (UUIDs, etc.)"),
        ]),
        Line::from(vec![
            Span::styled("  [ ]", Style::default().fg(Color::Yellow)),
            Span::raw("       Expand trace view (before/after)"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("↑/↓", Style::default().fg(Color::Yellow)),
            Span::styled(" scroll | ", Style::default()),
            Span::styled("ESC", Style::default().fg(Color::Yellow)),
            Span::styled(" or ", Style::default()),
            Span::styled("?", Style::default().fg(Color::Yellow)),
            Span::styled(" to close", Style::default()),
        ]),
    ];

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .wrap(Wrap { trim: true })
        .scroll((scroll_offset, 0));

    let area = centered_rect(60, 80, f.area());

    // Clear the area behind the popup
    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}
