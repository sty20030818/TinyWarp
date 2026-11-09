use super::*;

fn shell_starter(shell_type: ShellType, shell_path: &str) -> DirectShellStarter {
    DirectShellStarter::new_for_test(shell_type, PathBuf::from(shell_path), Vec::new())
}

fn env_value(command: &Command, key: &str) -> Option<Option<String>> {
    command
        .get_envs()
        .find(|(env_key, _)| *env_key == std::ffi::OsStr::new(key))
        .map(|(_, value)| value.map(|value| value.to_string_lossy().into_owned()))
}

#[test]
fn host_bash_command_sets_history_size_sentinels() {
    let command = build_host_shell_command(
        shell_starter(ShellType::Bash, "/bin/bash"),
        None,
        HashMap::new(),
        true,
        None,
        false,
        false,
        false,
        false,
        true,
    );

    assert_eq!(
        env_value(&command, "HISTFILESIZE"),
        Some(Some(BASH_HISTORY_SIZE_SENTINEL.to_owned()))
    );
    assert_eq!(
        env_value(&command, "HISTSIZE"),
        Some(Some(BASH_HISTORY_SIZE_SENTINEL.to_owned()))
    );
    assert_eq!(
        env_value(&command, "WARP_INITIAL_HISTFILESIZE"),
        Some(Some(BASH_HISTORY_SIZE_SENTINEL.to_owned()))
    );
    assert_eq!(
        env_value(&command, "WARP_INITIAL_HISTSIZE"),
        Some(Some(BASH_HISTORY_SIZE_SENTINEL.to_owned()))
    );
}

#[test]
fn host_non_bash_command_does_not_set_history_size_sentinels() {
    let command = build_host_shell_command(
        shell_starter(ShellType::Zsh, "/bin/zsh"),
        None,
        HashMap::new(),
        true,
        None,
        false,
        false,
        false,
        false,
        true,
    );

    assert_eq!(env_value(&command, "HISTFILESIZE"), None);
    assert_eq!(env_value(&command, "HISTSIZE"), None);
    assert_eq!(env_value(&command, "WARP_INITIAL_HISTFILESIZE"), None);
    assert_eq!(env_value(&command, "WARP_INITIAL_HISTSIZE"), None);
}

#[test]
fn docker_sandbox_command_sets_history_size_sentinels() {
    let docker_starter =
        DockerSandboxShellStarter::new(shell_starter(ShellType::Bash, "sbx"), None);
    let command = build_docker_sandbox_command(
        &docker_starter,
        None,
        HashMap::new(),
        true,
        false,
        false,
        false,
        false,
        true,
    );

    assert_eq!(
        env_value(&command, "HISTFILESIZE"),
        Some(Some(BASH_HISTORY_SIZE_SENTINEL.to_owned()))
    );
    assert_eq!(
        env_value(&command, "HISTSIZE"),
        Some(Some(BASH_HISTORY_SIZE_SENTINEL.to_owned()))
    );
    assert_eq!(
        env_value(&command, "WARP_INITIAL_HISTFILESIZE"),
        Some(Some(BASH_HISTORY_SIZE_SENTINEL.to_owned()))
    );
    assert_eq!(
        env_value(&command, "WARP_INITIAL_HISTSIZE"),
        Some(Some(BASH_HISTORY_SIZE_SENTINEL.to_owned()))
    );
}

fn cli_protocol_commands(
    enable_cli_agent_protocol: bool,
    env_vars: HashMap<OsString, OsString>,
) -> [Command; 2] {
    let host = build_host_shell_command(
        shell_starter(ShellType::Zsh, "/bin/zsh"),
        None,
        env_vars.clone(),
        enable_cli_agent_protocol,
        None,
        false,
        false,
        false,
        false,
        true,
    );
    let docker_starter =
        DockerSandboxShellStarter::new(shell_starter(ShellType::Bash, "sbx"), None);
    let docker = build_docker_sandbox_command(
        &docker_starter,
        None,
        env_vars,
        enable_cli_agent_protocol,
        false,
        false,
        false,
        false,
        true,
    );
    [host, docker]
}

#[test]
fn disabled_cli_agent_protocol_removes_inherited_and_overridden_advertisement() {
    let _guard = FeatureFlag::HOANotifications.override_enabled(true);
    for overrides in [
        HashMap::from([(OsString::from("USER_ENV"), OsString::from("keep-me"))]),
        HashMap::from([
            (
                WARP_CLI_AGENT_PROTOCOL_VERSION_ENV.into(),
                "caller-override".into(),
            ),
            ("USER_ENV".into(), "keep-me".into()),
        ]),
    ] {
        for command in cli_protocol_commands(false, overrides) {
            assert_eq!(
                env_value(&command, WARP_CLI_AGENT_PROTOCOL_VERSION_ENV),
                Some(None)
            );
            assert_eq!(
                env_value(&command, "USER_ENV"),
                Some(Some("keep-me".to_string()))
            );
        }
    }
}

