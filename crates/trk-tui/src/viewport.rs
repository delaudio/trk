use std::ops::Range;

use ratatui::widgets::ScrollbarState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverscrollPolicy {
    None,
    Trailing(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewportAxis {
    content_len: usize,
    viewport_len: usize,
    offset: usize,
    overscroll: OverscrollPolicy,
}

impl ViewportAxis {
    #[must_use]
    pub const fn new(content_len: usize, viewport_len: usize) -> Self {
        Self {
            content_len,
            viewport_len,
            offset: 0,
            overscroll: OverscrollPolicy::None,
        }
    }

    #[must_use]
    pub const fn with_offset(content_len: usize, viewport_len: usize, offset: usize) -> Self {
        Self {
            content_len,
            viewport_len,
            offset,
            overscroll: OverscrollPolicy::None,
        }
    }

    #[must_use]
    pub const fn with_overscroll(mut self, overscroll: OverscrollPolicy) -> Self {
        self.overscroll = overscroll;
        self
    }

    #[must_use]
    pub const fn content_len(self) -> usize {
        self.content_len
    }

    #[must_use]
    pub const fn viewport_len(self) -> usize {
        self.viewport_len
    }

    #[must_use]
    pub const fn offset(self) -> usize {
        self.offset
    }

    #[must_use]
    pub fn max_offset(self) -> usize {
        let trailing = match self.overscroll {
            OverscrollPolicy::None => 0,
            OverscrollPolicy::Trailing(amount) => amount,
        };
        self.content_len
            .saturating_add(trailing)
            .saturating_sub(self.viewport_len.max(1))
    }

    pub fn set_lengths(&mut self, content_len: usize, viewport_len: usize) {
        self.content_len = content_len;
        self.viewport_len = viewport_len;
        self.clamp();
    }

    pub fn set_offset(&mut self, offset: usize) {
        self.offset = offset.min(self.max_offset());
    }

    pub fn resize(&mut self, viewport_len: usize) {
        self.viewport_len = viewport_len;
        self.clamp();
    }

    pub fn clamp(&mut self) {
        self.offset = self.offset.min(self.max_offset());
    }

    pub fn keep_visible(&mut self, index: usize) {
        if self.viewport_len == 0 || self.content_len == 0 {
            self.offset = 0;
            return;
        }

        let index = index.min(self.content_len.saturating_sub(1));
        if index < self.offset {
            self.offset = index;
        } else if index >= self.offset.saturating_add(self.viewport_len) {
            self.offset = index.saturating_sub(self.viewport_len - 1);
        }
        self.clamp();
    }

    pub fn scroll_by(&mut self, delta: isize) {
        self.offset = self
            .offset
            .saturating_add_signed(delta)
            .min(self.max_offset());
    }

    pub fn page_up(&mut self) {
        self.scroll_by(-(self.viewport_len.max(1) as isize));
    }

    pub fn page_down(&mut self) {
        self.scroll_by(self.viewport_len.max(1) as isize);
    }

    pub fn home(&mut self) {
        self.offset = 0;
    }

    pub fn end(&mut self) {
        self.offset = self.max_offset();
    }

    pub fn jump_to(&mut self, offset: usize) {
        self.set_offset(offset);
    }

    #[must_use]
    pub fn visible_range(self) -> Range<usize> {
        let start = self.offset.min(self.content_len);
        let end = start
            .saturating_add(self.viewport_len)
            .min(self.content_len);
        start..end
    }

    #[must_use]
    pub fn scrollbar_state(self) -> ScrollbarState {
        ScrollbarState::new(self.content_len).position(self.offset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport2D {
    pub rows: ViewportAxis,
    pub columns: ViewportAxis,
}

impl Viewport2D {
    #[must_use]
    pub const fn new(
        row_count: usize,
        visible_rows: usize,
        column_count: usize,
        visible_columns: usize,
    ) -> Self {
        Self {
            rows: ViewportAxis::new(row_count, visible_rows),
            columns: ViewportAxis::new(column_count, visible_columns),
        }
    }

    pub fn keep_visible(&mut self, row: usize, column: usize) {
        self.rows.keep_visible(row);
        self.columns.keep_visible(column);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keep_visible_scrolls_down_to_include_cursor() {
        let mut viewport = ViewportAxis::new(64, 10);

        viewport.keep_visible(20);

        assert_eq!(viewport.offset(), 11);
        assert_eq!(viewport.visible_range(), 11..21);
    }

    #[test]
    fn keep_visible_scrolls_up_to_include_cursor() {
        let mut viewport = ViewportAxis::with_offset(64, 10, 20);

        viewport.keep_visible(5);

        assert_eq!(viewport.offset(), 5);
    }

    #[test]
    fn resize_clamps_offset_near_content_end() {
        let mut viewport = ViewportAxis::with_offset(64, 10, 60);

        viewport.resize(20);

        assert_eq!(viewport.offset(), 44);
        assert_eq!(viewport.visible_range(), 44..64);
    }

    #[test]
    fn boundary_navigation_stays_within_content() {
        let mut viewport = ViewportAxis::new(30, 8);

        viewport.page_down();
        viewport.page_down();
        viewport.page_down();
        viewport.end();

        assert_eq!(viewport.offset(), 22);

        viewport.page_up();
        assert_eq!(viewport.offset(), 14);

        viewport.home();
        viewport.page_up();
        assert_eq!(viewport.offset(), 0);
    }

    #[test]
    fn jump_to_clamps_to_maximum_offset() {
        let mut viewport = ViewportAxis::new(12, 5);

        viewport.jump_to(100);

        assert_eq!(viewport.offset(), 7);
    }

    #[test]
    fn two_dimensional_viewport_keeps_axes_independent() {
        let mut viewport = Viewport2D::new(64, 12, 16, 4);

        viewport.keep_visible(20, 7);

        assert_eq!(viewport.rows.offset(), 9);
        assert_eq!(viewport.columns.offset(), 4);
        assert_eq!(viewport.rows.visible_range(), 9..21);
        assert_eq!(viewport.columns.visible_range(), 4..8);
    }
}
