use std::ffi::OsStr;
use std::path::PathBuf;
use std::sync::Arc;

use ai::workspace::WorkspaceMetadata;
use chrono::{Local, Utc};
use cloud_object_persistence::to_cloud_object_permissions;
use diesel::connection::SimpleConnection;
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::vector::Vector2F;
use warp_core::features::FeatureFlag;
use warp_graphql::scalars::time::ServerTimestamp;

use super::{
    app_database_file_path, database_file_path_for_current_scope, database_file_path_for_scope,
    decode_path, deduplicate_events, encode_path, get_all_codebase_index_metadata,
    read_sqlite_data, save_app_state, save_codebase_index_metadata, setup_database, start_writer,
};
use crate::app_state::{
    AppState, CodePaneSnapShot, CodePaneTabSnapshot, LeafContents, LeafSnapshot, PaneNodeSnapshot,
    TabGroupSnapshot, TabSnapshot, TerminalPaneSnapshot, WindowSnapshot,
};
use crate::auth::UserUid;
use crate::cloud_object::{CloudObjectPermissions, Owner};
use crate::code::editor_management::CodeSource;
use crate::notebooks::{CloudNotebook, CloudNotebookModel};
use crate::persistence::model::ObjectPermissions;
use crate::persistence::{
    BlockCompleted, ModelEvent, PersistedDataScope, PersistenceScope, StartedCommandMetadata,
};
use crate::server::ids::{ClientId, ServerId};
use crate::tab::SelectedTabColor;
use crate::terminal::ShellLaunchData;
use crate::terminal::model::block::SerializedBlock;
use crate::terminal::model::session::SessionId;
use crate::themes::theme::AnsiColorIdentifier;
use crate::workspace::tab_group::TabGroupId;
use crate::workspaces::team::{MembershipRole, Team, TeamMember};
use crate::workspaces::user_profiles::UserProfileWithUID;
use crate::workspaces::workspace::Workspace;

#[test]
fn app_scope_database_path_matches_app_database_path() {
    assert_eq!(
        database_file_path_for_scope(&PersistenceScope::App),
        app_database_file_path()
    );
}

#[test]
fn tui_scope_database_path_is_tui_subdirectory_of_app_database_dir() {
    let tui_path = database_file_path_for_scope(&PersistenceScope::Tui);
    let app_path = database_file_path_for_scope(&PersistenceScope::App);

    assert_ne!(tui_path, app_path);
    assert_eq!(
        tui_path,
        warp_core::paths::tui_state_dir().join("warp.sqlite")
    );

    // The TUI database lives in a `tui` subdirectory of the same base
    // directory that holds the GUI database, so the two front-ends never
    // share (or migrate) each other's database.
    let tui_dir = tui_path
        .parent()
        .expect("TUI database path should have a parent");
    assert_eq!(tui_dir.file_name(), Some(OsStr::new("tui")));
    assert_eq!(tui_dir.parent(), app_path.parent());
}

#[test]
fn database_path_for_current_scope_defaults_to_app_scope() {
    // Unit tests never call `persistence::initialize`, so the process-wide
    // scope defaults to `App` and ad-hoc read-only connections resolve to
    // the GUI database. (nextest runs each test in its own process, so no
    // other test can have set the scope.)
    assert_eq!(
        database_file_path_for_current_scope(),
        app_database_file_path()
    );
}

#[test]
fn remote_server_daemon_scope_database_path_uses_identity_data_dir() {
    let path = database_file_path_for_scope(&PersistenceScope::RemoteServerDaemon {
        identity_key: "user@example.com/ssh host".to_string(),
    });
    let expected_data_dir =
        remote_server::setup::remote_server_daemon_data_dir("user@example.com/ssh host");

    assert!(path.is_absolute());
    assert_eq!(
        path,
        PathBuf::from(shellexpand::tilde(&expected_data_dir).into_owned()).join("warp.sqlite")
    );
}

#[test]
fn remote_server_daemon_scope_database_path_handles_empty_identity_key() {
    let path = database_file_path_for_scope(&PersistenceScope::RemoteServerDaemon {
        identity_key: String::new(),
    });
    let expected_data_dir = remote_server::setup::remote_server_daemon_data_dir("");

    assert_eq!(
        path,
        PathBuf::from(shellexpand::tilde(&expected_data_dir).into_owned()).join("warp.sqlite")
    );
}

#[cfg(unix)]
#[test]
fn remote_server_daemon_database_permissions_are_owner_only() {
    use std::fs::Permissions;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let daemon_dir = tempdir.path().join("daemon");
    let database_path = daemon_dir.join("warp.sqlite");

    std::fs::create_dir_all(&daemon_dir).expect("daemon dir should be created");
    std::fs::set_permissions(&daemon_dir, Permissions::from_mode(0o755))
        .expect("daemon dir permissions should be set");
    std::fs::write(&database_path, b"").expect("database file should be created");
    std::fs::set_permissions(&database_path, Permissions::from_mode(0o644))
        .expect("database file permissions should be set");

    super::ensure_owner_only_dir(&daemon_dir).expect("daemon dir should be owner-only");
    super::ensure_owner_only_file(&database_path).expect("database file should be owner-only");

    assert_eq!(daemon_dir.metadata().unwrap().mode() & 0o777, 0o700);
    assert_eq!(database_path.metadata().unwrap().mode() & 0o777, 0o600);
}

fn test_codebase_metadata(path: &str) -> WorkspaceMetadata {
    WorkspaceMetadata {
        path: PathBuf::from(path),
        navigated_ts: Some(Utc::now()),
        modified_ts: None,
        queried_ts: None,
    }
}

#[test]
fn sqlite_read_restores_app_state_and_codebase_metadata() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");

    let app_state = AppState {
        windows: vec![test_terminal_window_snapshot(false)],
        active_window_index: Some(0),
        block_lists: Default::default(),
        running_mcp_servers: Default::default(),
    };
    save_app_state(&mut conn, &app_state).expect("app state should save");

    let metadata = test_codebase_metadata("/tmp/remote-repo");
    save_codebase_index_metadata(&mut conn, metadata.clone())
        .expect("codebase index metadata should save");
    let restored = read_sqlite_data(&mut conn, None, PersistedDataScope::Full)
        .expect("persisted data should load");
    let restored_app_state = restored
        .app_state
        .expect("app state should be present for the full scope");
    assert_eq!(restored_app_state.windows.len(), 1);
    assert_eq!(restored.codebase_indices.len(), 1);
    assert_eq!(restored.codebase_indices[0].path, metadata.path);
}

