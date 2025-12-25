use ratatui::layout::Rect;

/// Mouse click regions for UI interaction
#[derive(Debug, Default)]
pub struct ClickRegions {
    /// Area of the process list widget
    pub process_list_area: Option<Rect>,
    /// Area of the log viewer widget
    pub log_viewer_area: Option<Rect>,
    /// Area of the status bar
    pub status_bar_area: Option<Rect>,
    /// Clickable regions for each process name (name, bounding rect)
    pub process_regions: Vec<(String, Rect)>,
}

impl ClickRegions {
    pub fn new() -> Self {
        Self::default()
    }
}
