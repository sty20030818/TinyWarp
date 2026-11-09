use warp_core::ui::appearance::Appearance;
use warpui::platform::WindowStyle;
use warpui::{App, TypedActionView};

use super::{Menu, MenuAction, MenuItem, MenuItemFields, SelectAction, SubMenu};

#[derive(Clone, Debug, PartialEq, Eq)]
enum TestAction {
    Root,
    ChildOne,
    ChildTwo,
}

fn test_submenu_items() -> Vec<MenuItem<TestAction>> {
    vec![
        MenuItem::Submenu {
            fields: MenuItemFields::new_submenu("submenu"),
            menu: SubMenu::new(vec![
                MenuItemFields::new("child one")
                    .with_on_select_action(TestAction::ChildOne)
                    .into_item(),
                MenuItemFields::new("child two")
                    .with_on_select_action(TestAction::ChildTwo)
                    .into_item(),
            ]),
        },
        MenuItemFields::new("root")
            .with_on_select_action(TestAction::Root)
            .into_item(),
    ]
}

#[test]
fn test_menu_item_selectable() {
    assert!(MenuItemFields::<()>::new("normal").into_item().selectable());
    assert!(
        !MenuItemFields::<()>::new("disabled")
            .with_disabled(true)
            .into_item()
            .selectable()
    );
    assert!(!MenuItem::<()>::Separator.selectable());
}

#[test]
fn test_next_and_previous_indexes() {
    App::test((), |mut app| async move {
        app.add_singleton_model(|_| Appearance::mock());

        let items = vec![
            MenuItemFields::<()>::new("item1")
                .with_disabled(true)
                .into_item(),
            MenuItemFields::<()>::new("item2").into_item(),
            MenuItemFields::<()>::new("item3")
                .with_disabled(true)
                .into_item(),
            MenuItemFields::<()>::new("item4").into_item(),
            MenuItemFields::<()>::new("item5").into_item(),
        ];

        let (_, menu) = app.add_window(WindowStyle::NotStealFocus, |ctx| {
            let mut menu = Menu::<()>::new();
            menu.set_items(items, ctx);
            menu
        });

        menu.update(&mut app, |menu, _ctx| {
            assert!(menu.selected_item().is_none());

            menu.menu
                .select_internal(SelectAction::Index { row: 1, item: 0 });
            assert!(menu.selected_item().is_some());
            assert_eq!(
                menu.selected_item().unwrap().fields().unwrap().label(),
                "item2"
            );

            // Make sure we skip the disabled menu items
            menu.menu.select_internal(SelectAction::Next);
            assert!(menu.selected_item().is_some());
            assert_eq!(
                menu.selected_item().unwrap().fields().unwrap().label(),
                "item4"
            );

            menu.menu.select_internal(SelectAction::Next);
            assert!(menu.selected_item().is_some());
            assert_eq!(
                menu.selected_item().unwrap().fields().unwrap().label(),
                "item5"
            );

            // Make sure we go around
            menu.menu.select_internal(SelectAction::Next);
            assert!(menu.selected_item().is_some());
            assert_eq!(
                menu.selected_item().unwrap().fields().unwrap().label(),
                "item2"
            );

            // Makre sure we go around with Prev action too
            menu.menu.select_internal(SelectAction::Previous);
            assert!(menu.selected_item().is_some());
            assert_eq!(
                menu.selected_item().unwrap().fields().unwrap().label(),
                "item5"
            );

            menu.menu.select_internal(SelectAction::Previous);
            assert!(menu.selected_item().is_some());
            assert_eq!(
                menu.selected_item().unwrap().fields().unwrap().label(),
                "item4"
            );

            // Makre sure we skip the disabled ones for previous as well
            menu.menu.select_internal(SelectAction::Previous);
            assert!(menu.selected_item().is_some());
            assert_eq!(
                menu.selected_item().unwrap().fields().unwrap().label(),
                "item2"
            );
        });
    })
}

#[test]
fn test_right_opens_selected_submenu_and_selects_first_child() {
    App::test((), |mut app| async move {
        app.add_singleton_model(|_| Appearance::mock());

        let (_, menu) = app.add_window(WindowStyle::NotStealFocus, |ctx| {
            let mut menu = Menu::<TestAction>::new();
            menu.set_items(test_submenu_items(), ctx);
            menu
        });

        menu.update(&mut app, |menu, ctx| {
            menu.set_selected_by_index(0, ctx);
            menu.handle_action(&MenuAction::OpenSubmenu, ctx);

            assert_eq!(menu.selected_index(), Some(0));
            let submenu = menu.menu.selected_submenu().unwrap();
            assert_eq!(submenu.selected_index(), Some(0));
            assert_eq!(
                submenu.selected_item().unwrap().fields().unwrap().label(),
                "child one"
            );
        });
    })
}

