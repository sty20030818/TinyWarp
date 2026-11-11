use super::PersistedDataScope;
use crate::LaunchMode;

#[test]
fn local_gui_scope_keeps_sessions_and_history_without_cloud_profiles_or_actions() {
    let scope = PersistedDataScope::LocalGui;
    assert!(scope.session_restoration());
    assert!(scope.command_history());
    assert!(!scope.user_profiles());
    assert!(!scope.gui_only_data());
}

#[test]
fn local_terminal_scope_applies_to_gui_and_explicit_test_launches_only() {
    let gui = LaunchMode::App {
        args: Default::default(),
        api_key: None,
    };
    let test = LaunchMode::Test {
        driver: Box::new(None),
        is_integration_test: true,
        local_terminal: true,
    };
    for mode in [&gui, &test] {
        assert_eq!(
            PersistedDataScope::for_launch_mode(mode, true),
            PersistedDataScope::LocalGui
        );
        assert_eq!(
            PersistedDataScope::for_launch_mode(mode, false),
            PersistedDataScope::Full
        );
    }
    for local_terminal in [false, true] {
        assert_eq!(
            PersistedDataScope::for_launch_mode(&LaunchMode::RemoteServerProxy, local_terminal),
            PersistedDataScope::Full
        );
        let daemon = LaunchMode::RemoteServerDaemon {
            identity_key: "fixture".to_owned(),
        };
        assert_eq!(
            PersistedDataScope::for_launch_mode(&daemon, local_terminal),
            PersistedDataScope::CodebaseIndicesOnly
        );
    }
}
