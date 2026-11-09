use ::local_control::{ActionKind, ErrorCode};

#[cfg(feature = "local_fs")]
use super::resolve_against_working_directory;
use super::{ensure_surface_available, validate_staged_input_text};
use crate::features::FeatureFlag;
use crate::local_control::handlers::metadata::SurfaceDestination;

#[test]
fn staged_input_rejects_line_breaks_and_control_sequences() {
    assert!(validate_staged_input_text(ActionKind::InputInsert, "safe staged text").is_ok());

    for text in ["line\nbreak", "line\rbreak", "tab\tbreak", "\u{1b}[31m"] {
        let error = validate_staged_input_text(ActionKind::InputInsert, text).err();
        assert!(error.is_some_and(|error| error.code == ErrorCode::InvalidParams));
    }
}

#[cfg(feature = "local_fs")]
#[test]
fn file_open_resolves_relative_paths_against_the_session_working_directory() {
    use std::path::{Path, PathBuf};

    let temp_dir = tempfile::tempdir().expect("temp dir");
    let working_directory = dunce::canonicalize(temp_dir.path()).expect("canonical temp dir");
    let nested = working_directory.join("docs");
    std::fs::create_dir(&nested).expect("nested dir");
    std::fs::write(working_directory.join("README.md"), "# hi").expect("readme");
    std::fs::write(nested.join("guide.md"), "# guide").expect("guide");

    assert_eq!(
        resolve_against_working_directory(Path::new("README.md"), &working_directory),
        working_directory.join("README.md")
    );
    assert_eq!(
        resolve_against_working_directory(Path::new("./README.md"), &working_directory),
        working_directory.join("README.md")
    );
    assert_eq!(
        resolve_against_working_directory(Path::new("docs/guide.md"), &working_directory),
        nested.join("guide.md")
    );
    assert_eq!(
        resolve_against_working_directory(Path::new("../README.md"), &nested),
        working_directory.join("README.md")
    );

    // Paths that do not exist still resolve against the session directory, so a genuine
    // failure reports the session-relative file rather than a process-relative one.
    assert_eq!(
        resolve_against_working_directory(Path::new("missing.md"), &working_directory),
        working_directory.join("missing.md")
    );

    let absolute = PathBuf::from(if cfg!(windows) {
        r"C:\tmp\absolute.md"
    } else {
        "/tmp/absolute.md"
    });
    assert_eq!(
        resolve_against_working_directory(&absolute, &working_directory),
        absolute
    );
}

#[test]
fn unavailable_surface_open_returns_structured_error() {
    let flag_guard = FeatureFlag::AgentManagementView.override_enabled(false);
    warpui::App::test((), |mut app| async move {
        let error = app
            .update(|ctx| {
                ensure_surface_available(
                    ActionKind::SurfaceAgentManagementOpen,
                    SurfaceDestination::AgentManagement,
                    ctx,
                )
            })
            .expect_err("disabled surface is rejected");
        assert_eq!(error.code, ErrorCode::UnsupportedAction);
        assert!(error.message.contains("surface.agent_management.open"));
    });
    drop(flag_guard);
}

#[test]
fn local_terminal_surface_requests_return_errors_before_target_resolution() {
    use ::local_control::protocol::TargetSelector;

    use crate::local_control::LocalControlBridge;
    use crate::settings_view::SettingsSection;

    warpui::App::test((), |mut app| async move {
        app.update(crate::local_terminal::enable_for_test);
        let bridge = app.add_singleton_model(LocalControlBridge::new);
        for action in [
            ActionKind::SurfaceWarpDriveOpen,
            ActionKind::SurfaceWarpDriveToggle,
            ActionKind::SurfaceAiAssistantToggle,
            ActionKind::SurfaceCodeReviewOpen,
            ActionKind::SurfaceCodeReviewToggle,
            ActionKind::SurfaceProjectExplorerOpen,
            ActionKind::SurfaceGlobalSearchOpen,
            ActionKind::SurfaceConversationListOpen,
            ActionKind::SurfaceLeftPanelToggle,
            ActionKind::SurfaceRightPanelToggle,
            ActionKind::SurfaceAgentManagementOpen,
        ] {
            let error = bridge
                .update(&mut app, |_, ctx| {
                    super::handle(
                        &None,
                        action,
                        &serde_json::json!({}),
                        &TargetSelector::default(),
                        ctx,
                    )
                })
                .expect_err("本地禁用功能应先于窗口查找返回不可用");
            assert_eq!(error.code, ErrorCode::UnsupportedAction);
            assert!(error.message.contains(action.as_str()));
        }
        let error = bridge
            .update(&mut app, |_, ctx| {
                super::handle(
                    &None,
                    ActionKind::SurfaceSettingsOpen,
                    &serde_json::json!({ "page": SettingsSection::EditorAndCodeReview.slug() }),
                    &TargetSelector::default(),
                    ctx,
                )
            })
            .expect_err("禁用编辑器设置不能返回成功");
        assert_eq!(error.code, ErrorCode::UnsupportedAction);
        assert!(app.read(|ctx| ctx.window_ids().next().is_none()));
    });
}

