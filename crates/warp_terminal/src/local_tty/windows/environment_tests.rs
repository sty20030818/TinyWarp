use super::*;

#[test]
fn wsl_env_allowlist_includes_client_version_without_notifications_flag() {
    let _guard = FeatureFlag::HOANotifications.override_enabled(false);

    let wslenv = wsl_env_allowlist(false, true)
        .to_string_lossy()
        .into_owned();

    assert_eq!(
        wslenv.split(':').collect::<Vec<_>>(),
        vec![
            format!("{HONOR_PS1_NAME}/u"),
            format!("{USE_SSH_WRAPPER_NAME}/u"),
            format!("{SSH_REUSE_CONTROL_MASTER_NAME}/u"),
            format!("{SHELL_DEBUG_MODE_NAME}/u"),
            format!("{TERM_PROGRAM_NAME}/u"),
            format!("{IS_LOCAL_SESSION_NAME}/u"),
            format!("{SSH_SOCKET_DIR}/u"),
            format!("{WARP_CLIENT_VERSION_ENV}/u"),
            format!("{TERMINAL_SESSION_UUID_ENV}/u"),
            format!("{FOCUS_URL_ENV}/u"),
            format!("{PROMPT_NODE_VERSION_ENABLED_NAME}/u"),
        ],
    );
}

#[test]
fn wsl_env_allowlist_includes_cli_agent_protocol_when_notifications_flag_is_enabled() {
    let _guard = FeatureFlag::HOANotifications.override_enabled(true);

    let wslenv = wsl_env_allowlist(true, true).to_string_lossy().into_owned();

    assert_eq!(
        wslenv.split(':').collect::<Vec<_>>(),
        vec![
            format!("{HONOR_PS1_NAME}/u"),
            format!("{USE_SSH_WRAPPER_NAME}/u"),
            format!("{SSH_REUSE_CONTROL_MASTER_NAME}/u"),
            format!("{SHELL_DEBUG_MODE_NAME}/u"),
            format!("{TERM_PROGRAM_NAME}/u"),
            format!("{IS_LOCAL_SESSION_NAME}/u"),
            format!("{SSH_SOCKET_DIR}/u"),
            format!("{WARP_CLIENT_VERSION_ENV}/u"),
            format!("{TERMINAL_SESSION_UUID_ENV}/u"),
            format!("{FOCUS_URL_ENV}/u"),
            format!("{PROMPT_NODE_VERSION_ENABLED_NAME}/u"),
            format!("{WARP_CLI_AGENT_PROTOCOL_VERSION_ENV}/u"),
            format!("{INITIAL_WORKING_DIR_NAME}/pu"),
        ],
    );
}

fn environment_entry(key: &str, value: OsString) -> (OsString, EnvEntry) {
    (
        map_key(key.into()),
        EnvEntry {
            preferred_key: key.into(),
            value,
        },
    )
}

#[test]
fn disabled_cli_agent_protocol_is_excluded_from_wsl_allowlist() {
    let _guard = FeatureFlag::HOANotifications.override_enabled(true);
    let wslenv = wsl_env_allowlist(true, false)
        .to_string_lossy()
        .into_owned();
    assert!(
        !wslenv
            .split(':')
            .any(|entry| entry.starts_with(WARP_CLI_AGENT_PROTOCOL_VERSION_ENV))
    );
    assert!(wslenv.contains(&format!("{WARP_CLIENT_VERSION_ENV}/u")));
    assert!(wslenv.contains(&format!("{INITIAL_WORKING_DIR_NAME}/pu")));
}

#[test]
fn disabled_cli_agent_protocol_removes_inherited_windows_env_and_wslenv_entries() {
    let protocol_key = WARP_CLI_AGENT_PROTOCOL_VERSION_ENV.to_ascii_lowercase();
    let mut env = BTreeMap::from([
        environment_entry(&protocol_key, "inherited-advertisement".into()),
        environment_entry("WsLeNv", format!("KEEP/p:{protocol_key}/u:OTHER/l").into()),
        environment_entry("USER_ENV", "keep-me".into()),
    ]);
    apply_environment_overrides(&mut env, &HashMap::new(), false);
    assert!(!env.contains_key(&map_key(WARP_CLI_AGENT_PROTOCOL_VERSION_ENV.into())));
    assert_eq!(
        env[&map_key(WSLENV.into())].value,
        OsString::from("KEEP/p:OTHER/l")
    );
    assert_eq!(
        env[&map_key("USER_ENV".into())].value,
        OsString::from("keep-me")
    );
}

#[test]
fn disabled_cli_agent_protocol_removes_user_overrides_and_preserves_utf16_wslenv() {
    let protocol_key = WARP_CLI_AGENT_PROTOCOL_VERSION_ENV.to_ascii_lowercase();
    let mut wslenv_units: Vec<u16> = "KEEP/p:".encode_utf16().collect();
    wslenv_units.push(0xd800);
    wslenv_units.extend(format!("/l:{protocol_key}/pu:OTHER/u").encode_utf16());
    let mut expected_units: Vec<u16> = "KEEP/p:".encode_utf16().collect();
    expected_units.push(0xd800);
    expected_units.extend("/l:OTHER/u".encode_utf16());
    let overrides = HashMap::from([
        (protocol_key.into(), OsString::from("caller-override")),
        (OsString::from("wslenv"), OsString::from_wide(&wslenv_units)),
        (OsString::from("USER_ENV"), OsString::from("keep-me")),
    ]);
    let mut env = BTreeMap::new();
    apply_environment_overrides(&mut env, &overrides, false);
    assert!(!env.contains_key(&map_key(WARP_CLI_AGENT_PROTOCOL_VERSION_ENV.into())));
    assert_eq!(
        env[&map_key(WSLENV.into())].value,
        OsString::from_wide(&expected_units)
    );
    assert_eq!(
        env[&map_key("USER_ENV".into())].value,
        OsString::from("keep-me")
    );
}

#[test]
fn enabled_cli_agent_protocol_keeps_windows_environment_overrides() {
    let overrides = HashMap::from([
        (
            OsString::from(WARP_CLI_AGENT_PROTOCOL_VERSION_ENV),
            OsString::from("caller-override"),
        ),
        (
            OsString::from(WSLENV),
            OsString::from(format!("KEEP/p:{WARP_CLI_AGENT_PROTOCOL_VERSION_ENV}/u")),
        ),
    ]);
    let mut env = BTreeMap::new();
    apply_environment_overrides(&mut env, &overrides, true);
    assert_eq!(
        env[&map_key(WARP_CLI_AGENT_PROTOCOL_VERSION_ENV.into())].value,
        OsString::from("caller-override")
    );
    assert_eq!(
        env[&map_key(WSLENV.into())].value,
        overrides[&OsString::from(WSLENV)]
    );
}