#[test]
fn test_up_and_down_navigate_the_active_submenu() {
    App::test((), |mut app| async move {
        app.add_singleton_model(|_| Appearance::mock());

        let (_, menu) = app.add_window(WindowStyle::NotStealFocus, |ctx| {
            let mut menu = Menu::<TestAction>::new();
            menu.set_items(test_submenu_items(), ctx);
            menu
        });

        menu.update(&mut app, |menu, ctx| {
            menu.set_selected_by_index(0, ctx);
            menu.handle_action(&MenuAction::OpenSubmenu, ctx);
            menu.handle_action(&MenuAction::Select(SelectAction::Next), ctx);

            let submenu = menu.menu.selected_submenu().unwrap();
            assert_eq!(submenu.selected_index(), Some(1));
            assert_eq!(
                submenu.selected_item().unwrap().fields().unwrap().label(),
                "child two"
            );

            menu.handle_action(&MenuAction::Select(SelectAction::Previous), ctx);

            let submenu = menu.menu.selected_submenu().unwrap();
            assert_eq!(submenu.selected_index(), Some(0));
            assert_eq!(
                submenu.selected_item().unwrap().fields().unwrap().label(),
                "child one"
            );
        });
    })
}

#[test]
fn test_enter_uses_the_active_submenu_selection() {
    App::test((), |mut app| async move {
        app.add_singleton_model(|_| Appearance::mock());

        let (_, menu) = app.add_window(WindowStyle::NotStealFocus, |ctx| {
            let mut menu = Menu::<TestAction>::new();
            menu.set_items(test_submenu_items(), ctx);
            menu
        });

        let mut selected_action = None;
        menu.update(&mut app, |menu, ctx| {
            menu.set_selected_by_index(0, ctx);
            menu.handle_action(&MenuAction::OpenSubmenu, ctx);
            menu.handle_action(&MenuAction::Select(SelectAction::Next), ctx);

            selected_action = menu.menu.selected_action_for_enter(ctx);
        });

        assert_eq!(selected_action, Some(TestAction::ChildTwo));
    })
}

#[test]
fn test_right_is_a_noop_for_leaf_items() {
    App::test((), |mut app| async move {
        app.add_singleton_model(|_| Appearance::mock());

        let (_, menu) = app.add_window(WindowStyle::NotStealFocus, |ctx| {
            let mut menu = Menu::<TestAction>::new();
            menu.set_items(test_submenu_items(), ctx);
            menu
        });

        menu.update(&mut app, |menu, ctx| {
            menu.set_selected_by_index(1, ctx);
            menu.handle_action(&MenuAction::OpenSubmenu, ctx);

            assert_eq!(menu.selected_index(), Some(1));
            assert!(menu.menu.selected_submenu().is_none());
            assert_eq!(
                menu.selected_item().unwrap().fields().unwrap().label(),
                "root"
            );
        });
    })
}

#[test]
fn localized_caption_preserves_selection_identity_and_accessibility() {
    App::test((), |mut app| async move {
        app.add_singleton_model(|_| Appearance::mock());
        let (_, menu) = app.add_window(WindowStyle::NotStealFocus, |ctx| {
            let mut menu = Menu::<TestAction>::new();
            menu.set_items(
                vec![
                    MenuItemFields::new("New tab config")
                        .with_display_label("新建标签配置")
                        .with_on_select_action(TestAction::Root)
                        .into_item(),
                    MenuItemFields::new("Terminal").into_item(),
                ],
                ctx,
            );
            menu
        });
        menu.update(&mut app, |menu, ctx| {
            assert!(menu.set_selected_by_name("New tab config", ctx));
            let selected = menu.selected_item().unwrap();
            let fields = selected.fields().unwrap();
            assert_eq!(fields.label(), "New tab config");
            assert_eq!(fields.get_a11y_text(), "新建标签配置");
            assert!(menu.set_selected_by_name("Terminal", ctx));
            assert_eq!(
                menu.selected_item()
                    .unwrap()
                    .fields()
                    .unwrap()
                    .get_a11y_text(),
                "Terminal"
            );
        });
    });
}

