use warpui::App;
use warpui::keymap::{EditableBinding, Keystroke, Trigger};
use warpui::platform::OperatingSystem;

use crate::terminal;
use crate::util::bindings::{keybinding_name_to_display_string, trigger_to_keystroke};
use crate::workspace::WorkspaceAction;

#[test]
fn test_keybinding_name_to_display_string() {
    App::test((), |mut app| async move {
        app.update(|ctx| {
            ctx.register_editable_bindings([
                EditableBinding::new(
                    "workspace:show_settings",
                    "Open settings",
                    WorkspaceAction::ShowSettings,
                )
                .with_key_binding("cmd-,"),
                EditableBinding::new(
                    "workspace:toggle_resource_center",
                    "Toggle Resource Center",
                    WorkspaceAction::ToggleResourceCenter,
                ),
            ]);

            let displayed_keybinding = if OperatingSystem::get().is_mac() {
                "⌘,"
            } else {
                "Logo ,"
            };
            assert_eq!(
                Some(displayed_keybinding),
                keybinding_name_to_display_string("workspace:show_settings", ctx).as_deref()
            );

            assert_eq!(
                None,
                keybinding_name_to_display_string("workspace:toggle_resource_center", ctx)
            );

            ctx.set_custom_trigger(
                "workspace:show_settings".to_owned(),
                Trigger::Keystrokes(vec![Keystroke::parse("cmd-shift-<").unwrap()]),
            );

            let displayed_keybinding = if OperatingSystem::get().is_mac() {
                "⇧⌘<"
            } else {
                "Shift Logo <"
            };
            assert_eq!(
                Some(displayed_keybinding),
                keybinding_name_to_display_string("workspace:show_settings", ctx).as_deref()
            );

            ctx.set_custom_trigger(
                "workspace:toggle_resource_center".to_owned(),
                Trigger::Keystrokes(vec![Keystroke::parse("cmd-alt-/").unwrap()]),
            );

            let expected_keybinding = if OperatingSystem::get().is_mac() {
                "⌥⌘/"
            } else {
                "Alt Logo /"
            };
            assert_eq!(
                Some(expected_keybinding),
                keybinding_name_to_display_string("workspace:toggle_resource_center", ctx)
                    .as_deref()
            );
        });
    });
}

#[test]
fn test_orchestration_cycle_bindings_are_editable() {
    App::test((), |mut app| async move {
        app.update(terminal::init);

        app.update(|ctx| {
            let next = ctx
                .editable_bindings()
                .find(|binding| binding.name == "terminal:cycle_next_orchestration_child_agent")
                .and_then(|binding| trigger_to_keystroke(binding.trigger));
            let previous = ctx
                .editable_bindings()
                .find(|binding| binding.name == "terminal:cycle_previous_orchestration_child_agent")
                .and_then(|binding| trigger_to_keystroke(binding.trigger));

            assert_eq!(next, Keystroke::parse("ctrl-alt-]").ok());
            assert_eq!(previous, Keystroke::parse("ctrl-alt-[").ok());
        });
    });
}

#[test]
fn test_toggle_maximize_pane_binding_is_editable() {
    App::test((), |mut app| async move {
        app.update(crate::pane_group::init);

        app.update(|ctx| {
            use crate::pane_group::TOGGLE_MAXIMIZE_PANE_BINDING_NAME;

            // The toggle-maximize-pane action is registered as an editable binding so
            // it can be assigned a shortcut in Settings → Keyboard shortcuts.
            assert!(
                ctx.editable_bindings()
                    .any(|binding| binding.name == TOGGLE_MAXIMIZE_PANE_BINDING_NAME),
                "{TOGGLE_MAXIMIZE_PANE_BINDING_NAME} should be registered as an editable binding"
            );

            // It ships with a mac-only default shortcut (cmd-shift-enter) via its custom
            // action; other platforms have no default until the user assigns one. Either
            // way, whatever resolves here is what the pane header menu item surfaces.
            let default = keybinding_name_to_display_string(TOGGLE_MAXIMIZE_PANE_BINDING_NAME, ctx);
            if OperatingSystem::get().is_mac() {
                assert_eq!(Some("⇧⌘⏎"), default.as_deref());
            } else {
                assert_eq!(None, default);
            }

            // A reassigned shortcut resolves to its display string on every platform.
            ctx.set_custom_trigger(
                TOGGLE_MAXIMIZE_PANE_BINDING_NAME.to_owned(),
                Trigger::Keystrokes(vec![Keystroke::parse("cmd-shift-M").unwrap()]),
            );

            let displayed_keybinding = if OperatingSystem::get().is_mac() {
                "⇧⌘M"
            } else {
                "Shift Logo M"
            };
            assert_eq!(
                Some(displayed_keybinding),
                keybinding_name_to_display_string(TOGGLE_MAXIMIZE_PANE_BINDING_NAME, ctx)
                    .as_deref()
            );
        });
    });
}

