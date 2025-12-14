use goose_tui::components::popups::navigate_list;

// ============================================================================
// navigate_list tests
// ============================================================================

#[test]
fn navigate_list_empty_returns_none() {
    let result = navigate_list(None, 1, 0);
    assert_eq!(result, None);

    let result = navigate_list(Some(0), 1, 0);
    assert_eq!(result, None);
}

#[test]
fn navigate_list_single_item_stays() {
    // With only one item, any navigation should stay at index 0
    let result = navigate_list(Some(0), 1, 1);
    assert_eq!(result, Some(0));

    let result = navigate_list(Some(0), -1, 1);
    assert_eq!(result, Some(0));

    let result = navigate_list(Some(0), 5, 1);
    assert_eq!(result, Some(0));
}

#[test]
fn navigate_list_wraps_forward() {
    // At last item (index 2), moving forward should wrap to 0
    let result = navigate_list(Some(2), 1, 3);
    assert_eq!(result, Some(0));
}

#[test]
fn navigate_list_wraps_backward() {
    // At first item (index 0), moving backward should wrap to last
    let result = navigate_list(Some(0), -1, 3);
    assert_eq!(result, Some(2));
}

#[test]
fn navigate_list_none_starts_at_zero_then_moves() {
    // When current is None, we start at 0 and then apply delta
    // (0 + 1) % 3 = 1
    let result = navigate_list(None, 1, 3);
    assert_eq!(result, Some(1));

    // (0 + -1) % 3 = 2 (using rem_euclid for proper modulo)
    let result = navigate_list(None, -1, 3);
    assert_eq!(result, Some(2));
}

#[test]
fn navigate_list_normal_forward() {
    let result = navigate_list(Some(0), 1, 5);
    assert_eq!(result, Some(1));

    let result = navigate_list(Some(1), 1, 5);
    assert_eq!(result, Some(2));

    let result = navigate_list(Some(3), 1, 5);
    assert_eq!(result, Some(4));
}

#[test]
fn navigate_list_normal_backward() {
    let result = navigate_list(Some(4), -1, 5);
    assert_eq!(result, Some(3));

    let result = navigate_list(Some(2), -1, 5);
    assert_eq!(result, Some(1));
}

#[test]
fn navigate_list_large_delta_forward() {
    // Jump multiple items forward
    let result = navigate_list(Some(0), 3, 5);
    assert_eq!(result, Some(3));

    // Jump past end, should wrap
    let result = navigate_list(Some(3), 4, 5);
    assert_eq!(result, Some(2)); // (3 + 4) % 5 = 2
}

#[test]
fn navigate_list_large_delta_backward() {
    // Jump multiple items backward
    let result = navigate_list(Some(4), -3, 5);
    assert_eq!(result, Some(1)); // (4 - 3) % 5 = 1

    // Jump past beginning, should wrap
    let result = navigate_list(Some(1), -4, 5);
    assert_eq!(result, Some(2)); // (1 - 4) % 5 = -3 -> 2 with rem_euclid
}

#[test]
fn navigate_list_zero_delta() {
    let result = navigate_list(Some(2), 0, 5);
    assert_eq!(result, Some(2));

    let result = navigate_list(None, 0, 5);
    assert_eq!(result, Some(0));
}
