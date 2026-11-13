use std::collections::HashMap;

use settings::Setting as _;
use warp::integration_testing::command_search::{
    assert_command_search_has_results, assert_command_search_is_open,
    assert_history_filter_is_active,
};
use warp::integration_testing::local_terminal::{
    AuthFixture, LEGACY_HISTORY_COMMAND, assert_external_cli_finished_successfully,
    assert_legacy_tabs_and_history, assert_local_settings_pane_is_open_once,
    assert_onboarding_incomplete, assert_shell_runtime_keeps_legacy_agent_preference,
    assert_startup_state, configure_startup, seed_legacy_history,
};
use warp::integration_testing::persistence::{PersistenceScope, database_file_path_for_scope};
use warp::integration_testing::settings::{assert_settings_widget_rendered, open_settings_page};
use warp::integration_testing::terminal::util::ExactLine;
use warp::integration_testing::terminal::{
    assert_active_block_output_for_single_terminal_in_tab, assert_active_block_received_precmd,
    assert_command_executed_for_single_terminal_in_tab, assert_execute_command_successfully,
    assert_input_editor_contents, execute_long_running_command_for_pane,
    wait_until_bootstrapped_single_pane_for_tab,
};
use warp::integration_testing::workspace::assert_focused_tab_index;
use warp::settings::ThemeSettings;
use warp::settings_view::SettingsSection;
use warp::terminal::model::session::get_local_hostname;
use warp::terminal::shell::ShellType;
use warp::themes::theme::ThemeKind;
use warp::workspace::{CommandSearchOptions, InitContent, WorkspaceAction};
use warpui_core::assets::asset_cache::{AssetCache, AssetSource, AssetState};
use warpui_core::image_cache::ImageType;
use warpui_core::integration::TestStep;
use warpui_core::{SingletonEntity, TypedActionView, async_assert};

use super::TEST_ONLY_ASSETS;
use crate::Builder;
use crate::util::{get_local_user, write_histfiles_for_test};

pub fn test_local_terminal_first_launch_without_account() -> Builder {
    Builder::new()
        .use_tmp_filesystem_for_test_root_directory()
        .with_setup(|_| configure_startup(AuthFixture::Empty))
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(
            TestStep::new("真实无账号首启直接进入终端")
                .add_assertion(assert_startup_state)
                .add_assertion(assert_onboarding_incomplete),
        )
        .with_step(assert_execute_command_successfully(
            "printf 'tinywarp-l1-local-command\\n'",
            ExactLine::from("tinywarp-l1-local-command"),
        ))
        .with_step(
            TestStep::new("命令执行后不写账号凭据或 onboarding")
                .add_assertion(assert_startup_state)
                .add_assertion(assert_onboarding_incomplete),
        )
}

pub fn test_local_terminal_preserves_legacy_account_history_and_tabs() -> Builder {
    Builder::new()
        .use_tmp_filesystem_for_test_root_directory()
        .with_user_defaults(HashMap::from([(
            "HasCompletedOnboarding".to_owned(),
            "true".to_owned(),
        )]))
        .with_setup(|utils| {
            configure_startup(AuthFixture::LegacyUser);
            warp::integration_testing::create_file_from_assets(
                TEST_ONLY_ASSETS,
                "three_tabs.sqlite",
                &database_file_path_for_scope(&PersistenceScope::App),
            );
            seed_legacy_history(
                &get_local_user(),
                &get_local_hostname().expect("本地 hostname"),
            );
            write_histfiles_for_test(
                utils.test_dir(),
                vec![LEGACY_HISTORY_COMMAND],
                [
                    ShellType::Zsh,
                    ShellType::Bash,
                    ShellType::Fish,
                    ShellType::PowerShell,
                ],
            );
        })
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(wait_until_bootstrapped_single_pane_for_tab(1))
        .with_step(wait_until_bootstrapped_single_pane_for_tab(2))
        .with_step(
            TestStep::new("旧账号记录不被使用，历史与三个 Tab 保留")
                .add_assertion(assert_startup_state)
                .add_assertion(assert_legacy_tabs_and_history)
                .add_named_assertion(
                    "保留旧版 fixture 的活动标签选择",
                    assert_focused_tab_index(2),
                ),
        )
        .with_step(
            TestStep::new("切换到执行 helper 对应的标签")
                .with_action(|app, window_id, _| {
                    warp::integration_testing::view_getters::workspace_view(app, window_id).update(
                        app,
                        |workspace, ctx| {
                            workspace.handle_action(&WorkspaceAction::ActivateTab(0), ctx);
                        },
                    );
                })
                .add_named_assertion("执行输入与断言都使用标签 0", assert_focused_tab_index(0)),
        )
        .with_step(assert_execute_command_successfully(
            "printf 'tinywarp-l1-upgrade-command\\n'",
            ExactLine::from("tinywarp-l1-upgrade-command"),
        ))
        .with_step(
            TestStep::new("升级后仍能执行命令且旧账号字节不变")
                .add_assertion(assert_startup_state)
                .add_assertion(assert_legacy_tabs_and_history),
        )
        .with_step(
            TestStep::new("在真实命令搜索界面查询旧历史")
                .with_action(|app, window_id, _| {
                    warp::integration_testing::view_getters::workspace_view(app, window_id).update(
                        app,
                        |workspace, ctx| {
                            workspace.handle_action(
                                &WorkspaceAction::ShowCommandSearch(CommandSearchOptions {
                                    filter: Some(warp::search::QueryFilter::History),
                                    init_content: InitContent::Custom(
                                        LEGACY_HISTORY_COMMAND.to_owned(),
                                    ),
                                }),
                                ctx,
                            );
                        },
                    );
                })
                .add_named_assertion("本地命令搜索已打开", assert_command_search_is_open())
                .add_named_assertion("保持历史过滤", assert_history_filter_is_active())
                .add_named_assertion(
                    "旧历史命令具有搜索结果",
                    assert_command_search_has_results(),
                ),
        )
        .with_step(
            TestStep::new("接受旧历史搜索结果后原命令进入输入框")
                .with_keystrokes(&["enter"])
                .add_named_assertion(
                    "旧命令逐字保留",
                    assert_input_editor_contents(0, LEGACY_HISTORY_COMMAND),
                )
                .add_assertion(assert_startup_state),
        )
}