#[test]
fn enabled_cli_agent_protocol_keeps_feature_gated_advertisement_and_overrides() {
    let _guard = FeatureFlag::HOANotifications.override_enabled(true);
    for command in cli_protocol_commands(true, HashMap::new()) {
        assert_eq!(
            env_value(&command, WARP_CLI_AGENT_PROTOCOL_VERSION_ENV),
            Some(Some(CLI_AGENT_PROTOCOL_VERSION.to_string()))
        );
    }
    let overrides = HashMap::from([
        (
            WARP_CLI_AGENT_PROTOCOL_VERSION_ENV.into(),
            "caller-override".into(),
        ),
        ("USER_ENV".into(), "keep-me".into()),
    ]);
    for command in cli_protocol_commands(true, overrides) {
        assert_eq!(
            env_value(&command, WARP_CLI_AGENT_PROTOCOL_VERSION_ENV),
            Some(Some("caller-override".to_string()))
        );
        assert_eq!(
            env_value(&command, "USER_ENV"),
            Some(Some("keep-me".to_string()))
        );
    }
}

#[test]
fn enabled_cli_agent_protocol_does_not_advertise_when_notifications_are_disabled() {
    let _guard = FeatureFlag::HOANotifications.override_enabled(false);
    for command in cli_protocol_commands(true, HashMap::new()) {
        assert_eq!(
            env_value(&command, WARP_CLI_AGENT_PROTOCOL_VERSION_ENV),
            None
        );
    }
}

fn cli_protocol_pty_options(enable_cli_agent_protocol: bool) -> PtyOptions {
    PtyOptions {
        size: SizeInfo::new_without_font_metrics(24, 80),
        window_id: None,
        shell_starter: ShellStarter::Direct(shell_starter(ShellType::Zsh, "/bin/zsh")),
        start_dir: None,
        env_vars: HashMap::new(),
        enable_cli_agent_protocol,
        enable_ssh_wrapper: true,
        reuse_ssh_control_master: true,
        shell_debug_mode: false,
        honor_ps1: true,
        node_version_chip_enabled: true,
        close_fds: true,
    }
}

#[test]
fn pty_options_json_missing_field_defaults_cli_agent_protocol_to_enabled() {
    let options = cli_protocol_pty_options(false);
    let mut payload = serde_json::to_value(&options).unwrap();
    let explicit: PtyOptions = serde_json::from_value(payload.clone()).unwrap();
    assert!(!explicit.enable_cli_agent_protocol);
    payload
        .as_object_mut()
        .unwrap()
        .remove("enable_cli_agent_protocol");
    let defaulted: PtyOptions = serde_json::from_value(payload).unwrap();
    assert!(defaulted.enable_cli_agent_protocol);
    assert!(defaulted.enable_ssh_wrapper);
    assert!(defaulted.reuse_ssh_control_master);
    assert_eq!(defaulted.size.rows, 24);
    assert_eq!(defaulted.size.columns, 80);
}

#[test]
fn pty_options_current_bincode_round_trip_preserves_cli_protocol_and_existing_fields() {
    for enabled in [false, true] {
        let mut options = cli_protocol_pty_options(enabled);
        options.window_id = Some(7);
        options.start_dir = Some(PathBuf::from("/fixture/旧目录"));
        options.env_vars.insert("USER_ENV".into(), "原值".into());
        let wire = bincode::serialize(&options).unwrap();
        let decoded: PtyOptions = bincode::deserialize(&wire).unwrap();
        assert_eq!(decoded.enable_cli_agent_protocol, enabled);
        assert_eq!(decoded.window_id, options.window_id);
        assert_eq!(decoded.start_dir, options.start_dir);
        assert_eq!(decoded.env_vars, options.env_vars);
        assert_eq!(decoded.size.rows, options.size.rows);
        assert_eq!(decoded.size.columns, options.size.columns);
        assert_eq!(decoded.enable_ssh_wrapper, options.enable_ssh_wrapper);
        assert_eq!(
            decoded.reuse_ssh_control_master,
            options.reuse_ssh_control_master
        );
        assert_eq!(decoded.shell_debug_mode, options.shell_debug_mode);
        assert_eq!(decoded.honor_ps1, options.honor_ps1);
        assert_eq!(
            decoded.node_version_chip_enabled,
            options.node_version_chip_enabled
        );
        assert_eq!(decoded.close_fds, options.close_fds);
        assert_eq!(bincode::serialize(&decoded).unwrap(), wire);
    }
}
