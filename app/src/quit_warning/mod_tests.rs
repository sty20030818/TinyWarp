use super::{QuitScope, UnsavedStateSummary};

fn summary(scope: QuitScope<'static>) -> UnsavedStateSummary<'static> {
    UnsavedStateSummary {
        scope,
        total_long_running_commands: 3,
        windows_with_long_running_commands: 2,
        tabs_with_long_running_commands: 2,
        terminal_sessions: vec![],
        shared_sessions: 1,
        unsaved_code_changes: true,
    }
}

#[test]
fn localized_quit_warning_keeps_every_unsaved_state_category_and_count() {
    let warning = summary(QuitScope::App).warning_text();
    let lines = warning.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("3") && lines[0].contains("2"));
    assert!(lines[0].contains("进程") && lines[0].contains("窗口"));
    assert!(lines[1].contains("1") && lines[1].contains("共享"));
    assert!(lines[2].contains("未保存"));
}

#[test]
fn localized_quit_warning_preserves_unicode_file_name_and_discard_warning() {
    let file_name = "报告🔥.md";
    let mut state = summary(QuitScope::EditorTab {
        file_name: Some(file_name.into()),
        editor_status: vec![],
    });
    state.total_long_running_commands = 0;
    state.shared_sessions = 0;
    let warning = state.warning_text();
    assert!(warning.contains(file_name));
    assert!(warning.contains("不保存") && warning.contains("丢失"));
}
