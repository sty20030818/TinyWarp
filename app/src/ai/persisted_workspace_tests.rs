use warpui::App;

use super::*;

#[test]
fn local_terminal_keeps_workspace_metadata_without_starting_lsp_or_indexing() {
    App::test((), |mut app| async move {
        app.update(crate::local_terminal::enable_for_test);
        let path = PathBuf::from("/local-mode-must-not-scan");
        let (tx, rx) = std::sync::mpsc::sync_channel(4);
        let model = app.add_singleton_model(|ctx| {
            PersistedWorkspace::new(
                vec![WorkspaceMetadata {
                    path: path.clone(),
                    ..Default::default()
                }],
                [(
                    path.clone(),
                    [(LSPServerType::RustAnalyzer, EnablementState::Yes)].into(),
                )]
                .into(),
                Some(tx),
                ctx,
            )
        });
        model.update(&mut app, |model, ctx| {
            model.index_repo(path.clone(), ctx);
            #[cfg(feature = "local_fs")]
            {
                model.detect_available_servers_for_workspaces(vec![path.clone()], true, ctx);
                model.execute_lsp_task(
                    LspTask::Spawn {
                        file_path: path.join("main.rs"),
                    },
                    ctx,
                );
                assert!(model.lsp_installation_status.is_empty());
            }
            assert_eq!(model.workspaces.len(), 1);
            assert_eq!(model.workspaces[&path].metadata.path, path);
            assert_eq!(
                model.workspaces[&path].language_servers[&LSPServerType::RustAnalyzer],
                EnablementState::Yes
            );
        });
        assert!(matches!(
            rx.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));
    });
}
