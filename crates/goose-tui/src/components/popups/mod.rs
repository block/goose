pub mod builder;
pub mod config;
pub mod help;
pub mod message;
pub mod session;
pub mod theme;
pub mod todo;

use crate::utils::styles::Theme;
use ratatui::layout::{Alignment, Margin, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use ratatui::Frame;
use std::time::{Duration, Instant};

/// Shared scroll state for popups with scrollable content
#[derive(Default)]
pub struct PopupScrollState {
    pub scroll: u16,
    pub content_height: u16,
    pub viewport_height: u16,
    last_scroll_time: Option<Instant>,
}

impl PopupScrollState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.scroll = 0;
        self.last_scroll_time = None;
    }

    fn max_scroll(&self) -> u16 {
        self.content_height.saturating_sub(self.viewport_height)
    }

    pub fn scroll_by(&mut self, delta: i16) {
        if delta > 0 {
            self.scroll = self
                .scroll
                .saturating_add(delta as u16)
                .min(self.max_scroll());
        } else {
            self.scroll = self.scroll.saturating_sub((-delta) as u16);
        }
        self.last_scroll_time = Some(Instant::now());
    }

    pub fn clamp(&mut self) {
        if self.scroll > self.max_scroll() {
            self.scroll = self.max_scroll();
        }
    }

    /// Render a scrollbar that appears briefly after scrolling
    pub fn render_transient_scrollbar(&self, f: &mut Frame, area: Rect) {
        const SCROLLBAR_VISIBLE_DURATION: Duration = Duration::from_secs(1);

        if let Some(last) = self.last_scroll_time {
            if last.elapsed() < SCROLLBAR_VISIBLE_DURATION
                && self.content_height > self.viewport_height
            {
                let mut scrollbar_state = ScrollbarState::default()
                    .content_length(self.content_height as usize)
                    .viewport_content_length(self.viewport_height as usize)
                    .position(self.scroll as usize);

                f.render_stateful_widget(
                    Scrollbar::new(ScrollbarOrientation::VerticalRight)
                        .begin_symbol(Some("↑"))
                        .end_symbol(Some("↓")),
                    area,
                    &mut scrollbar_state,
                );
            }
        }
    }
}

/// Create a standard popup block with rounded borders and theme styling
pub fn popup_block(title: &str, theme: &Theme) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.base.border))
        .title(title.to_string())
        .style(Style::default().bg(theme.base.background))
}

/// Render a scrollbar on the right side of an area
pub fn render_scrollbar(f: &mut Frame, area: Rect, state: &mut ScrollbarState) {
    f.render_stateful_widget(
        Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓")),
        area.inner(Margin::new(0, 1)),
        state,
    );
}

/// Render keyboard hints at the bottom of a popup
pub fn render_hints(f: &mut Frame, area: Rect, theme: &Theme, hints: &[(&str, &str)]) {
    let spans: Vec<Span> = hints
        .iter()
        .enumerate()
        .flat_map(|(i, (key, desc))| {
            let mut v = vec![];
            if i > 0 {
                v.push(Span::raw("  "));
            }
            v.push(Span::styled(
                (*key).to_string(),
                Style::default().fg(theme.status.info),
            ));
            v.push(Span::styled(
                format!(" {desc}"),
                Style::default().fg(theme.base.foreground),
            ));
            v
        })
        .collect();
    f.render_widget(
        Paragraph::new(Line::from(spans)).alignment(Alignment::Center),
        area,
    );
}

/// Navigate within a list, wrapping around at boundaries
pub fn navigate_list(current: Option<usize>, delta: i32, count: usize) -> Option<usize> {
    if count == 0 {
        return None;
    }
    let cur = current.unwrap_or(0) as i32;
    let next = (cur + delta).rem_euclid(count as i32) as usize;
    Some(next)
}