/// Mirrors `init_db(&PersistenceScope::Tui)` in an isolated tempdir: the TUI
/// database lives in a `tui/` subdirectory, runs the same migrations, and
/// round-trips a write+read using the TUI's `PersistedDataScope`.
#[test]
fn tui_database_in_tui_subdirectory_round_trips_data() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("tui").join("warp.sqlite");
    std::fs::create_dir_all(
        database_path
            .parent()
            .expect("database path should have a parent"),
    )
    .expect("tui subdirectory should be created");
    let mut conn = setup_database(&database_path).expect("database should initialize");

    let metadata = test_codebase_metadata("/tmp/tui-repo");
    save_codebase_index_metadata(&mut conn, metadata.clone())
        .expect("codebase index metadata should save");
    let writer = start_writer(conn, database_path.clone()).expect("writer should start");
    writer
        .sender
        .send(ModelEvent::InsertCommand {
            metadata: StartedCommandMetadata {
                command: "ls".to_owned(),
                start_ts: Some(Local::now()),
                pwd: Some("/tmp/tui-repo".to_owned()),
                shell: Some("zsh".to_owned()),
                username: Some("test-user".to_owned()),
                hostname: Some("test-host".to_owned()),
                session_id: Some(SessionId::from(1)),
                git_branch: None,
                cloud_workflow_id: None,
                workflow_command: None,
                is_agent_executed: false,
            },
        })
        .expect("insert command event should send");
    writer
        .sender
        .send(ModelEvent::UpsertUserProfiles {
            profiles: vec![UserProfileWithUID {
                firebase_uid: UserUid::new("creator-uid"),
                display_name: Some("MCP Creator".to_owned()),
                email: "creator@example.com".to_owned(),
                photo_url: String::new(),
            }],
        })
        .expect("user profile event should send");
    writer
        .sender
        .send(ModelEvent::Terminate)
        .expect("terminate event should send");
    writer.handle.join().expect("writer should terminate");

    let mut conn = setup_database(&database_path).expect("database should reopen");

    let restored = read_sqlite_data(&mut conn, None, PersistedDataScope::TuiFrontend)
        .expect("persisted data should load");
    // The TUI data scope skips GUI session restoration...
    assert!(restored.app_state.is_none());
    // ...but restores command history and shared data like creator profiles and
    // codebase index metadata.
    assert_eq!(restored.command_history.len(), 1);
    assert_eq!(restored.command_history[0].command, "ls");
    assert_eq!(restored.user_profiles.len(), 1);
    assert_eq!(
        restored.user_profiles[0].display_name.as_deref(),
        Some("MCP Creator")
    );
    assert_eq!(restored.codebase_indices.len(), 1);
    assert_eq!(restored.codebase_indices[0].path, metadata.path);
}

#[test]
fn sqlite_writer_reuses_codebase_index_metadata_events() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let conn = setup_database(&database_path).expect("database should initialize");

    let writer = start_writer(conn, database_path.clone()).expect("writer should start");
    let metadata = test_codebase_metadata("/tmp/writer-repo");
    writer
        .sender
        .send(ModelEvent::UpsertCodebaseIndexMetadata {
            index_metadata: Box::new(metadata.clone()),
        })
        .expect("upsert event should send");
    writer
        .sender
        .send(ModelEvent::Terminate)
        .expect("terminate event should send");
    writer.handle.join().expect("writer should terminate");

    let mut conn = setup_database(&database_path).expect("database should reopen");
    let restored = get_all_codebase_index_metadata(&mut conn).expect("metadata should load");
    assert_eq!(restored.len(), 1);
    assert_eq!(restored[0].path, metadata.path);

    let writer = start_writer(conn, database_path.clone()).expect("writer should restart");
    writer
        .sender
        .send(ModelEvent::DeleteCodebaseIndexMetadata {
            repo_path: metadata.path,
        })
        .expect("delete event should send");
    writer
        .sender
        .send(ModelEvent::Terminate)
        .expect("terminate event should send");
    writer.handle.join().expect("writer should terminate");

    let mut conn = setup_database(&database_path).expect("database should reopen");
    let restored = get_all_codebase_index_metadata(&mut conn).expect("metadata should load");
    assert!(restored.is_empty());
}
#[test]
fn test_deduplicate_snapshots() {
    let local_notebook = CloudNotebook::new_local(
        CloudNotebookModel {
            title: "Hello".to_string(),
            data: "World".to_string(),
            ai_document_id: None,
            conversation_id: None,
        },
        Owner::mock_current_user(),
        None,
        ClientId::new(),
    );
    let completed_block_1 = BlockCompleted {
        pane_id: vec![1, 2, 3],
        block: Arc::new(SerializedBlock::default()),
        is_local: true,
    };
    let completed_block_2 = BlockCompleted {
        pane_id: vec![4, 5, 6],
        block: Arc::new(SerializedBlock::default()),
        is_local: true,
    };
    let snapshot_1 = AppState {
        active_window_index: Some(1),
        block_lists: Default::default(),
        windows: Default::default(),
        running_mcp_servers: Default::default(),
    };
    let snapshot_2 = AppState {
        active_window_index: Some(2),
        block_lists: Default::default(),
        windows: Default::default(),
        running_mcp_servers: Default::default(),
    };
    let snapshot_3 = AppState {
        active_window_index: Some(3),
        block_lists: Default::default(),
        windows: Default::default(),
        running_mcp_servers: Default::default(),
    };

    let original_events = vec![
        ModelEvent::UpsertNotebook {
            notebook: local_notebook.clone(),
        },
        ModelEvent::Snapshot(snapshot_1.clone()),
        ModelEvent::SaveBlock(completed_block_1.clone()),
        ModelEvent::Snapshot(snapshot_2.clone()),
        ModelEvent::SaveBlock(completed_block_2.clone()),
        ModelEvent::Snapshot(snapshot_3.clone()),
        ModelEvent::UpsertNotebook {
            notebook: local_notebook.clone(),
        },
    ];

    let filtered_events = deduplicate_events(original_events);
    assert_eq!(filtered_events.len(), 5);

    assert!(matches!(
        &filtered_events[0],
        &ModelEvent::UpsertNotebook { .. }
    ));
    // The first snapshot should have been filtered out.
    assert!(matches!(&filtered_events[1], &ModelEvent::SaveBlock(_)));
    // The second snapshot should have been filtered out.
    assert!(matches!(&filtered_events[2], &ModelEvent::SaveBlock(_)));
    // The third snapshot should be preserved.
    match &filtered_events[3] {
        ModelEvent::Snapshot(snapshot) => assert_eq!(snapshot, &snapshot_3),
        other => panic!("Expected ModelEvent::Snapshot, got {other:?}"),
    }
    assert!(matches!(
        &filtered_events[4],
        &ModelEvent::UpsertNotebook { .. }
    ));
}

#[test]
fn test_deduplicate_no_snapshots() {
    let original_events = vec![ModelEvent::SaveBlock(BlockCompleted {
        pane_id: vec![1, 2, 3],
        block: Default::default(),
        is_local: true,
    })];
    let filtered_events = deduplicate_events(original_events);
    assert_eq!(filtered_events.len(), 1);
    assert!(matches!(&filtered_events[0], &ModelEvent::SaveBlock(_)));
}

fn test_terminal_window_snapshot(vertical_tabs_panel_open: bool) -> WindowSnapshot {
    WindowSnapshot {
        tabs: vec![TabSnapshot {
            custom_title: None,
            root: PaneNodeSnapshot::Leaf(LeafSnapshot {
                is_focused: true,
                custom_vertical_tabs_title: None,
                contents: LeafContents::Terminal(TerminalPaneSnapshot {
                    uuid: vec![u8::from(vertical_tabs_panel_open) + 1],
                    cwd: Some("/tmp".to_string()),
                    shell_launch_data: Some(ShellLaunchData::Executable {
                        executable_path: PathBuf::from("/bin/zsh"),
                        shell_type: crate::terminal::shell::ShellType::Zsh,
                    }),
                    is_active: true,
                    is_read_only: false,
                    input_config: None,
                    llm_model_override: None,
                    active_profile_id: None,
                    conversation_ids_to_restore: vec![],
                    active_conversation_id: None,
                }),
            }),
            default_directory_color: None,
            selected_color: SelectedTabColor::default(),
            left_panel: None,
            right_panel: None,
            group_id: None,
            pinned: false,
        }],
        active_tab_index: 0,
        team_uid: None,
        bounds: None,
        fullscreen_state: Default::default(),
        quake_mode: false,
        universal_search_width: None,
        warp_ai_width: None,
        voltron_width: None,
        warp_drive_index_width: None,
        left_panel_open: false,
        vertical_tabs_panel_open,
        left_panel_width: None,
        right_panel_width: None,
        agent_management_filters: None,
        tab_groups: vec![],
    }
}

