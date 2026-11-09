use std::collections::HashMap;

use warp::integration_testing::command_search::{
    assert_command_search_has_results, assert_command_search_is_open,
    assert_history_filter_is_active,
};
use warp::integration_testing::local_terminal::{
    AuthFixture, LEGACY_HISTORY_COMMAND, assert_legacy_tabs_and_history,
    assert_onboarding_incomplete, assert_startup_state, configure_startup, seed_legacy_history,
};
use warp::integration_testing::persistence::{PersistenceScope, database_file_path_for_scope};
use warp::integration_testing::terminal::util::ExactLine;
use warp::integration_testing::terminal::{
    assert_execute_command_successfully, assert_input_editor_contents,
    wait_until_bootstrapped_single_pane_for_tab,
};
use warp::integration_testing::workspace::assert_focused_tab_index;
use warp::terminal::model::session::get_local_hostname;
use warp::terminal::shell::ShellType;
use warp::workspace::{CommandSearchOptions, InitContent, WorkspaceAction};
use warpui_core::TypedActionView;
use warpui_core::integration::TestStep;

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
