use crate::CellField;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionEndpoint {
    pub row: usize,
    pub track: usize,
    pub field: CellField,
}

impl SelectionEndpoint {
    #[must_use]
    pub const fn new(row: usize, track: usize, field: CellField) -> Self {
        Self { row, track, field }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionShape {
    Rectangle,
    FocusedField(CellField),
    WholeRows,
    WholeTracks,
    Pattern,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackerSelection {
    anchor: SelectionEndpoint,
    extent: SelectionEndpoint,
    shape: SelectionShape,
}

impl TrackerSelection {
    #[must_use]
    pub const fn rectangle(anchor: SelectionEndpoint, extent: SelectionEndpoint) -> Self {
        Self {
            anchor,
            extent,
            shape: SelectionShape::Rectangle,
        }
    }

    #[must_use]
    pub const fn focused_field(anchor: SelectionEndpoint, extent: SelectionEndpoint) -> Self {
        Self {
            anchor,
            extent,
            shape: SelectionShape::FocusedField(anchor.field),
        }
    }

    #[must_use]
    pub const fn whole_rows(anchor: SelectionEndpoint, extent: SelectionEndpoint) -> Self {
        Self {
            anchor,
            extent,
            shape: SelectionShape::WholeRows,
        }
    }

    #[must_use]
    pub const fn whole_tracks(anchor: SelectionEndpoint, extent: SelectionEndpoint) -> Self {
        Self {
            anchor,
            extent,
            shape: SelectionShape::WholeTracks,
        }
    }

    #[must_use]
    pub const fn pattern(anchor: SelectionEndpoint) -> Self {
        Self {
            anchor,
            extent: anchor,
            shape: SelectionShape::Pattern,
        }
    }

    #[must_use]
    pub const fn anchor(self) -> SelectionEndpoint {
        self.anchor
    }

    #[must_use]
    pub const fn extent(self) -> SelectionEndpoint {
        self.extent
    }

    #[must_use]
    pub const fn shape(self) -> SelectionShape {
        self.shape
    }

    #[must_use]
    pub const fn with_extent(mut self, extent: SelectionEndpoint) -> Self {
        self.extent = extent;
        self
    }

    #[must_use]
    pub fn bounds(self, row_count: usize, track_count: usize) -> Option<SelectionBounds> {
        if row_count == 0 || track_count == 0 {
            return None;
        }

        let anchor_row = self.anchor.row.min(row_count.saturating_sub(1));
        let extent_row = self.extent.row.min(row_count.saturating_sub(1));
        let anchor_track = self.anchor.track.min(track_count.saturating_sub(1));
        let extent_track = self.extent.track.min(track_count.saturating_sub(1));

        let (row_start, row_end) = minmax(anchor_row, extent_row);
        let (track_start, track_end) = minmax(anchor_track, extent_track);

        Some(match self.shape {
            SelectionShape::Rectangle => SelectionBounds {
                row_start,
                row_end,
                track_start,
                track_end,
                field: None,
            },
            SelectionShape::FocusedField(field) => SelectionBounds {
                row_start,
                row_end,
                track_start,
                track_end,
                field: Some(field),
            },
            SelectionShape::WholeRows => SelectionBounds {
                row_start,
                row_end,
                track_start: 0,
                track_end: track_count.saturating_sub(1),
                field: None,
            },
            SelectionShape::WholeTracks => SelectionBounds {
                row_start: 0,
                row_end: row_count.saturating_sub(1),
                track_start,
                track_end,
                field: None,
            },
            SelectionShape::Pattern => SelectionBounds {
                row_start: 0,
                row_end: row_count.saturating_sub(1),
                track_start: 0,
                track_end: track_count.saturating_sub(1),
                field: None,
            },
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionBounds {
    pub row_start: usize,
    pub row_end: usize,
    pub track_start: usize,
    pub track_end: usize,
    pub field: Option<CellField>,
}

impl SelectionBounds {
    #[must_use]
    pub const fn contains_cell(self, row: usize, track: usize) -> bool {
        self.row_start <= row
            && row <= self.row_end
            && self.track_start <= track
            && track <= self.track_end
    }

    #[must_use]
    pub const fn row_count(self) -> usize {
        self.row_end - self.row_start + 1
    }

    #[must_use]
    pub const fn track_count(self) -> usize {
        self.track_end - self.track_start + 1
    }
}

const fn minmax(a: usize, b: usize) -> (usize, usize) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint(row: usize, track: usize) -> SelectionEndpoint {
        SelectionEndpoint::new(row, track, CellField::Note)
    }

    #[test]
    fn rectangular_selection_normalizes_reversed_anchor_and_extent() {
        let selection = TrackerSelection::rectangle(endpoint(8, 3), endpoint(2, 1));

        assert_eq!(
            selection.bounds(16, 8),
            Some(SelectionBounds {
                row_start: 2,
                row_end: 8,
                track_start: 1,
                track_end: 3,
                field: None,
            })
        );
    }

    #[test]
    fn focused_field_selection_preserves_field_description() {
        let selection = TrackerSelection::focused_field(
            SelectionEndpoint::new(1, 0, CellField::Velocity),
            SelectionEndpoint::new(4, 2, CellField::Pan),
        );

        assert_eq!(
            selection.bounds(8, 4).map(|bounds| bounds.field),
            Some(Some(CellField::Velocity))
        );
    }

    #[test]
    fn whole_row_and_track_selections_expand_to_pattern_edges() {
        assert_eq!(
            TrackerSelection::whole_rows(endpoint(3, 2), endpoint(1, 0)).bounds(8, 4),
            Some(SelectionBounds {
                row_start: 1,
                row_end: 3,
                track_start: 0,
                track_end: 3,
                field: None,
            })
        );
        assert_eq!(
            TrackerSelection::whole_tracks(endpoint(3, 2), endpoint(1, 0)).bounds(8, 4),
            Some(SelectionBounds {
                row_start: 0,
                row_end: 7,
                track_start: 0,
                track_end: 2,
                field: None,
            })
        );
    }

    #[test]
    fn pattern_selection_and_resizing_clamp_to_current_edges() {
        let selection = TrackerSelection::pattern(endpoint(99, 99));

        assert_eq!(
            selection.bounds(4, 2),
            Some(SelectionBounds {
                row_start: 0,
                row_end: 3,
                track_start: 0,
                track_end: 1,
                field: None,
            })
        );
        assert_eq!(selection.bounds(0, 2), None);
    }
}