#[test]
fn test_sqlite_round_trips_vertical_tabs_panel_open() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");

    let app_state = AppState {
        windows: vec![
            test_terminal_window_snapshot(false),
            test_terminal_window_snapshot(true),
        ],
        active_window_index: Some(1),
        block_lists: Default::default(),
        running_mcp_servers: Default::default(),
    };

    save_app_state(&mut conn, &app_state).expect("app state should save");

    let restored = read_sqlite_data(&mut conn, None, PersistedDataScope::Full)
        .expect("app state should load")
        .app_state
        .expect("app state should be present for the full scope");

    assert_eq!(restored.active_window_index, Some(1));
    assert_eq!(
        restored
            .windows
            .iter()
            .map(|window| window.vertical_tabs_panel_open)
            .collect::<Vec<_>>(),
        vec![false, true]
    );
}

#[test]
fn test_sqlite_round_trips_window_team_uid() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");
    let team_uid = ServerId::from(123);
    let mut assigned_window = test_terminal_window_snapshot(false);
    assigned_window.team_uid = Some(team_uid);

    let app_state = AppState {
        windows: vec![assigned_window, test_terminal_window_snapshot(true)],
        active_window_index: Some(0),
        block_lists: Default::default(),
        running_mcp_servers: Default::default(),
    };

    save_app_state(&mut conn, &app_state).expect("app state should save");

    let restored = read_sqlite_data(&mut conn, None, PersistedDataScope::Full)
        .expect("app state should load")
        .app_state
        .expect("app state should be present for the full scope");

    assert_eq!(restored.windows[0].team_uid, Some(team_uid));
    assert_eq!(restored.windows[1].team_uid, None);
}

#[test]
fn test_sqlite_round_trips_custom_vertical_tabs_title() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");

    let app_state = AppState {
        windows: vec![WindowSnapshot {
            tabs: vec![TabSnapshot {
                custom_title: None,
                root: PaneNodeSnapshot::Leaf(LeafSnapshot {
                    is_focused: true,
                    custom_vertical_tabs_title: Some("Production API".to_string()),
                    contents: LeafContents::Terminal(TerminalPaneSnapshot {
                        uuid: vec![42],
                        cwd: Some("/tmp".to_string()),
                        shell_launch_data: Some(ShellLaunchData::Executable {
                            executable_path: PathBuf::from("/bin/zsh"),
                            shell_type: crate::terminal::shell::ShellType::Zsh,
                        }),
                        is_active: true,
                        is_read_only: false,
                        input_config: None,
                        llm_model_override: None,
                        active_profile_id: None,
                        conversation_ids_to_restore: vec![],
                        active_conversation_id: None,
                    }),
                }),
                default_directory_color: None,
                selected_color: SelectedTabColor::default(),
                left_panel: None,
                right_panel: None,
                group_id: None,
                pinned: false,
            }],
            active_tab_index: 0,
            team_uid: None,
            bounds: None,
            fullscreen_state: Default::default(),
            quake_mode: false,
            universal_search_width: None,
            warp_ai_width: None,
            voltron_width: None,
            warp_drive_index_width: None,
            left_panel_open: false,
            vertical_tabs_panel_open: false,
            left_panel_width: None,
            right_panel_width: None,
            agent_management_filters: None,
            tab_groups: vec![],
        }],
        active_window_index: Some(0),
        block_lists: Default::default(),
        running_mcp_servers: Default::default(),
    };

    save_app_state(&mut conn, &app_state).expect("app state should save");

    let restored = read_sqlite_data(&mut conn, None, PersistedDataScope::Full)
        .expect("app state should load")
        .app_state
        .expect("app state should be present for the full scope");

    let PaneNodeSnapshot::Leaf(LeafSnapshot {
        custom_vertical_tabs_title,
        ..
    }) = &restored.windows[0].tabs[0].root
    else {
        panic!("Expected terminal pane leaf");
    };
    assert_eq!(
        custom_vertical_tabs_title.as_deref(),
        Some("Production API")
    );
}

#[test]
fn test_sqlite_round_trips_code_pane_with_multiple_tabs() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");

    let app_state = AppState {
        windows: vec![WindowSnapshot {
            tabs: vec![TabSnapshot {
                custom_title: None,
                root: PaneNodeSnapshot::Leaf(LeafSnapshot {
                    is_focused: true,
                    custom_vertical_tabs_title: None,
                    contents: LeafContents::Code(CodePaneSnapShot::Local {
                        tabs: vec![
                            CodePaneTabSnapshot {
                                path: Some(PathBuf::from("/tmp/main.rs")),
                            },
                            CodePaneTabSnapshot {
                                path: Some(PathBuf::from("/tmp/lib.rs")),
                            },
                            CodePaneTabSnapshot { path: None },
                        ],
                        active_tab_index: 1,
                        source: Some(CodeSource::FileTree {
                            location: crate::code::buffer_location::LocalOrRemotePath::Local(
                                PathBuf::from("/tmp/main.rs"),
                            ),
                        }),
                    }),
                }),
                default_directory_color: None,
                selected_color: SelectedTabColor::default(),
                left_panel: None,
                right_panel: None,
                group_id: None,
                pinned: false,
            }],
            active_tab_index: 0,
            team_uid: None,
            bounds: None,
            fullscreen_state: Default::default(),
            quake_mode: false,
            universal_search_width: None,
            warp_ai_width: None,
            voltron_width: None,
            warp_drive_index_width: None,
            left_panel_open: false,
            vertical_tabs_panel_open: false,
            left_panel_width: None,
            right_panel_width: None,
            agent_management_filters: None,
            tab_groups: vec![],
        }],
        active_window_index: Some(0),
        block_lists: Default::default(),
        running_mcp_servers: Default::default(),
    };

    save_app_state(&mut conn, &app_state).expect("app state should save");

    let restored = read_sqlite_data(&mut conn, None, PersistedDataScope::Full)
        .expect("app state should load")
        .app_state
        .expect("app state should be present for the full scope");

    assert_eq!(restored.windows.len(), 1);
    let restored_tab = &restored.windows[0].tabs[0];
    let PaneNodeSnapshot::Leaf(LeafSnapshot {
        contents:
            LeafContents::Code(CodePaneSnapShot::Local {
                tabs,
                active_tab_index,
                source,
            }),
        ..
    }) = &restored_tab.root
    else {
        panic!("Expected code pane leaf");
    };

    assert_eq!(tabs.len(), 3);
    assert_eq!(*active_tab_index, 1);
    assert_eq!(tabs[0].path, Some(PathBuf::from("/tmp/main.rs")));
    assert_eq!(tabs[1].path, Some(PathBuf::from("/tmp/lib.rs")));
    assert_eq!(tabs[2].path, None);
    assert!(matches!(source, Some(CodeSource::FileTree { .. })));
}

