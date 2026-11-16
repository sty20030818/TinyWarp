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
#[cfg(not(target_family = "wasm"))]
fn complete_local_keybinding_page_uses_chinese_and_excludes_disabled_actors() {
    use warpui::keymap::DescriptionContext;

    use crate::auth::AuthStateProvider;
    use crate::gpu_state::GPUState;
    use crate::workspaces::user_workspaces::UserWorkspaces;

    App::test((), |mut app| async move {
        app.update(crate::local_terminal::enable_for_test);
        crate::test_util::settings::initialize_settings_for_tests(&mut app);
        app.add_singleton_model(|_| GPUState::new());
        app.add_singleton_model(|_| AuthStateProvider::new_logged_out_for_test());
        app.add_singleton_model(UserWorkspaces::default_mock);
        app.update(crate::register_keybindings);
        app.update(|ctx| {
            let raw_count = ctx.editable_bindings().count();
            let bindings = super::editable_command_bindings(ctx);
            assert!(bindings.len() > 50, "完整启动注册应保留本地终端动作");
            assert!(
                raw_count > bindings.len(),
                "完整注册包含被本地策略排除的动作"
            );
            let missing = bindings
                .iter()
                .filter(|binding| {
                    binding
                        .description
                        .in_context(DescriptionContext::Default)
                        .is_ascii()
                })
                .map(|binding| {
                    format!(
                        "{} | {} | {}",
                        binding.name,
                        binding
                            .action
                            .as_ref()
                            .map(|action| action.as_ref().type_name())
                            .unwrap_or("无 typed action"),
                        binding.description.in_context(DescriptionContext::Default)
                    )
                })
                .collect::<Vec<_>>();
            assert!(
                missing.is_empty(),
                "本地快捷键页仍有英文文案：\n{}",
                missing.join("\n")
            );
            assert!(
                ctx.editable_bindings().any(|lens| lens
                    .action
                    .as_ref()
                    .as_any()
                    .is::<crate::ai::ai_document_view::AIDocumentAction>(
                )),
                "AI文档原动作仍注册"
            );
            for name in [
                "Close Workflow",
                "Close Env Var Collection",
                "ai_assistant_panel:focus_terminal_input",
                "editor:next_find_match",
                "editor:previous_find_match",
                "workspace:link_to_slack",
                "workspace:jump_to_latest_toast",
                "project_buttons:create_new_project",
                "code_view:save",
            ] {
                assert!(
                    !bindings.iter().any(|binding| binding.name == name),
                    "已停用动作仍在快捷键页：{name}"
                );
            }
            for name in [
                "editor_view:left",
                "terminal:paste",
                "input:clear_screen",
                "workspace:disable_terminal_input_syncing",
                "input:toggle_workflows",
                "appearance:choose_menu_icon",
                "appearance:reset_menu_icon",
            ] {
                assert!(
                    bindings.iter().any(|binding| binding.name == name),
                    "保留本地动作：{name}"
                );
            }
            for (name, chinese, english) in [
                #[cfg(target_os = "macos")]
                ("workspace:sample_process", "采样当前进程", "Sample Process"),
                (
                    "workspace:send_feedback",
                    "发送反馈（打开外部链接）",
                    "Send feedback (opens external link)",
                ),
            ] {
                let binding = bindings
                    .iter()
                    .find(|binding| binding.name == name)
                    .expect("保留本地动作");
                assert_eq!(
                    binding.description.in_context(DescriptionContext::Default),
                    chinese
                );
                assert!(
                    binding
                        .description
                        .search_aliases()
                        .contains(&english.to_owned())
                );
            }
        });
    });
}