/// 使用真实子 CLI 验证 PTY/输入/输出；不依赖任何联网 AI 工具。
pub fn test_local_terminal_external_cli_ignores_legacy_agent_default() -> Builder {
    use warp::settings::ai::DefaultSessionMode;

    let command = r#"/bin/sh "$HOME/tinywarp-l2-external-cli.sh""#;
    Builder::new()
        .use_tmp_filesystem_for_test_root_directory()
        .with_user_defaults(HashMap::from([(
            DefaultSessionMode::storage_key().to_owned(),
            serde_json::to_string(&DefaultSessionMode::Agent).expect("沿用旧偏好真实序列化"),
        )]))
        .with_setup(|utils| {
            configure_startup(AuthFixture::Empty);
            // 故意模拟从旧宿主继承广告；必须由真实 PTY 路径删除。
            utils.set_env(
                warp_core::cli_agent_protocol::WARP_CLI_AGENT_PROTOCOL_VERSION_ENV,
                Some("l2-inherited-fixture"),
            );
            std::fs::write(
                utils.test_dir().join("tinywarp-l2-external-cli.sh"),
                r#"#!/bin/sh
if [ "${WARP_CLI_AGENT_PROTOCOL_VERSION+x}" = x ]; then
    printf 'tinywarp-l2-unexpected-protocol-advertisement\n'
    exit 31
fi
printf 'tinywarp-l2-cli-ready\n'
IFS= read -r answer || exit 32
printf 'tinywarp-l2-stdin:%s\n' "$answer"
[ "$answer" = '石头鱼 L2 {stdin}' ] || exit 33
exit 0
"#,
            )
            .expect("仅在临时 HOME 创建无网络子 CLI fixture");
        })
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(
            TestStep::new("旧默认 AI 偏好不影响真实无账号终端首启")
                .add_assertion(assert_startup_state)
                .add_assertion(assert_shell_runtime_keeps_legacy_agent_preference),
        )
        .with_step(
            execute_long_running_command_for_pane(0, 0, command).add_named_assertion(
                "子 CLI 不含协议广告，已进入 stdin 读取",
                assert_active_block_output_for_single_terminal_in_tab(
                    ExactLine::from("tinywarp-l2-cli-ready"),
                    0,
                ),
            ),
        )
        .with_step(
            TestStep::new("普通 Enter 将中文 stdin 交给外部 CLI 并正常退出")
                .with_typed_characters(&["石头鱼 L2 {stdin}"])
                .with_keystrokes(&["enter"])
                .add_named_assertion(
                    "真实命令完成",
                    assert_command_executed_for_single_terminal_in_tab(0, command.to_owned()),
                )
                .add_assertion(assert_external_cli_finished_successfully)
                .add_named_assertion(
                    "主 shell 已重新就绪",
                    assert_active_block_received_precmd(0, 0),
                )
                .add_assertion(assert_startup_state)
                .add_assertion(assert_shell_runtime_keeps_legacy_agent_preference),
        )
}