#[test]
fn test_terminal_page_scroll_bindings_are_editable() {
    App::test((), |mut app| async move {
        app.update(terminal::init);

        app.update(|ctx| {
            let page_up = ctx
                .editable_bindings()
                .find(|binding| binding.name == "terminal:scroll_up_one_page")
                .and_then(|binding| trigger_to_keystroke(binding.trigger));
            let page_down = ctx
                .editable_bindings()
                .find(|binding| binding.name == "terminal:scroll_down_one_page")
                .and_then(|binding| trigger_to_keystroke(binding.trigger));

            assert_eq!(page_up, Keystroke::parse("pageup").ok());
            assert_eq!(page_down, Keystroke::parse("pagedown").ok());
        });
    });
}

#[test]
fn test_markdown_display_mode_toggle_is_rebound_in_both_directions() {
    use crate::notebooks::file::TOGGLE_MARKDOWN_DISPLAY_MODE_BINDING_NAME;

    App::test((), |mut app| async move {
        app.update(crate::code::view::init);
        app.update(crate::notebooks::file::init);

        app.update(|ctx| {
            let toggle_keystrokes = |ctx: &warpui::AppContext| {
                ctx.editable_bindings()
                    .filter(|binding| binding.name == TOGGLE_MARKDOWN_DISPLAY_MODE_BINDING_NAME)
                    .map(|binding| trigger_to_keystroke(binding.trigger))
                    .collect::<Vec<_>>()
            };

            // The rendered and raw panes each register one direction of the toggle.
            let default = Keystroke::parse("cmdorctrl-e").ok();
            assert_eq!(vec![default.clone(), default], toggle_keystrokes(ctx));

            ctx.set_custom_trigger(
                TOGGLE_MARKDOWN_DISPLAY_MODE_BINDING_NAME.to_owned(),
                Trigger::Keystrokes(vec![Keystroke::parse("cmd-shift-M").unwrap()]),
            );

            let custom = Keystroke::parse("cmd-shift-M").ok();
            assert_eq!(vec![custom.clone(), custom], toggle_keystrokes(ctx));
        });
    });
}

#[test]
fn localized_description_survives_command_binding_materialization() {
    use warpui::keymap::DescriptionContext;

    use super::{CommandBinding, localized_description};

    App::test((), |mut app| async move {
        app.update(|ctx| {
            let description = localized_description("action.open_settings", "Open Settings")
                .with_search_alias("Settings")
                .with_dynamic_override(|_| Some("打开 MCP 设置".into()));
            ctx.register_editable_bindings([EditableBinding::new(
                "workspace:h1_localized_settings",
                description,
                WorkspaceAction::ShowSettings,
            )]);
            let lens = ctx
                .editable_bindings()
                .find(|binding| binding.name == "workspace:h1_localized_settings")
                .expect("localized binding is registered");
            let id = lens.id;
            let binding = CommandBinding::from_editable_lens(lens, ctx);
            assert_eq!(binding.id, id);
            assert_eq!(binding.name, "workspace:h1_localized_settings");
            assert_eq!(
                binding.description.in_context(DescriptionContext::Default),
                "打开 MCP 设置"
            );
            assert_eq!(
                binding.description.search_aliases(),
                &["Open Settings", "Settings"]
            );
        });
    });
}

#[test]
fn shortcut_filter_keeps_english_alias_and_original_binding_after_localization() {
    use warpui::keymap::{BindingDescription, DescriptionContext};

    use super::{CommandBinding, filter_bindings_including_keystroke};

    let mut binding = CommandBinding::new(
        "terminal:accept_autosuggestion".into(),
        String::new(),
        Some(Keystroke::parse("cmd-right").unwrap()),
    );
    binding.description = BindingDescription::new_preserve_case("接受自动建议")
        .with_search_alias("Accept Autosuggestion");
    for query in [
        "接受",
        "Accept Autosuggestion",
        "ACCEPT AUTOSUGGESTION",
        "cmd right",
    ] {
        let results = filter_bindings_including_keystroke(
            std::iter::once(&binding),
            query,
            DescriptionContext::Default,
        )
        .collect::<Vec<_>>();
        assert_eq!(results.len(), 1, "查询 {query}");
        assert_eq!(results[0].1.id, binding.id);
        assert!(
            results[0].0.is_none(),
            "快捷键搜索不使用英文别名下标高亮中文"
        );
    }
}

