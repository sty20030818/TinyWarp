use ::local_control::protocol::TargetSelector;
use ::local_control::{ErrorCode, InstanceId};
use warpui::App;

use super::create_tab;
use crate::local_control::LocalControlBridge;
use crate::workspace::view::tests::{initialize_app, mock_workspace};

#[test]
fn tab_create_handler_adds_and_activates_terminal_tab() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let workspace = mock_workspace(&mut app);
        let previous_count = workspace.read(&app, |workspace, _| workspace.tab_count());
        let bridge = app.add_singleton_model(LocalControlBridge::new);
        let instance_id = InstanceId("inst_test".to_owned());

        let response = bridge.update(&mut app, |bridge, ctx| {
            bridge.set_instance_id(instance_id.clone());
            create_tab(
                &Some(instance_id.clone()),
                &serde_json::json!({}),
                &TargetSelector::default(),
                ctx,
            )
            .expect("tab.create handler succeeds")
        });

        workspace.read(&app, |workspace, _| {
            assert_eq!(workspace.tab_count(), previous_count + 1);
            assert_eq!(workspace.active_tab_index(), previous_count);
        });
        assert_eq!(response["action"], "tab.create");
        assert_eq!(response["created"], true);
        assert_eq!(response["instance_id"], "inst_test");
        assert_eq!(response["tab"]["previous_count"], previous_count);
        assert_eq!(response["tab"]["count"], previous_count + 1);
        assert_eq!(response["tab"]["active_index"], previous_count);
        assert!(response["tab"]["id"].is_string());
    });
}

#[test]
fn tab_create_rejects_shell_parameter() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let _workspace = mock_workspace(&mut app);
        let bridge = app.add_singleton_model(LocalControlBridge::new);
        let instance_id = InstanceId("inst_test".to_owned());

        let err = bridge.update(&mut app, |bridge, ctx| {
            bridge.set_instance_id(instance_id.clone());
            create_tab(
                &Some(instance_id.clone()),
                &serde_json::json!({ "shell": "zsh" }),
                &TargetSelector::default(),
                ctx,
            )
            .expect_err("shell parameter must be rejected")
        });

        assert_eq!(err.code, ErrorCode::InvalidParams);
    });
}

#[test]
fn local_terminal_tab_create_rejects_agent_before_resolving_or_mutating_a_window() {
    App::test((), |mut app| async move {
        app.update(crate::local_terminal::enable_for_test);
        let bridge = app.add_singleton_model(LocalControlBridge::new);
        let error = bridge
            .update(&mut app, |_, ctx| {
                create_tab(
                    &None,
                    &serde_json::json!({ "tab_type": "agent" }),
                    &TargetSelector::default(),
                    ctx,
                )
            })
            .expect_err("无窗口也应先返回明确的不可用错误");
        assert_eq!(error.code, ErrorCode::UnsupportedAction);
        assert!(error.message.contains("tab.create"));
        assert!(app.read(|ctx| ctx.window_ids().next().is_none()));
    });
}