/// Verifies that a tab group and its membership round-trip through save/restore.
#[test]
fn test_sqlite_round_trips_tab_groups() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");

    let group_id = TabGroupId::new();
    let tab_in_group = TabSnapshot {
        custom_title: None,
        root: PaneNodeSnapshot::Leaf(LeafSnapshot {
            is_focused: true,
            custom_vertical_tabs_title: None,
            contents: LeafContents::Terminal(TerminalPaneSnapshot {
                uuid: vec![1],
                cwd: Some("/tmp/grouped".to_string()),
                shell_launch_data: Some(ShellLaunchData::Executable {
                    executable_path: PathBuf::from("/bin/zsh"),
                    shell_type: crate::terminal::shell::ShellType::Zsh,
                }),
                is_active: true,
                is_read_only: false,
                input_config: None,
                llm_model_override: None,
                active_profile_id: None,
                conversation_ids_to_restore: vec![],
                active_conversation_id: None,
            }),
        }),
        default_directory_color: None,
        selected_color: SelectedTabColor::default(),
        left_panel: None,
        right_panel: None,
        group_id: Some(group_id),
        pinned: false,
    };
    let tab_outside_group = TabSnapshot {
        custom_title: None,
        root: PaneNodeSnapshot::Leaf(LeafSnapshot {
            is_focused: false,
            custom_vertical_tabs_title: None,
            contents: LeafContents::Terminal(TerminalPaneSnapshot {
                uuid: vec![2],
                cwd: Some("/tmp/ungrouped".to_string()),
                shell_launch_data: Some(ShellLaunchData::Executable {
                    executable_path: PathBuf::from("/bin/zsh"),
                    shell_type: crate::terminal::shell::ShellType::Zsh,
                }),
                is_active: false,
                is_read_only: false,
                input_config: None,
                llm_model_override: None,
                active_profile_id: None,
                conversation_ids_to_restore: vec![],
                active_conversation_id: None,
            }),
        }),
        default_directory_color: None,
        selected_color: SelectedTabColor::default(),
        left_panel: None,
        right_panel: None,
        group_id: None,
        pinned: false,
    };

    let app_state = AppState {
        windows: vec![WindowSnapshot {
            tabs: vec![tab_in_group, tab_outside_group],
            active_tab_index: 0,
            team_uid: None,
            bounds: None,
            fullscreen_state: Default::default(),
            quake_mode: false,
            universal_search_width: None,
            warp_ai_width: None,
            voltron_width: None,
            warp_drive_index_width: None,
            left_panel_open: false,
            vertical_tabs_panel_open: false,
            left_panel_width: None,
            right_panel_width: None,
            agent_management_filters: None,
            tab_groups: vec![TabGroupSnapshot {
                id: group_id,
                name: Some("Backend".to_string()),
                color: SelectedTabColor::Color(AnsiColorIdentifier::Blue),
                collapsed: true,
                pinned: false,
            }],
        }],
        active_window_index: Some(0),
        block_lists: Default::default(),
        running_mcp_servers: Default::default(),
    };

    save_app_state(&mut conn, &app_state).expect("app state should save");

    let restored = read_sqlite_data(&mut conn, None, PersistedDataScope::Full)
        .expect("app state should load")
        .app_state
        .expect("app state should be present for the full scope");

    assert_eq!(restored.windows.len(), 1);
    let restored_window = &restored.windows[0];
    assert_eq!(restored_window.tab_groups.len(), 1);
    let restored_group = &restored_window.tab_groups[0];
    assert_eq!(restored_group.name.as_deref(), Some("Backend"));
    assert_eq!(
        restored_group.color,
        SelectedTabColor::Color(AnsiColorIdentifier::Blue)
    );
    assert!(restored_group.collapsed);

    // The in-memory `TabGroupId` is minted fresh on restore, so we check that
    // the grouped tab points at the restored group, and the ungrouped tab
    // remains ungrouped.
    assert_eq!(restored_window.tabs.len(), 2);
    assert_eq!(restored_window.tabs[0].group_id, Some(restored_group.id));
    assert_eq!(restored_window.tabs[1].group_id, None);
}

/// Verifies that the `pinned` flag on tabs and tab groups round-trips through
/// save/restore so the user's pinned layout survives an app restart.
#[test]
fn test_sqlite_round_trips_pinned_state() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");

    let pinned_group_id = TabGroupId::new();
    let unpinned_group_id = TabGroupId::new();

    let pinned_tab = TabSnapshot {
        custom_title: None,
        root: PaneNodeSnapshot::Leaf(LeafSnapshot {
            is_focused: true,
            custom_vertical_tabs_title: None,
            contents: LeafContents::Terminal(TerminalPaneSnapshot {
                uuid: vec![10],
                cwd: Some("/tmp/pinned".to_string()),
                shell_launch_data: Some(ShellLaunchData::Executable {
                    executable_path: PathBuf::from("/bin/zsh"),
                    shell_type: crate::terminal::shell::ShellType::Zsh,
                }),
                is_active: true,
                is_read_only: false,
                input_config: None,
                llm_model_override: None,
                active_profile_id: None,
                conversation_ids_to_restore: vec![],
                active_conversation_id: None,
            }),
        }),
        default_directory_color: None,
        selected_color: SelectedTabColor::default(),
        left_panel: None,
        right_panel: None,
        group_id: None,
        pinned: true,
    };
    let unpinned_tab = TabSnapshot {
        custom_title: None,
        root: PaneNodeSnapshot::Leaf(LeafSnapshot {
            is_focused: false,
            custom_vertical_tabs_title: None,
            contents: LeafContents::Terminal(TerminalPaneSnapshot {
                uuid: vec![11],
                cwd: Some("/tmp/unpinned".to_string()),
                shell_launch_data: Some(ShellLaunchData::Executable {
                    executable_path: PathBuf::from("/bin/zsh"),
                    shell_type: crate::terminal::shell::ShellType::Zsh,
                }),
                is_active: false,
                is_read_only: false,
                input_config: None,
                llm_model_override: None,
                active_profile_id: None,
                conversation_ids_to_restore: vec![],
                active_conversation_id: None,
            }),
        }),
        default_directory_color: None,
        selected_color: SelectedTabColor::default(),
        left_panel: None,
        right_panel: None,
        group_id: Some(unpinned_group_id),
        pinned: false,
    };
    let tab_in_pinned_group = TabSnapshot {
        custom_title: None,
        root: PaneNodeSnapshot::Leaf(LeafSnapshot {
            is_focused: false,
            custom_vertical_tabs_title: None,
            contents: LeafContents::Terminal(TerminalPaneSnapshot {
                uuid: vec![12],
                cwd: Some("/tmp/pinned-group".to_string()),
                shell_launch_data: Some(ShellLaunchData::Executable {
                    executable_path: PathBuf::from("/bin/zsh"),
                    shell_type: crate::terminal::shell::ShellType::Zsh,
                }),
                is_active: false,
                is_read_only: false,
                input_config: None,
                llm_model_override: None,
                active_profile_id: None,
                conversation_ids_to_restore: vec![],
                active_conversation_id: None,
            }),
        }),
        default_directory_color: None,
        selected_color: SelectedTabColor::default(),
        left_panel: None,
        right_panel: None,
        group_id: Some(pinned_group_id),
        pinned: false,
    };

    let app_state = AppState {
        windows: vec![WindowSnapshot {
            tabs: vec![pinned_tab, tab_in_pinned_group, unpinned_tab],
            active_tab_index: 0,
            team_uid: None,
            bounds: None,
            fullscreen_state: Default::default(),
            quake_mode: false,
            universal_search_width: None,
            warp_ai_width: None,
            voltron_width: None,
            warp_drive_index_width: None,
            left_panel_open: false,
            vertical_tabs_panel_open: false,
            left_panel_width: None,
            right_panel_width: None,
            agent_management_filters: None,
            tab_groups: vec![
                TabGroupSnapshot {
                    id: pinned_group_id,
                    name: Some("Pinned".to_string()),
                    color: SelectedTabColor::default(),
                    collapsed: false,
                    pinned: true,
                },
                TabGroupSnapshot {
                    id: unpinned_group_id,
                    name: Some("Loose".to_string()),
                    color: SelectedTabColor::default(),
                    collapsed: false,
                    pinned: false,
                },
            ],
        }],
        active_window_index: Some(0),
        block_lists: Default::default(),
        running_mcp_servers: Default::default(),
    };

    save_app_state(&mut conn, &app_state).expect("app state should save");

    let restored = read_sqlite_data(&mut conn, None, PersistedDataScope::Full)
        .expect("app state should load")
        .app_state
        .expect("app state should be present for the full scope");

    assert_eq!(restored.windows.len(), 1);
    let restored_window = &restored.windows[0];

    // Tabs come back in insertion order; pinned flag should match what we saved.
    assert_eq!(restored_window.tabs.len(), 3);
    assert!(restored_window.tabs[0].pinned);
    assert!(!restored_window.tabs[1].pinned);
    assert!(!restored_window.tabs[2].pinned);

    // Both groups round-trip with their pinned state preserved. Group ids are
    // minted fresh on restore, so we look them up by name.
    assert_eq!(restored_window.tab_groups.len(), 2);
    let restored_pinned_group = restored_window
        .tab_groups
        .iter()
        .find(|group| group.name.as_deref() == Some("Pinned"))
        .expect("pinned group should restore");
    let restored_loose_group = restored_window
        .tab_groups
        .iter()
        .find(|group| group.name.as_deref() == Some("Loose"))
        .expect("unpinned group should restore");
    assert!(restored_pinned_group.pinned);
    assert!(!restored_loose_group.pinned);
}