#[test]
fn localized_caption_identity_survives_action_type_projection() {
    let item = MenuItemFields::<()>::new_submenu("Move to group")
        .with_display_label("移到分组")
        .with_different_on_select_action_type(Some(TestAction::ChildOne));
    assert_eq!(item.label(), "Move to group");
    assert_eq!(item.get_a11y_text(), "移到分组");
    assert!(item.into_item().is_approximately_same_item_as(
        &MenuItemFields::<TestAction>::new("Move to group").into_item(),
    ));
}

#[test]
fn local_terminal_menu_keeps_local_identity_custom_labels_and_unknown_actions() {
    use std::sync::Arc;

    use warpui::Element as _;
    use warpui::elements::Empty;

    use super::MenuItemLabel;
    use crate::workspace::WorkspaceAction;

    #[derive(Clone, Debug, PartialEq)]
    struct UnknownAction(u8);

    App::test((), |mut app| async move {
        app.update(crate::local_terminal::enable_for_test);
        app.add_singleton_model(|_| Appearance::mock());
        let (_, menu) = app.add_window(WindowStyle::NotStealFocus, |ctx| {
            let mut menu = Menu::<WorkspaceAction>::new();
            menu.set_items(
                vec![
                    MenuItemFields::new("Agent")
                        .with_on_select_action(WorkspaceAction::AddAgentTab)
                        .into_item(),
                    MenuItem::Separator,
                    MenuItemFields::new("stable-local-id")
                        .with_display_label("本地设置")
                        .with_on_select_action(WorkspaceAction::ShowSettings)
                        .into_item(),
                    MenuItem::Submenu {
                        fields: MenuItemFields::new_submenu("empty-ai-submenu"),
                        menu: SubMenu::new(vec![
                            MenuItemFields::new("hidden")
                                .with_on_select_action(WorkspaceAction::AddAgentTab)
                                .into_item(),
                        ]),
                    },
                    MenuItem::ItemsRow {
                        items: vec![
                            MenuItemFields::new("hidden-ai-row-item")
                                .with_on_select_action(WorkspaceAction::AddAgentTab),
                            MenuItemFields::new("local-row-item")
                                .with_on_select_action(WorkspaceAction::ShowSettings),
                        ],
                    },
                    MenuItemFields::new_with_custom_label(
                        Arc::new(|_, _, _, _| Empty::new().finish()),
                        Some("用户的 Custom 标签 {保留}".to_owned()),
                    )
                    .with_on_select_action(WorkspaceAction::ShowSettings)
                    .into_item(),
                    MenuItem::Separator,
                ],
                ctx,
            );
            menu
        });
        menu.update(&mut app, |menu, ctx| {
            assert_eq!(menu.items().len(), 3);
            assert!(menu.set_selected_by_name("stable-local-id", ctx));
            let selected = menu.selected_item().unwrap();
            let fields = selected.fields().unwrap();
            assert_eq!(fields.label(), "stable-local-id");
            assert_eq!(fields.get_a11y_text(), "本地设置");
            assert!(matches!(
                fields.on_select_action(),
                Some(WorkspaceAction::ShowSettings)
            ));
            let MenuItem::ItemsRow { items } = &menu.items()[1] else {
                panic!("本地 row 应保留")
            };
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].label(), "local-row-item");
            let fields = menu.items()[2].fields().unwrap();
            assert!(matches!(&fields.element, MenuItemLabel::Custom { .. }));
            assert_eq!(fields.label(), "用户的 Custom 标签 {保留}");
        });

        let (_, unknown_menu) = app.add_window(WindowStyle::NotStealFocus, |ctx| {
            let mut menu = Menu::<UnknownAction>::new();
            menu.set_items(
                vec![
                    MenuItemFields::new("Agent 用户名称并非内置能力")
                        .with_on_select_action(UnknownAction(7))
                        .into_item(),
                ],
                ctx,
            );
            menu
        });
        unknown_menu.update(&mut app, |menu, ctx| {
            assert_eq!(menu.items().len(), 1);
            assert!(menu.set_selected_by_name("Agent 用户名称并非内置能力", ctx));
            assert_eq!(
                menu.selected_item()
                    .unwrap()
                    .fields()
                    .unwrap()
                    .on_select_action(),
                Some(&UnknownAction(7))
            );
        });
    });
}
