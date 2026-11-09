use super::{Menu, MenuItem};
use crate::actions::StandardAction;

#[test]
fn window_role_does_not_depend_on_the_display_title() {
    assert!(!Menu::new("Window", vec![]).is_window_menu());
    assert!(!Menu::new("窗口", vec![]).is_window_menu());

    let mut menu =
        Menu::new("窗口", vec![MenuItem::Standard(StandardAction::Minimize)]).with_window_role();
    assert!(menu.is_window_menu());

    menu.title = "Windows".to_owned();
    assert!(menu.is_window_menu());
    assert_eq!(menu.menu_items.len(), 1);
}