pub fn test_local_terminal_settings_open_once_and_return_to_terminal() -> Builder {
    let (_, content_widget_id) =
        warp::settings_view::settings_widget_deeplink_target("global_hotkey")
            .expect("沿用 Features 本地设置控件的既有稳定映射");
    let menu_step = TestStep::new("展开本地应用菜单").with_action(|app, window_id, _| {
        warp::integration_testing::view_getters::workspace_view(app, window_id)
            .update(app, |workspace, ctx| {
                workspace.handle_action(&WorkspaceAction::ToggleUserMenu, ctx)
            });
    });
    let menu_step = if std::env::var_os("WARPUI_USE_REAL_DISPLAY_IN_INTEGRATION_TESTS").is_some() {
        menu_step.with_take_screenshot("application-menu.png")
    } else {
        menu_step
    };
    let settings_step = TestStep::new("普通设置入口复用原标签并回到外观")
        .with_action(|app, window_id, _| {
            warp::integration_testing::view_getters::workspace_view(app, window_id)
                .update(app, |workspace, ctx| {
                    workspace.handle_action(&WorkspaceAction::ShowSettings, ctx)
                });
        })
        .add_assertion(assert_local_settings_pane_is_open_once)
        .add_named_assertion("仍聚焦同一个设置标签", assert_focused_tab_index(1));
    let settings_step =
        if std::env::var_os("WARPUI_USE_REAL_DISPLAY_IN_INTEGRATION_TESTS").is_some() {
            settings_step.with_take_screenshot("settings-appearance.png")
        } else {
            settings_step
        };
    Builder::new()
        .use_tmp_filesystem_for_test_root_directory()
        .with_setup(|_| configure_startup(AuthFixture::Empty))
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(
            TestStep::new("无账号终端首启保持独立测试身份").add_assertion(assert_startup_state),
        )
        .with_step(assert_settings_widget_rendered(content_widget_id, false))
        .with_step(menu_step)
        .with_step(
            TestStep::new("从菜单用键盘打开真实本地设置")
                .with_keystrokes(&["down", "enter"])
                .add_assertion(assert_local_settings_pane_is_open_once)
                .add_named_assertion("设置标签获得焦点", assert_focused_tab_index(1))
                .add_assertion(assert_startup_state),
        )
        .with_step(assert_settings_widget_rendered(content_widget_id, false))
        .with_step(open_settings_page(SettingsSection::Features))
        .with_step(assert_settings_widget_rendered(content_widget_id, true))
        .with_step(open_settings_page(SettingsSection::About))
        .with_step(about_theme_preview(ThemeKind::Dark, "about-dark.png"))
        .with_step(about_theme_preview(ThemeKind::Light, "about-light.png"))
        .with_step(settings_step)
        .with_step(
            TestStep::new("返回原终端标签")
                .with_action(|app, window_id, _| {
                    warp::integration_testing::view_getters::workspace_view(app, window_id).update(
                        app,
                        |workspace, ctx| {
                            workspace.handle_action(&WorkspaceAction::ActivateTab(0), ctx)
                        },
                    );
                })
                .add_named_assertion("原终端获得焦点", assert_focused_tab_index(0)),
        )
        .with_step(assert_execute_command_successfully(
            "printf 'tinywarp-l2-settings-return\\n'",
            ExactLine::from("tinywarp-l2-settings-return"),
        ))
        .with_step(
            TestStep::new("打开设置后本地身份及原终端保持可用")
                .add_assertion(assert_startup_state)
                .add_assertion(assert_local_settings_pane_is_open_once),
        )
}

fn about_theme_preview(theme: ThemeKind, screenshot: &'static str) -> TestStep {
    let step = TestStep::new(&format!("关于页主题与字标加载：{theme:?}"))
        .with_action(move |app, _, _| {
            app.update(|ctx| {
                ThemeSettings::handle(ctx).update(ctx, |settings, ctx| {
                    settings.use_system_theme.set_value(false, ctx).unwrap();
                    settings.theme_kind.set_value(theme.clone(), ctx).unwrap();
                });
            });
        })
        .add_named_assertion("TinyWarp 字标 PNG 成功解码", |app, _| {
            let loaded = app.read(|ctx| {
                matches!(
                    AssetCache::as_ref(ctx).load_asset::<ImageType>(AssetSource::Bundled {
                        path: "bundled/png/tinywarp-logo.png",
                    }),
                    AssetState::Loaded { .. }
                )
            });
            async_assert!(loaded, "关于页字标资源应可加载并解码")
        });
    if std::env::var_os("WARPUI_USE_REAL_DISPLAY_IN_INTEGRATION_TESTS").is_some() {
        step.with_take_screenshot(screenshot)
    } else {
        step
    }
}
