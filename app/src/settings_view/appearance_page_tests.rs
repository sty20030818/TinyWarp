use super::*;

#[test]
fn appearance_search_keeps_english_keywords_and_adds_chinese() {
    let theme_terms = CreateCustomThemeWidget::default();
    let font_terms = TerminalFontWidget::default();
    let opacity_terms = WindowOpacityWidget::default();
    for (terms, english, chinese) in [
        (theme_terms.search_terms(), "custom theme", "主题"),
        (font_terms.search_terms(), "font", "字体"),
        (opacity_terms.search_terms(), "opacity", "透明度"),
    ] {
        assert!(super::super::settings_page::search_terms_match(
            terms, english
        ));
        assert!(super::super::settings_page::search_terms_match(
            terms, chinese
        ));
    }
}

#[test]
fn menu_icon_search_is_independent_from_dock_icon_settings() {
    let menu = MenuIconWidget::default();
    let dock = CustomAppIconWidget::default();
    for query in ["menu icon", "菜单 图标", "PNG", "恢复 默认"] {
        assert!(super::super::settings_page::search_terms_match(
            menu.search_terms(),
            query
        ));
    }
    assert!(!super::super::settings_page::search_terms_match(
        dock.search_terms(),
        "菜单"
    ));
    assert!(!super::super::settings_page::search_terms_match(
        menu.search_terms(),
        "dock"
    ));
}

#[test]
fn menu_icon_commands_are_editable_and_only_registered_for_local_terminal() {
    for local in [false, true] {
        warpui::App::test((), |mut app| async move {
            crate::test_util::settings::initialize_settings_for_tests(&mut app);
            app.update(|ctx| {
                if local {
                    crate::local_terminal::enable_for_test(ctx);
                }
                init_actions_from_parent_view(ctx, &ContextPredicate::Just(true), |action| action);
                for name in ["appearance:choose_menu_icon", "appearance:reset_menu_icon"] {
                    assert_eq!(
                        ctx.editable_bindings().any(|binding| binding.name == name),
                        local
                    );
                }
            });
        });
    }
}