#[test]
fn localized_registered_shortcuts_preserve_english_search_and_custom_keys() {
    use warpui::keymap::DescriptionContext;

    use super::{CommandBinding, filter_bindings_including_keystroke};

    App::test((), |mut app| async move {
        app.update(crate::local_terminal::enable_for_test);
        app.update(crate::editor::init);
        app.update(terminal::init);
        app.update(crate::terminal::input::init);
        app.update(crate::workspace::sync_inputs::init);
        app.update(crate::undo_close::init);
        app.update(crate::root_view::init);

        app.update(|ctx| {
            for lens in ctx.editable_bindings().filter(|lens| {
                (lens.name.starts_with("editor_view:") || lens.name.starts_with("editor:"))
                    && super::is_action_available_in_local_terminal(lens.action.as_ref(), ctx)
            }) {
                let binding = CommandBinding::from_editable_lens(lens, ctx);
                assert!(
                    !binding
                        .description
                        .in_context(DescriptionContext::Default)
                        .is_ascii(),
                    "共享编辑器动作应有中文显示文案：{}",
                    binding.name
                );
            }
            for (name, english, chinese) in [
                ("editor_view:left", "Move cursor left", "向左移动光标"),
                (
                    "editor_view:move_backward_one_word",
                    "Move backward one word",
                    "向左移动一个单词",
                ),
                ("editor_view:home", "Home", "移至可视行首"),
                ("terminal:paste", "Paste", "粘贴"),
                ("input:clear_screen", "Clear screen", "清屏"),
                ("input:toggle_workflows", "Workflows", "工作流"),
                (
                    "terminal:scroll_up_one_page",
                    "Scroll terminal output up one page",
                    "将终端输出向上滚动一页",
                ),
                (
                    "workspace:disable_terminal_input_syncing",
                    "Stop Synchronizing Any Panes",
                    "停止同步所有窗格的输入",
                ),
                (
                    "app:reopen_closed_session",
                    "Reopen closed session",
                    "重新打开已关闭的会话",
                ),
                (
                    "root_view:toggle_fullscreen",
                    "Toggle fullscreen",
                    "切换全屏",
                ),
            ] {
                let bindings = ctx
                    .editable_bindings()
                    .filter(|lens| lens.name == name)
                    .map(|lens| {
                        let id = lens.id;
                        let binding = CommandBinding::from_editable_lens(lens, ctx);
                        assert_eq!(binding.id, id);
                        assert_eq!(binding.name, name);
                        assert_eq!(
                            binding.description.in_context(DescriptionContext::Default),
                            chinese
                        );
                        assert!(
                            binding
                                .description
                                .search_aliases()
                                .contains(&english.to_string())
                        );
                        binding
                    })
                    .collect::<Vec<_>>();
                assert!(!bindings.is_empty(), "动作 {name} 必须保留注册");
                assert!(
                    bindings
                        .iter()
                        .all(|binding| binding.description == bindings[0].description),
                    "同名动作的本地化描述和英文别名必须一致，避免快捷键页重复行：{name}"
                );
                for query in [chinese, english] {
                    let found = filter_bindings_including_keystroke(
                        bindings.iter(),
                        query,
                        DescriptionContext::Default,
                    )
                    .collect::<Vec<_>>();
                    assert_eq!(found.len(), bindings.len(), "查询 {query}");
                }

                let custom = Keystroke::parse("cmd-shift-L").unwrap();
                ctx.set_custom_trigger(name.into(), Trigger::Keystrokes(vec![custom.clone()]));
                for lens in ctx.editable_bindings().filter(|lens| lens.name == name) {
                    let binding = CommandBinding::from_editable_lens(lens, ctx);
                    assert_eq!(binding.name, name);
                    assert_eq!(binding.trigger.as_ref(), Some(&custom));
                    assert!(
                        filter_bindings_including_keystroke(
                            std::iter::once(&binding),
                            "cmd shift l",
                            DescriptionContext::Default,
                        )
                        .next()
                        .is_some()
                    );
                }
            }
        });
    });
}

