use goose_tui::components::popups::navigate_list;

#[test]
fn navigate_list_returns_none_for_empty() {
    assert_eq!(navigate_list(None, 1, 0), None);
    assert_eq!(navigate_list(Some(0), 1, 0), None);
}

#[test]
fn navigate_list_single_item_stays_at_zero() {
    assert_eq!(navigate_list(Some(0), 1, 1), Some(0));
    assert_eq!(navigate_list(Some(0), -1, 1), Some(0));
    assert_eq!(navigate_list(Some(0), 5, 1), Some(0));
}

#[test]
fn navigate_list_wraps_at_boundaries() {
    assert_eq!(navigate_list(Some(2), 1, 3), Some(0));
    assert_eq!(navigate_list(Some(0), -1, 3), Some(2));
    assert_eq!(navigate_list(Some(3), 4, 5), Some(2));
    assert_eq!(navigate_list(Some(1), -4, 5), Some(2));
}

#[test]
fn navigate_list_from_none_starts_at_zero() {
    assert_eq!(navigate_list(None, 1, 3), Some(1));
    assert_eq!(navigate_list(None, -1, 3), Some(2));
    assert_eq!(navigate_list(None, 0, 5), Some(0));
}

#[test]
fn navigate_list_zero_delta_preserves_position() {
    assert_eq!(navigate_list(Some(2), 0, 5), Some(2));
}
