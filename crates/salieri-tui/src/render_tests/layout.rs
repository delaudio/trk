use super::*;

#[test]
fn classifies_responsive_layout_breakpoints() {
    assert_eq!(layout_kind(79), LayoutKind::Small);
    assert_eq!(layout_kind(80), LayoutKind::Medium);
    assert_eq!(layout_kind(119), LayoutKind::Medium);
    assert_eq!(layout_kind(120), LayoutKind::Large);
}
