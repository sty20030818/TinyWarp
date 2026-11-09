use warpui::App;
use warpui::platform::WindowStyle;

use super::{
    Appearance, DisplayIdx, Dropdown, FeaturesPageAction, GlobalHotkeyMode, QuakeModeSettings,
    init_display_count_dropdown, init_global_hotkey_dropdown,
};

#[test]
fn localized_dropdown_selection_preserves_hotkey_and_display_actions() {
    App::test((), |mut app| async move {
        app.add_singleton_model(|_| Appearance::mock());
        let (_, dropdown) = app.add_window(WindowStyle::NotStealFocus, Dropdown::new);

        app.update(|ctx| {
            dropdown.update(ctx, |dropdown, ctx| {
                for mode in [
                    GlobalHotkeyMode::Disabled,
                    GlobalHotkeyMode::QuakeMode,
                    GlobalHotkeyMode::ActivationHotkey,
                ] {
                    init_global_hotkey_dropdown(mode, dropdown, ctx);
                    assert_eq!(
                        dropdown.selected_action(),
                        Some(FeaturesPageAction::SetGlobalHotkeyMode(mode)),
                    );
                }

                for pin_screen in [
                    None,
                    Some(DisplayIdx::Primary),
                    Some(DisplayIdx::External(0)),
                    Some(DisplayIdx::External(1)),
                ] {
                    let settings = QuakeModeSettings {
                        pin_screen,
                        ..Default::default()
                    };
                    init_display_count_dropdown(&settings, 3, dropdown, ctx);
                    assert_eq!(
                        dropdown.selected_action(),
                        Some(FeaturesPageAction::QuakeEditorSetPinScreen(pin_screen)),
                    );
                }
            });
        });
    });
}