#[test]
fn local_terminal_hides_account_candidates_without_removing_registered_bindings() {
    use super::{CommandBinding, is_action_available_in_local_terminal};
    use crate::settings_view::{SettingsAction, SettingsSection};

    App::test((), |mut app| async move {
        app.update(|ctx| {
            crate::local_terminal::enable_for_test(ctx);
            ctx.register_editable_bindings([
                EditableBinding::new("test:l1_log_out", "Log out", WorkspaceAction::LogOut),
                EditableBinding::new(
                    "test:l1_settings",
                    "Settings",
                    WorkspaceAction::ShowSettings,
                ),
                EditableBinding::new(
                    "test:l1_share",
                    "Share block",
                    terminal::view::TerminalAction::OpenShareModal,
                ),
                EditableBinding::new(
                    "test:l1_local_sync",
                    "Synchronize local pane input",
                    WorkspaceAction::ToggleSyncAllTerminalInputsInAllTabs,
                ),
            ]);
            for (name, visible) in [
                ("test:l1_log_out", false),
                ("test:l1_settings", true),
                ("test:l1_share", false),
                ("test:l1_local_sync", true),
            ] {
                let lens = ctx
                    .get_binding_by_name(name)
                    .expect("旧 action 注册必须保留");
                let id = lens.id;
                let binding = CommandBinding::from_lens(lens, ctx);
                assert_eq!(binding.is_some(), visible, "{name}");
                if let Some(binding) = binding {
                    assert_eq!(binding.id, id);
                    assert_eq!(binding.name, name);
                }
            }
            assert!(!is_action_available_in_local_terminal(
                &SettingsAction::SelectAndRefresh(SettingsSection::Account),
                ctx,
            ));
            assert!(is_action_available_in_local_terminal(
                &SettingsAction::SelectAndRefresh(SettingsSection::Scripting),
                ctx,
            ));
        });
    });
}

#[test]
fn local_terminal_candidates_keep_raw_cli_bytes_and_local_history_actions() {
    use super::CommandBinding;
    use crate::terminal::input::InputAction;
    use crate::terminal::view::TerminalAction;

    App::test((), |mut app| async move {
        app.update(|ctx| {
            crate::local_terminal::enable_for_test(ctx);
            let raw_bytes = vec![0x1b, b'[', b'A', 0x03, 0x07];
            ctx.register_editable_bindings([
                EditableBinding::new(
                    "test:l2_raw_cli",
                    "CLI bytes",
                    TerminalAction::UserInputSequence(raw_bytes.clone()),
                ),
                EditableBinding::new("test:l2_paste", "Paste", TerminalAction::Paste),
                EditableBinding::new("test:l2_history", "History", InputAction::CtrlR),
                EditableBinding::new(
                    "test:l2_completions",
                    "Shell completions",
                    InputAction::MaybeOpenCompletionSuggestions,
                ),
                EditableBinding::new("test:l2_agent", "Agent", TerminalAction::SetInputModeAgent),
                EditableBinding::new(
                    "test:l2_rich_cli",
                    "Rich CLI",
                    TerminalAction::ToggleCLIAgentRichInput,
                ),
            ]);
            for (name, visible) in [
                ("test:l2_raw_cli", true),
                ("test:l2_paste", true),
                ("test:l2_history", true),
                ("test:l2_completions", true),
                ("test:l2_agent", false),
                ("test:l2_rich_cli", false),
            ] {
                let lens = ctx.get_binding_by_name(name).expect("原注册与ID保留");
                let id = lens.id;
                let binding = CommandBinding::from_lens(lens, ctx);
                assert_eq!(binding.is_some(), visible, "{name}");
                if let Some(binding) = binding {
                    assert_eq!(binding.id, id);
                    if name == "test:l2_raw_cli" {
                        let action = binding
                            .action
                            .as_ref()
                            .unwrap()
                            .as_ref()
                            .as_any()
                            .downcast_ref::<TerminalAction>()
                            .unwrap();
                        let TerminalAction::UserInputSequence(bytes) = action else {
                            panic!("保留原始PTY动作")
                        };
                        assert_eq!(bytes, &raw_bytes);
                    }
                }
            }
        });
    });
}
