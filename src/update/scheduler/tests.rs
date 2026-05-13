use super::auto_apply::is_in_maintenance_window;

#[test]
fn normal_window_inside() {
    let now = chrono::NaiveTime::from_hms_opt(3, 30, 0).unwrap();
    assert!(is_in_maintenance_window(now, "03:00", "05:00"));
}

#[test]
fn normal_window_outside_before() {
    let now = chrono::NaiveTime::from_hms_opt(2, 30, 0).unwrap();
    assert!(!is_in_maintenance_window(now, "03:00", "05:00"));
}

#[test]
fn normal_window_outside_after() {
    let now = chrono::NaiveTime::from_hms_opt(5, 30, 0).unwrap();
    assert!(!is_in_maintenance_window(now, "03:00", "05:00"));
}

#[test]
fn normal_window_at_start_boundary() {
    let now = chrono::NaiveTime::from_hms_opt(3, 0, 0).unwrap();
    assert!(is_in_maintenance_window(now, "03:00", "05:00"));
}

#[test]
fn normal_window_at_end_boundary() {
    let now = chrono::NaiveTime::from_hms_opt(5, 0, 0).unwrap();
    assert!(!is_in_maintenance_window(now, "03:00", "05:00"));
}

#[test]
fn wrapping_window_late_night() {
    let now = chrono::NaiveTime::from_hms_opt(23, 30, 0).unwrap();
    assert!(is_in_maintenance_window(now, "23:00", "02:00"));
}

#[test]
fn wrapping_window_early_morning() {
    let now = chrono::NaiveTime::from_hms_opt(1, 0, 0).unwrap();
    assert!(is_in_maintenance_window(now, "23:00", "02:00"));
}

#[test]
fn wrapping_window_outside() {
    let now = chrono::NaiveTime::from_hms_opt(12, 0, 0).unwrap();
    assert!(!is_in_maintenance_window(now, "23:00", "02:00"));
}

#[test]
fn wrapping_window_at_end_boundary() {
    let now = chrono::NaiveTime::from_hms_opt(2, 0, 0).unwrap();
    assert!(!is_in_maintenance_window(now, "23:00", "02:00"));
}

#[test]
fn invalid_window_times() {
    let now = chrono::NaiveTime::from_hms_opt(3, 0, 0).unwrap();
    assert!(!is_in_maintenance_window(now, "invalid", "05:00"));
    assert!(!is_in_maintenance_window(now, "03:00", "invalid"));
}
