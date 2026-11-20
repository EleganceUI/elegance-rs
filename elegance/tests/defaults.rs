// Default values that must match Elegance's C++ defaults (include/elg/*.h).

use elegance::desktop::TrayMenuItem;
use elegance::prelude::*;

#[test]
fn tray_menu_items_are_enabled_by_default() {
    // desktop.h: `bool enabled = true`.
    assert!(TrayMenuItem::default().enabled);
    assert!(TrayMenuItem { title: "Quit".into(), ..Default::default() }.enabled);
    assert!(TrayMenuItem::separator().enabled);
    assert!(TrayMenuItem::new("Open", || {}).enabled);
    assert!(!TrayMenuItem::new("Open", || {}).enabled(false).enabled);
}

#[test]
fn a_default_pose_is_the_view_in_place() {
    // animation.h: Transition::Pose has opacity 1, scale 1, no offset.
    let pose = Pose::default();
    assert_eq!((pose.opacity, pose.scale, pose.x, pose.y), (1.0, 1.0, 0.0, 0.0));
}

#[test]
fn events_default_to_a_single_click() {
    // events.h: `int click_count = 1` in PointerEvent and ClickEvent.
    assert_eq!(PointerEvent::default().click_count, 1);
    assert_eq!(ClickEvent::default().click_count, 1);
    assert_eq!(PointerEvent::default().kind, PointerEventKind::Move);
    assert_eq!(ClickEvent::default().button, MouseButton::Left);
}