#[test]
fn local_terminal_hides_unavailable_view_shortcuts_without_removing_registration() {
    use super::{CommandBinding, is_action_available_in_local_terminal};
    use crate::pane_group::pane::view::PaneAction;
    use crate::workspace::view::right_panel::RightPanelAction;

    App::test((), |mut app| async move {
        app.update(terminal::init);
        app.update(crate::terminal::input::init);
        app.update(|ctx| {
            ctx.register_editable_bindings([
                EditableBinding::new("test:share_pane", "Share pane", PaneAction::ShareContents),
                EditableBinding::new(
                    "test:maximize_code_review",
                    "Toggle Maximize Code Review Panel",
                    RightPanelAction::ToggleMaximize,
                ),
            ]);
            for name in [
                "test:share_pane",
                "test:maximize_code_review",
                "input:clear_and_reset_ai_context_menu_query",
                "terminal:stop_sharing_current_session",
            ] {
                let lens = ctx.get_binding_by_name(name).expect("原动作已注册");
                assert!(CommandBinding::from_lens(lens, ctx).is_some());
            }
            crate::local_terminal::enable_for_test(ctx);
            for name in [
                "test:share_pane",
                "test:maximize_code_review",
                "input:clear_and_reset_ai_context_menu_query",
                "terminal:stop_sharing_current_session",
            ] {
                let lens = ctx.get_binding_by_name(name).expect("本地策略不删除原注册");
                assert!(!is_action_available_in_local_terminal(
                    lens.action.as_ref(),
                    ctx
                ));
                assert!(CommandBinding::from_lens(lens, ctx).is_none());
            }
        });
    });
}

#[test]
#[cfg(not(target_family = "wasm"))]
fn local_terminal_hides_disabled_editor_candidates_without_removing_registration() {
    use warpui::keymap::DescriptionContext;

    use super::{CommandBinding, is_action_available_in_local_terminal};

    App::test((), |mut app| async move {
        app.update(crate::editor::init);
        app.update(crate::notebooks::init);
        app.update(crate::code::view::init);
        app.update(|ctx| {
            let ids = ctx
                .editable_bindings()
                .filter(|lens| lens.name == "editor_view:left")
                .map(|lens| {
                    assert!(is_action_available_in_local_terminal(
                        lens.action.as_ref(),
                        ctx
                    ));
                    lens.id
                })
                .collect::<Vec<_>>();
            assert_eq!(ids.len(), 3, "三种编辑器的原始注册都存在");
            let standalone_bindings = [
                "code_view:save",
                "notebookview:increase_font_size",
                "notebookview:reload_file",
            ]
            .map(|name| {
                let lens = ctx
                    .get_binding_by_name(name)
                    .expect("独立编辑器的真实动作已注册");
                assert!(CommandBinding::from_lens(lens, ctx).is_some());
                (name, ctx.get_binding_by_name(name).unwrap().id)
            });
            crate::local_terminal::enable_for_test(ctx);
            for (name, id) in standalone_bindings {
                let lens = ctx
                    .get_binding_by_name(name)
                    .expect("本地策略保留原动作注册");
                assert_eq!(lens.id, id);
                assert!(!is_action_available_in_local_terminal(
                    lens.action.as_ref(),
                    ctx
                ));
                assert!(CommandBinding::from_lens(lens, ctx).is_none());
            }
            let local_candidates = ctx
                .editable_bindings()
                .filter(|lens| lens.name == "editor_view:left")
                .filter(|lens| is_action_available_in_local_terminal(lens.action.as_ref(), ctx))
                .map(|lens| CommandBinding::from_editable_lens(lens, ctx))
                .collect::<Vec<_>>();
            assert_eq!(local_candidates.len(), 1);
            assert_eq!(
                local_candidates[0]
                    .description
                    .in_context(DescriptionContext::Default),
                "向左移动光标"
            );
            assert_eq!(
                ctx.editable_bindings()
                    .filter(|lens| lens.name == "editor_view:left")
                    .map(|lens| lens.id)
                    .collect::<Vec<_>>(),
                ids,
                "候选过滤不删除或改变原始注册"
            );
        });
    });
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
