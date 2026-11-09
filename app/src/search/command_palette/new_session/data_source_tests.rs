use std::path::PathBuf;

use warpui::keymap::DescriptionContext;

use super::*;
use crate::terminal::available_shells::AvailableShell;
use crate::terminal::shell::ShellType;

#[test]
fn localized_new_session_search_keeps_english_aliases_and_original_id() {
    let shell = AvailableShell::new_custom_shell(
        "/tmp/用户Shell".into(),
        PathBuf::from("/tmp/用户Shell"),
        ShellType::Zsh,
    );
    let id = NewSessionOptionId::new("shell:original-new-tab-id".into());
    let option = Arc::new(NewSessionOption::new(
        id.clone(),
        NewSessionConfig::NewTab(shell),
    ));
    assert!(option.description().starts_with("新建标签页"));
    let mut searcher = FuzzyNewSessionSearcher::default();
    searcher.bindings_mut().insert(id.clone(), option.clone());
    for query in ["新建标签页", "create new tab"] {
        let results = searcher.search(query).unwrap();
        assert_eq!(results.len(), 1);
        let CommandPaletteItemAction::NewSession { source } = results[0].accept_result() else {
            panic!("新会话搜索结果必须保留原操作");
        };
        assert_eq!(source.id(), &id);
    }
    let matched = match_binding_description(
        option.binding_description(),
        DescriptionContext::Default,
        "create new tab",
    )
    .unwrap();
    assert!(matched.matched_indices.is_empty());
}

#[test]
fn localized_shell_details_preserve_custom_path_and_selection_value() {
    let path = "/tmp/中文 shell {用户路径}";
    let shell = AvailableShell::new_custom_shell(path.into(), PathBuf::from(path), ShellType::Zsh);
    assert_eq!(shell.details(), format!("自定义： {path}"));
    assert_eq!(shell.short_name(), "Custom");
    assert_eq!(AvailableShell::default().details(), "系统默认 Shell");
}