#[cfg(feature = "local_fs")]
#[test]
fn local_terminal_file_open_rejects_builtin_editor_and_markdown_before_dispatch() {
    use ::local_control::protocol::TargetSelector;
    use settings::Setting as _;
    use warpui::SingletonEntity as _;

    use crate::local_control::LocalControlBridge;
    use crate::util::file::external_editor::EditorSettings;
    use crate::util::file::external_editor::settings::EditorChoice;
    use crate::workspace::view::tests::{initialize_app, mock_workspace};

    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        app.update(crate::local_terminal::enable_for_test);
        let workspace = mock_workspace(&mut app);
        let count = workspace.read(&app, |workspace, _| workspace.tab_count());
        EditorSettings::handle(&app).update(&mut app, |settings, ctx| {
            settings
                .open_file_editor
                .set_value(EditorChoice::Warp, ctx)
                .unwrap();
            settings
                .prefer_markdown_viewer
                .set_value(true, ctx)
                .unwrap();
            settings
                .open_file_layout
                .set_value(
                    crate::util::openable_file_type::EditorLayout::SplitPane,
                    ctx,
                )
                .unwrap();
        });
        let fixture = tempfile::tempdir().expect("独立文件 fixture");
        let bridge = app.add_singleton_model(LocalControlBridge::new);
        for filename in ["legacy.md", "legacy.rs"] {
            let path = fixture.path().join(filename);
            std::fs::write(&path, "旧文件 {原值}").unwrap();
            let error = bridge
                .update(&mut app, |_, ctx| {
                    super::handle(
                        &None,
                        ActionKind::FileOpen,
                        &serde_json::json!({ "path": path }),
                        &TargetSelector::default(),
                        ctx,
                    )
                })
                .expect_err("内置viewer/editor应在dispatch前拒绝");
            assert_eq!(error.code, ErrorCode::UnsupportedAction);
            assert!(error.message.contains("file.open"));
            assert_eq!(
                workspace.read(&app, |workspace, _| workspace.tab_count()),
                count
            );
        }
        // 后缀只用于使既有 resolver 选中 CodeEditor；目录本身仍应打开普通终端。
        let directory = fixture.path().join("terminal-directory.rs");
        std::fs::create_dir(&directory).unwrap();
        let error = bridge
            .update(&mut app, |_, ctx| {
                super::handle(
                    &None,
                    ActionKind::FileOpen,
                    &serde_json::json!({ "path": directory, "new_tab": false }),
                    &TargetSelector::default(),
                    ctx,
                )
            })
            .expect_err("目录的 CodeEditor(SplitPane) 请求仍不可用");
        assert_eq!(error.code, ErrorCode::UnsupportedAction);
        assert_eq!(
            workspace.read(&app, |workspace, _| workspace.tab_count()),
            count
        );
        let response = bridge
            .update(&mut app, |_, ctx| {
                super::handle(
                    &None,
                    ActionKind::FileOpen,
                    &serde_json::json!({ "path": directory, "new_tab": true }),
                    &TargetSelector::default(),
                    ctx,
                )
            })
            .expect("CodeEditor(NewTab) 目录路径保留普通终端能力");
        assert_eq!(response["action"], "file.open");
        assert_eq!(
            workspace.read(&app, |workspace, _| workspace.tab_count()),
            count + 1
        );
    });
}
