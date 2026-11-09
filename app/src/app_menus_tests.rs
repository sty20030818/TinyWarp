use warpui::keymap::Keystroke;
use warpui::platform::menu::MenuItem;

use super::{dock_menu, make_new_window_menu};

#[test]
fn translated_window_menu_keeps_its_native_role() {
    let menu = make_new_window_menu();
    assert_eq!(menu.title, "窗口");
    assert!(menu.is_window_menu());
    assert_eq!(menu.menu_items.len(), 5);
}

#[test]
fn translated_dock_item_keeps_new_window_shortcut() {
    let menu = dock_menu();
    let MenuItem::Custom(item) = &menu.menu_items[0] else {
        panic!("新建窗口必须保留自定义菜单动作");
    };
    assert_eq!(item.properties.name, "新建窗口");
    assert_eq!(
        item.properties.keystroke,
        Some(Keystroke::parse("cmd-n").expect("有效快捷键")),
    );
}