fn assert_encode_then_decode_preserves_original_path(original_path: PathBuf) {
    let bytes = encode_path(original_path.clone());
    let decoded_path = decode_path(bytes);
    assert_eq!(original_path, decoded_path);
}

/// Test that a local path can be encoded and decoded. We use this when persisting a local
/// file path for notebooks in sqlite. We need this test because Windows `OsString`s are
/// often arbitrary sequences of 16-bit values, unlike Unix which uses sequences of 8-bit
/// values (bytes). Since `diesel::sql_types::Binary` deals with sequences of bytes (`u8`)
/// we need to perform special casting on `OsString`s on Windows.
#[test]
fn test_path_encode_decode() {
    // Empty path
    assert_encode_then_decode_preserves_original_path(PathBuf::new());

    // Windows-style paths
    assert_encode_then_decode_preserves_original_path(PathBuf::from(r"C:\windows\system32.dll"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from("c:temp"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from(r"\temp"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from(r"\temp\emoji\🙈.txt"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from(r"\temp\ñoñàscii\temp.txt"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from(r"\temp\hindi\हिन्दी"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from(r"\temp\cjk\狗没有耐心"));

    // Unix-style paths
    assert_encode_then_decode_preserves_original_path(PathBuf::from(
        "/home/persistence/example.sql",
    ));
    assert_encode_then_decode_preserves_original_path(PathBuf::from("./database/log.txt"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from("/temp/emoji/🙈.txt"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from("/temp/ñoñàscii/temp.txt"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from("/temp/hindi/हिन्दी"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from("/temp/cjk/狗没有耐心"));
}

#[test]
fn test_deserialize_corrupted_guests() {
    let _ = FeatureFlag::SharedWithMe.override_enabled(true);
    // Use a hardcoded timestamp to ensure this test works on systems with more-than-microsecond
    // precision.
    let permissions_ts_micros = 123456;
    let permissions_ts =
        ServerTimestamp::from_unix_timestamp_micros(permissions_ts_micros).unwrap();

    let db_permissions = ObjectPermissions {
        id: 42,
        object_metadata_id: 10,
        subject_type: "TEAM".to_string(),
        subject_id: Some("7".to_string()),
        subject_uid: "team_uid12345678912345".to_string(),
        permissions_last_updated_at: Some(permissions_ts_micros),
        // This is not a valid set of encoded object guests.
        object_guests: Some(vec![1, 2, 3]),
        anyone_with_link_access_level: None,
        anyone_with_link_source: None,
    };

    // The overall permissions should successfully convert, minus the object guests.
    let cloud_permissions = to_cloud_object_permissions(&db_permissions, None);
    assert_eq!(
        cloud_permissions,
        Some(CloudObjectPermissions {
            owner: Owner::Team {
                team_uid: crate::server::ids::ServerId::from_string_lossy("team_uid12345678912345"),
            },
            permissions_last_updated_ts: Some(permissions_ts),
            anyone_with_link: None,
            guests: vec![],
        })
    );
}

// Regression: GH#10083. The macOS green-tile button could leave a 1px-wide
// window bound in `AppContext::window_bounds`, which previously round-tripped
// through SQLite and restored as an unusable 1px sliver. Bounds below the
// platform minimum window size must be dropped on save.
#[test]
fn test_sqlite_drops_too_small_bounds_on_save() {
    use diesel::prelude::*;

    use crate::persistence::schema::windows;

    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");

    let mut snapshot = test_terminal_window_snapshot(false);
    snapshot.bounds = Some(RectF::new(
        Vector2F::new(0.0, -1410.0),
        Vector2F::new(1.0, 1410.0),
    ));

    let app_state = AppState {
        windows: vec![snapshot],
        active_window_index: Some(0),
        block_lists: Default::default(),
        running_mcp_servers: Default::default(),
    };

    save_app_state(&mut conn, &app_state).expect("app state should save");

    // Query the row directly so the assertion isolates the save guard and is
    // not masked by the read-side guard in `read_sqlite_data`.
    let row: (Option<f32>, Option<f32>, Option<f32>, Option<f32>) = windows::dsl::windows
        .select((
            windows::columns::window_width,
            windows::columns::window_height,
            windows::columns::origin_x,
            windows::columns::origin_y,
        ))
        .first(&mut conn)
        .expect("a windows row should have been inserted");

    assert_eq!(
        row,
        (None, None, None, None),
        "save-path guard must persist NULL bound columns for sub-minimum geometry"
    );
}

// Regression: GH#10083. Users whose warp.sqlite already contains a 1px row
// (because they hit the bug on an earlier build) must still recover to default
// geometry on next launch rather than restoring the sliver.
#[test]
fn test_sqlite_drops_too_small_bounds_on_read() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let mut conn = setup_database(&database_path).expect("database should initialize");

    // Save with no bounds so a row exists, then corrupt it directly to bypass
    // the save-path guard and simulate a pre-existing bad row.
    let app_state = AppState {
        windows: vec![test_terminal_window_snapshot(false)],
        active_window_index: Some(0),
        block_lists: Default::default(),
        running_mcp_servers: Default::default(),
    };
    save_app_state(&mut conn, &app_state).expect("app state should save");

    conn.batch_execute(
        "UPDATE windows \
         SET window_width = 1.0, window_height = 1410.0, \
             origin_x = 0.0, origin_y = -1410.0",
    )
    .expect("corrupting update should succeed");

    let restored = read_sqlite_data(&mut conn, None, PersistedDataScope::Full)
        .expect("app state should load")
        .app_state
        .expect("app state should be present for the full scope");

    assert_eq!(restored.windows.len(), 1);
    assert!(
        restored.windows[0].bounds.is_none(),
        "tiny persisted bounds must be discarded on read so users recover from a corrupt DB"
    );
}

#[test]
fn team_member_is_disabled_round_trips_through_sqlite_cache() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let conn = setup_database(&database_path).expect("database should initialize");

    let team = Team::from_local_cache(
        ServerId::from_string_lossy(format!("{:0>22}", "team")),
        "Team".to_string(),
        None,
        None,
        Some(vec![
            TeamMember {
                uid: UserUid::new("active-user"),
                email: "active@example.com".to_string(),
                role: MembershipRole::User,
                is_disabled: false,
            },
            TeamMember {
                uid: UserUid::new("disabled-user"),
                email: "disabled@example.com".to_string(),
                role: MembershipRole::User,
                is_disabled: true,
            },
        ]),
        None,
    );
    let workspace = Workspace::from_local_cache(
        format!("{:0>22}", "workspace").into(),
        "Workspace".to_string(),
        Some(vec![team]),
        None,
    );

    let writer = start_writer(conn, database_path.clone()).expect("writer should start");
    writer
        .sender
        .send(ModelEvent::UpsertWorkspaces {
            workspaces: vec![workspace],
        })
        .expect("upsert workspaces event should send");
    writer
        .sender
        .send(ModelEvent::Terminate)
        .expect("terminate event should send");
    writer.handle.join().expect("writer should terminate");

    let mut conn = setup_database(&database_path).expect("database should reopen");
    let restored = read_sqlite_data(&mut conn, None, PersistedDataScope::Full)
        .expect("persisted data should load");

    let members = &restored.workspaces[0].teams[0].members;
    let active_member = members
        .iter()
        .find(|member| member.email == "active@example.com")
        .expect("active member should be present");
    let disabled_member = members
        .iter()
        .find(|member| member.email == "disabled@example.com")
        .expect("disabled member should be present");
    assert!(!active_member.is_disabled);
    assert!(disabled_member.is_disabled);
}

#[test]
fn local_terminal_logged_out_restore_preserves_legacy_tabs_and_history() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("legacy-account.sqlite");
    let mut conn = setup_database(&database_path).expect("fixture database should initialize");
    let mut window = test_terminal_window_snapshot(true);
    window.tabs.clear();
    for index in 0..3 {
        let mut tab = test_terminal_window_snapshot(true).tabs.remove(0);
        tab.custom_title = Some(format!("旧标签 {index} {{原值}}"));
        tab.pinned = index == 0;
        if let PaneNodeSnapshot::Leaf(LeafSnapshot {
            contents: LeafContents::Terminal(terminal),
            ..
        }) = &mut tab.root
        {
            terminal.uuid = vec![index + 1];
            terminal.cwd = Some(format!("/tmp/旧目录 {index} {{原值}}"));
        }
        window.tabs.push(tab);
    }
    let app_state = AppState {
        windows: vec![window],
        active_window_index: Some(0),
        block_lists: Default::default(),
        running_mcp_servers: Default::default(),
    };
    save_app_state(&mut conn, &app_state).expect("legacy tabs should save");
    let command = "printf '旧历史 {用户输入}'";
    super::insert_command(
        &mut conn,
        StartedCommandMetadata {
            command: command.to_owned(),
            start_ts: Some(Local::now()),
            pwd: Some("/tmp/旧目录 {原值}".to_owned()),
            shell: Some("zsh".to_owned()),
            username: Some("fixture-user".to_owned()),
            hostname: Some("fixture-host".to_owned()),
            session_id: Some(SessionId::from(1)),
            git_branch: Some("feature/旧分支".to_owned()),
            cloud_workflow_id: None,
            workflow_command: None,
            is_agent_executed: false,
        },
    )
    .expect("legacy command should save");

    // 同一 fixture 先按旧账号读取，再按真实 None 读取；不清库、不迁移身份。
    for user in [Some(UserUid::new("l1-fixture-user")), None] {
        let restored = read_sqlite_data(&mut conn, user, PersistedDataScope::Full)
            .expect("legacy data should load with and without an account");
        let state = restored
            .app_state
            .expect("GUI sessions should remain available");
        assert_eq!(state.windows.len(), 1);
        assert_eq!(state.windows[0].tabs.len(), 3);
        assert!(state.windows[0].tabs[0].pinned);
        for (index, tab) in state.windows[0].tabs.iter().enumerate() {
            assert_eq!(
                tab.custom_title.as_deref(),
                Some(format!("旧标签 {index} {{原值}}").as_str())
            );
        }
        assert_eq!(restored.command_history.len(), 1);
        assert_eq!(restored.command_history[0].command, command);
        assert_eq!(
            restored.command_history[0].pwd.as_deref(),
            Some("/tmp/旧目录 {原值}")
        );
        assert_eq!(
            restored.command_history[0].git_branch.as_deref(),
            Some("feature/旧分支")
        );
    }
}

#[test]
fn local_terminal_legacy_window_and_ai_block_survive_snapshot_and_save_block_round_trip() {
    use crate::ai::agent::conversation::AIConversationId;
    use crate::app_state::{BranchSnapshot, PaneFlex, SplitDirection};
    use crate::terminal::model::block::SerializedAgentViewVisibility;
    use crate::terminal::model::test_utils::TestBlockListBuilder;

    let tempdir = tempfile::tempdir().expect("temp fixture directory");
    let path = tempdir.path().join("l2-legacy.sqlite");
    let mut conn = setup_database(&path).expect("fixture schema");
    let pane_uuid = vec![42; 16];
    let conversation_id = AIConversationId::new();
    let mut terminal_window = test_terminal_window_snapshot(true);
    let mut terminal_leaf = match terminal_window.tabs[0].root.clone() {
        PaneNodeSnapshot::Leaf(leaf) => leaf,
        _ => panic!("terminal fixture is a leaf"),
    };
    if let LeafContents::Terminal(terminal) = &mut terminal_leaf.contents {
        terminal.uuid = pane_uuid.clone();
        terminal.input_config = Some(crate::ai::blocklist::InputConfig {
            input_type: crate::ai::blocklist::InputType::AI,
            is_locked: true,
        });
        terminal.llm_model_override = Some("legacy-model-opaque-value".to_owned());
        terminal.active_profile_id = Some(crate::server::ids::SyncId::ClientId(ClientId::new()));
        terminal.conversation_ids_to_restore = vec![conversation_id];
        terminal.active_conversation_id = Some(conversation_id);
    }
    let mut children = vec![(PaneFlex(2.), PaneNodeSnapshot::Leaf(terminal_leaf))];
    // 旧 SQLite 不存设置搜索词，两个编辑器共用同一 kind；按实际磁盘描述构造 fixture。
    children.extend(
        crate::app_state::retired_pane_test_fixtures()
            .into_iter()
            .filter(LeafContents::is_persisted)
            .map(|contents| match contents {
                LeafContents::CustomRouterEditor => LeafContents::ExecutionProfileEditor,
                LeafContents::Settings(crate::app_state::SettingsPaneSnapshot::Local {
                    current_page,
                    ..
                }) => LeafContents::Settings(crate::app_state::SettingsPaneSnapshot::Local {
                    current_page,
                    search_query: None,
                }),
                other => other,
            })
            .enumerate()
            .map(|(index, contents)| {
                (
                    PaneFlex(1.),
                    PaneNodeSnapshot::Leaf(LeafSnapshot {
                        is_focused: false,
                        custom_vertical_tabs_title: Some(format!("旧面板 {index} {{原值}}")),
                        contents,
                    }),
                )
            }),
    );
    terminal_window.tabs[0].root = PaneNodeSnapshot::Branch(BranchSnapshot {
        direction: SplitDirection::Horizontal,
        children,
    });
    terminal_window.tabs[0].custom_title = Some("旧窗口标签 {原值}".to_owned());
    terminal_window.tabs[0].pinned = true;
    let state = AppState {
        windows: vec![terminal_window],
        active_window_index: Some(0),
        block_lists: Default::default(),
        running_mcp_servers: vec![uuid::Uuid::new_v4()],
    };
    save_app_state(&mut conn, &state).expect("write legacy window fixture");
    let mut block = SerializedBlock::new_for_test("printf '旧命令 {原样}'".into(), "旧输出".into());
    block.ai_metadata = Some(format!(
        "  {}  ",
        serde_json::json!({
            "requested_command_action_id": null, "conversation_id": conversation_id,
            "subagent_task_id": null, "long_running_control_state": null,
            "has_agent_written_to_block": true, "legacy_unknown_field": ["保留", 17],
        })
    ));
    block.agent_view_visibility = Some(SerializedAgentViewVisibility::Agent {
        origin_conversation_id: conversation_id,
        pending_other_conversation_ids: Default::default(),
        other_conversation_ids: Default::default(),
    });
    super::save_block(&mut conn, pane_uuid.clone(), &block, true)
        .expect("write legacy block fixture");
    let loaded = read_sqlite_data(&mut conn, None, PersistedDataScope::LocalGui)
        .expect("load local GUI without account");
    let loaded_state = loaded.app_state.expect("legacy GUI window remains");
    assert_eq!(loaded_state.windows, state.windows);
    assert_eq!(loaded_state.running_mcp_servers, state.running_mcp_servers);
    let originals = loaded_state
        .block_lists
        .get(&crate::app_state::PaneUuid(pane_uuid.clone()))
        .unwrap();
    let mut blocks = TestBlockListBuilder::new()
        .with_restored_blocks(originals)
        .build();
    blocks.preserve_restored_ai_data(originals);
    let restored = blocks
        .block_at(blocks.block_index_for_id(&block.id).unwrap())
        .unwrap();
    let resaved_block = SerializedBlock::from(restored);
    assert_eq!(resaved_block.ai_metadata, block.ai_metadata);
    assert_eq!(
        resaved_block.agent_view_visibility,
        block.agent_view_visibility
    );
    let writer = start_writer(conn, path.clone()).expect("fixture writer");
    writer
        .sender
        .send(ModelEvent::SaveBlock(BlockCompleted {
            pane_id: pane_uuid.clone(),
            is_local: true,
            block: Arc::new(resaved_block),
        }))
        .unwrap();
    writer
        .sender
        .send(ModelEvent::Snapshot(loaded_state))
        .unwrap();
    writer.sender.send(ModelEvent::Terminate).unwrap();
    writer.handle.join().unwrap();
    let mut conn = setup_database(&path).expect("reopen fixture");
    let again = read_sqlite_data(&mut conn, None, PersistedDataScope::Full)
        .unwrap()
        .app_state
        .unwrap();
    assert_eq!(again.windows, state.windows);
    assert_eq!(again.running_mcp_servers, state.running_mcp_servers);
    let crate::ai::blocklist::SerializedBlockListItem::Command { block: saved } =
        &again.block_lists[&crate::app_state::PaneUuid(pane_uuid)][0];
    assert_eq!(saved.ai_metadata, block.ai_metadata);
    assert_eq!(saved.agent_view_visibility, block.agent_view_visibility);
    assert_eq!(saved.stylized_command, block.stylized_command);
    assert_eq!(saved.stylized_output, block.stylized_output);
}

fn local_gui_command(command: &str) -> StartedCommandMetadata {
    StartedCommandMetadata {
        command: command.to_owned(),
        start_ts: Some(Local::now()),
        pwd: Some("/tmp/本地项目".to_owned()),
        shell: Some("zsh".to_owned()),
        username: Some("fixture-user".to_owned()),
        hostname: Some("fixture-host".to_owned()),
        session_id: Some(SessionId::from(5)),
        git_branch: Some("feature/local".to_owned()),
        cloud_workflow_id: None,
        workflow_command: None,
        is_agent_executed: false,
    }
}

fn seed_local_gui_fixture(conn: &mut diesel::SqliteConnection) {
    use diesel::RunQueryDsl;
    let mut window = test_terminal_window_snapshot(true);
    window.tabs[0].pinned = true;
    window.tabs[0].custom_title = Some("本地窗口".to_owned());
    let state = AppState {
        windows: vec![window],
        active_window_index: Some(0),
        block_lists: Default::default(),
        running_mcp_servers: vec![uuid::Uuid::new_v4()],
    };
    save_app_state(conn, &state).expect("保存旧会话");
    super::insert_command(conn, local_gui_command("printf '旧历史'")).expect("保存历史");
    save_codebase_index_metadata(conn, test_codebase_metadata("/tmp/旧索引")).expect("保存索引");
    conn.batch_execute("INSERT INTO folders (id, name, is_open, is_warp_pack) VALUES (501, '旧云目录', 1, 0);
        INSERT INTO object_actions (hashed_object_id, action, data, pending) VALUES ('legacy-object', 'legacy-opaque-action', '原样保留', 1);
        INSERT INTO user_profiles (firebase_uid, photo_url, email, display_name) VALUES ('legacy-user', '', 'fixture@example.com', '旧账号');
        INSERT INTO projects (path, added_ts) VALUES ('/tmp/旧项目', '2026-01-01 00:00:00');
        INSERT INTO ignored_suggestions (suggestion, suggestion_type) VALUES ('旧忽略命令', 'shell_command'), ('旧 AI 提示', 'ai_query');")
        .expect("保存未使用的旧记录");
    let data = serde_json::to_string(&crate::persistence::model::AgentConversationData::default())
        .expect("会话数据");
    diesel::sql_query("INSERT INTO agent_conversations (conversation_id, conversation_data, last_modified_at, summary) VALUES (?, ?, '2026-01-01 00:00:00', NULL)")
        .bind::<diesel::sql_types::Text, _>(uuid::Uuid::new_v4().to_string())
        .bind::<diesel::sql_types::Text, _>(data)
        .execute(conn).expect("保存待回填的旧 AI 会话");
}

#[derive(diesel::QueryableByName)]
struct LocalGuiStoredRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    value: String,
}

fn local_gui_excluded_rows(conn: &mut diesel::SqliteConnection) -> Vec<Vec<String>> {
    use diesel::RunQueryDsl;
    [
        "SELECT quote(id)||quote(name)||quote(is_open)||quote(is_warp_pack) AS value FROM folders ORDER BY id",
        "SELECT quote(id)||quote(hashed_object_id)||quote(action)||quote(data)||quote(pending) AS value FROM object_actions ORDER BY id",
        "SELECT quote(id)||quote(conversation_id)||quote(conversation_data)||quote(summary)||quote(last_modified_at) AS value FROM agent_conversations ORDER BY id",
        "SELECT quote(firebase_uid)||quote(photo_url)||quote(email)||quote(display_name) AS value FROM user_profiles ORDER BY firebase_uid",
        "SELECT quote(id)||quote(repo_path)||quote(navigated_ts)||quote(modified_ts)||quote(queried_ts) AS value FROM workspace_metadata ORDER BY id",
        "SELECT quote(path)||quote(added_ts)||quote(last_opened_ts) AS value FROM projects ORDER BY path",
    ].into_iter().map(|sql| diesel::sql_query(sql).load::<LocalGuiStoredRow>(conn)
        .expect("读取旧记录原始字段").into_iter().map(|row| row.value).collect()).collect()
}

#[test]
fn local_gui_reads_terminal_data_without_cloud_and_ai_control_queries() {
    use diesel::Connection;
    use diesel::connection::InstrumentationEvent;
    let directory = tempfile::tempdir().expect("临时目录");
    let mut conn = setup_database(&directory.path().join("scope.sqlite")).expect("测试数据库");
    seed_local_gui_fixture(&mut conn);
    let queries = Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorded = queries.clone();
    conn.set_instrumentation(move |event: InstrumentationEvent<'_>| {
        if let InstrumentationEvent::StartQuery { query, .. } = event {
            recorded.lock().expect("查询记录锁").push(query.to_string());
        }
    });
    let loaded = read_sqlite_data(
        &mut conn,
        Some(UserUid::new("legacy-user")),
        PersistedDataScope::LocalGui,
    )
    .expect("本地读取");
    let state = loaded.app_state.expect("保留 GUI 会话");
    assert_eq!(
        state.windows[0].tabs[0].custom_title.as_deref(),
        Some("本地窗口")
    );
    assert!(state.windows[0].tabs[0].pinned);
    assert_eq!(state.running_mcp_servers.len(), 1);
    assert_eq!(loaded.command_history[0].command, "printf '旧历史'");
    assert_eq!(
        loaded.command_history[0].pwd.as_deref(),
        Some("/tmp/本地项目")
    );
    assert!(loaded.cloud_objects.is_empty());
    assert!(loaded.workspaces.is_empty());
    assert!(loaded.object_actions.is_empty());
    assert!(loaded.multi_agent_conversations.is_empty());
    assert!(loaded.conversation_summary_backfills.is_empty());
    assert!(loaded.codebase_indices.is_empty());
    assert_eq!(loaded.projects.len(), 1);
    assert_eq!(loaded.projects[0].path, "/tmp/旧项目");
    assert_eq!(
        loaded.ignored_suggestions,
        vec![(
            "旧忽略命令".to_owned(),
            crate::suggestions::ignored_suggestions_model::SuggestionType::ShellCommand
        )]
    );
    let queries = queries.lock().expect("查询记录锁");
    assert!(queries.iter().any(|q| q.contains("commands")));
    assert!(queries.iter().any(|q| q.contains("windows")));
    assert!(queries.iter().any(|q| q.contains("projects")));
    assert!(
        queries
            .iter()
            .any(|q| q.contains("ignored_suggestions") && q.contains("shell_command"))
    );
    for table in [
        "object_metadata",
        "workflows",
        "notebooks",
        "folders",
        "generic_string_objects",
        "teams",
        "team_members",
        "team_settings",
        "workspace_teams",
        "workspaces",
        "user_profiles",
        "object_actions",
        "server_experiments",
        "ai_queries",
        "workspace_metadata",
        "workspace_language_server",
        "agent_conversations",
        "agent_tasks",
        "project_rules",
        "mcp_server_installations",
        "mcp_servers_to_restore",
    ] {
        assert!(
            !queries.iter().any(|q| q.contains(table)),
            "本地启动不应查询 {table}"
        );
    }
    assert!(
        queries
            .iter()
            .all(|q| !q.to_uppercase().contains("UPDATE "))
    );
}

#[test]
fn local_gui_read_and_normal_writer_preserve_legacy_rows_and_session() {
    let directory = tempfile::tempdir().expect("临时目录");
    let path = directory.path().join("writer.sqlite");
    let mut conn = setup_database(&path).expect("测试数据库");
    seed_local_gui_fixture(&mut conn);
    let before = local_gui_excluded_rows(&mut conn);
    assert!(before.iter().all(|rows| rows.len() == 1));
    let restored =
        read_sqlite_data(&mut conn, None, PersistedDataScope::LocalGui).expect("本地读取");
    let state = restored.app_state.expect("旧会话");
    let old_windows = state.windows.clone();
    let old_mcp = state.running_mcp_servers.clone();
    let writer = start_writer(conn, path.clone()).expect("正常持久化 writer");
    writer
        .sender
        .send(ModelEvent::Snapshot(state))
        .expect("保存会话");
    writer
        .sender
        .send(ModelEvent::InsertCommand {
            metadata: local_gui_command("printf '新历史'"),
        })
        .expect("写历史");
    writer.sender.send(ModelEvent::Terminate).expect("正常退出");
    writer.handle.join().expect("writer 结束");
    let mut conn = setup_database(&path).expect("重新打开测试数据库");
    assert_eq!(local_gui_excluded_rows(&mut conn), before);
    let restored =
        read_sqlite_data(&mut conn, None, PersistedDataScope::LocalGui).expect("再次本地读取");
    let state = restored.app_state.expect("会话仍在");
    assert_eq!(state.windows, old_windows);
    assert_eq!(state.running_mcp_servers, old_mcp);
    assert_eq!(restored.command_history.len(), 2);
    let metadata = get_all_codebase_index_metadata(&mut conn).expect("完整数据仍在磁盘");
    assert_eq!(metadata[0].path, PathBuf::from("/tmp/旧索引"));
}

#[test]
fn local_gui_ignores_unavailable_cloud_tables_but_propagates_local_read_failures() {
    let directory = tempfile::tempdir().expect("临时目录");
    let mut conn = setup_database(&directory.path().join("cloud.sqlite")).expect("测试数据库");
    conn.batch_execute(
        "ALTER TABLE folders RENAME TO unavailable_folders;
        ALTER TABLE agent_conversations RENAME TO unavailable_conversations;
        ALTER TABLE workspace_metadata RENAME TO unavailable_indices;",
    )
    .expect("模拟不可用的非终端表");
    assert!(read_sqlite_data(&mut conn, None, PersistedDataScope::LocalGui).is_ok());
    assert!(read_sqlite_data(&mut conn, None, PersistedDataScope::Full).is_err());
    for table in ["windows", "commands"] {
        let directory = tempfile::tempdir().expect("临时目录");
        let mut conn = setup_database(&directory.path().join("local.sqlite")).expect("测试数据库");
        conn.batch_execute(&format!(
            "ALTER TABLE {table} RENAME TO unavailable_{table}"
        ))
        .expect("模拟本地读取失败");
        assert!(
            read_sqlite_data(&mut conn, None, PersistedDataScope::LocalGui).is_err(),
            "{table} 错误不得伪装成空数据"
        );
    }
}
