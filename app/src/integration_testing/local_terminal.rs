//! 本地终端专项集成测试的启动前 fixture；仅随 integration_tests 编译。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use parking_lot::Mutex;
use warp_core::AppId;
use warp_core::channel::{Channel, ChannelState};
use warpui::integration::AssertionOutcome;
use warpui::{App, AppContext, SingletonEntity, WindowId};
use warpui_extras::secure_storage::{self, SecureStorage};

use crate::auth::AuthStateProvider;
use crate::auth::auth_state::PersistAction;

pub const TEST_APP_NAME: &str = "WarpL1Integration";

// 人工测试凭据，只有内存 fixture 使用，不能从真实 store 或环境变量复制。
const LEGACY_USER: &str = r#"{"id_token":{"id_token":"l1-fixture-not-a-real-id-token","refresh_token":"l1-fixture-not-a-real-refresh-token","expiration_time":"2099-01-01T00:00:00Z"},"refresh_token":"","local_id":"l1-fixture-user","email":"l1-fixture@example.invalid","display_name":"L1 旧账号 fixture","photo_url":null,"is_onboarded":true,"needs_sso_link":false,"anonymous_user_type":null,"linked_at":null,"personal_object_limits":null,"is_on_work_domain":false}"#;

#[derive(Clone, Copy)]
pub enum AuthFixture {
    Empty,
    LegacyUser,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Operations {
    reads: usize,
    writes: usize,
    removals: usize,
}

#[derive(Default)]
struct StoreState {
    registered: bool,
    values: HashMap<String, String>,
    user_operations: Operations,
    all_operations: Operations,
}

struct StorageSpy {
    initial_user: Option<String>,
    state: Mutex<StoreState>,
}

struct RegisteredStorage(Arc<StorageSpy>);

static STARTUP: OnceLock<Arc<StorageSpy>> = OnceLock::new();

/// 必须在构建 driver 时、真实应用初始化之前设置，不能用首个 TestStep 注入身份。
pub fn configure_startup(fixture: AuthFixture) {
    let initial_user = match fixture {
        AuthFixture::Empty => None,
        AuthFixture::LegacyUser => {
            serde_json::from_str::<crate::auth::user::persistence::PersistedUser>(LEGACY_USER)
                .expect("旧账号 fixture 应保持真实持久化格式");
            Some(LEGACY_USER.to_owned())
        }
    };
    let mut state = StoreState::default();
    if let Some(value) = initial_user.clone() {
        state.values.insert("User".to_owned(), value);
    }
    assert!(
        STARTUP
            .set(Arc::new(StorageSpy {
                initial_user,
                state: Mutex::new(state),
            }))
            .is_ok(),
        "每个专项测试进程只能配置一次启动 fixture"
    );
}

pub fn is_configured() -> bool {
    STARTUP.get().is_some()
}

/// 返回 false 时调用者继续使用原有 integration no-op store。
pub(crate) fn register_secure_storage(ctx: &mut AppContext) -> bool {
    let Some(store) = STARTUP.get() else {
        return false;
    };
    let store = store.clone();
    store.state.lock().registered = true;
    ctx.add_singleton_model(|_| -> secure_storage::Model { Box::new(RegisteredStorage(store)) });
    true
}

impl SecureStorage for RegisteredStorage {
    fn read_value(&self, key: &str) -> Result<String, secure_storage::Error> {
        let mut state = self.0.state.lock();
        state.all_operations.reads += 1;
        if key == "User" {
            state.user_operations.reads += 1;
        }
        state
            .values
            .get(key)
            .cloned()
            .ok_or(secure_storage::Error::NotFound)
    }

    fn write_value(&self, key: &str, value: &str) -> Result<(), secure_storage::Error> {
        let mut state = self.0.state.lock();
        state.all_operations.writes += 1;
        if key == "User" {
            state.user_operations.writes += 1;
        }
        state.values.insert(key.to_owned(), value.to_owned());
        Ok(())
    }

    fn remove_value(&self, key: &str) -> Result<(), secure_storage::Error> {
        let mut state = self.0.state.lock();
        state.all_operations.removals += 1;
        if key == "User" {
            state.user_operations.removals += 1;
        }
        state.values.remove(key);
        Ok(())
    }
}

pub fn assert_startup_state(app: &mut App, _window_id: WindowId) -> AssertionOutcome {
    app.read(|ctx| {
        if ChannelState::channel() != Channel::Oss
            || ChannelState::app_id().to_string()
                != AppId::new("dev", "warp", TEST_APP_NAME).to_string()
        {
            return AssertionOutcome::failure("专项测试必须保留独立 OSS 测试身份".to_owned());
        }
        if !crate::local_terminal::is_enabled(ctx) {
            return AssertionOutcome::failure("初始化前应已启用本地终端策略".to_owned());
        }
        let auth = AuthStateProvider::as_ref(ctx).get();
        if auth.user_id().is_some() || auth.credentials().is_some() || auth.is_logged_in() {
            return AssertionOutcome::failure("首启必须是真实无账号、无凭据状态".to_owned());
        }
        if !matches!(auth.persist_action(), PersistAction::DoNothing) {
            return AssertionOutcome::failure("本地认证状态不能请求修改持久化凭据".to_owned());
        }
        if warp_core::paths::secure_state_dir().is_some() {
            return AssertionOutcome::failure("专项测试不能使用 App Group 数据目录".to_owned());
        }
        #[cfg(target_os = "macos")]
        if warp_core::paths::app_group_container_path().is_some() {
            return AssertionOutcome::failure("专项测试不能探测 App Group 容器".to_owned());
        }
        let home = PathBuf::from(std::env::var_os("HOME").expect("driver 设置临时 HOME"));
        if !warp_core::paths::state_dir().starts_with(&home)
            || !crate::settings::user_preferences_file_path().starts_with(&home)
        {
            return AssertionOutcome::failure("数据库和配置必须在临时 HOME 内".to_owned());
        }
        let store = STARTUP.get().expect("专项测试已在初始化前配置 fixture");
        let state = store.state.lock();
        if !state.registered {
            return AssertionOutcome::failure("应用初始化必须实际注册凭据 Spy".to_owned());
        }
        if state.user_operations != Operations::default() {
            return AssertionOutcome::failure(format!(
                "旧账号凭据不应被读取、写入或删除：User {:?}，所有键 {:?}",
                state.user_operations, state.all_operations,
            ));
        }
        if state.values.get("User") != store.initial_user.as_ref() {
            return AssertionOutcome::failure("旧账号凭据 fixture 必须保持原始字节".to_owned());
        }
        AssertionOutcome::Success
    })
}

pub fn assert_onboarding_incomplete(app: &mut App, _window_id: WindowId) -> AssertionOutcome {
    app.read(|ctx| {
        if crate::root_view::has_completed_local_onboarding(ctx) {
            AssertionOutcome::failure("无账号首启不能靠写入 onboarding 完成标记进入终端".to_owned())
        } else {
            AssertionOutcome::Success
        }
    })
}

pub const LEGACY_HISTORY_COMMAND: &str = "printf 'L1 旧历史 {保持原样}'";
pub const LEGACY_HISTORY_PWD: &str = "/tmp/L1 旧目录 {原值}";

/// 只向 driver 已复制进临时 HOME 的旧版 SQLite fixture 添加人工历史。
pub fn seed_legacy_history(username: &str, hostname: &str) {
    use diesel::sql_types::Text;
    use diesel::{Connection as _, RunQueryDsl as _};

    let path = crate::persistence::database_file_path_for_scope(
        &crate::persistence::PersistenceScope::App,
    );
    let home = PathBuf::from(std::env::var_os("HOME").expect("driver 设置临时 HOME"));
    assert!(
        path.starts_with(home),
        "只能修改临时 HOME 的 fixture 数据库"
    );
    let mut conn =
        diesel::SqliteConnection::establish(path.to_str().expect("fixture path 是 UTF-8"))
            .expect("连接已复制的旧版 fixture");
    for shell in ["zsh", "bash", "fish", "pwsh"] {
        diesel::sql_query(
            "INSERT INTO commands (command, pwd, shell, username, hostname) VALUES (?, ?, ?, ?, ?)",
        )
        .bind::<Text, _>(LEGACY_HISTORY_COMMAND)
        .bind::<Text, _>(LEGACY_HISTORY_PWD)
        .bind::<Text, _>(shell)
        .bind::<Text, _>(username)
        .bind::<Text, _>(hostname)
        .execute(&mut conn)
        .expect("向 fixture 插入人工历史");
    }
}

pub fn assert_legacy_tabs_and_history(app: &mut App, window_id: WindowId) -> AssertionOutcome {
    use crate::integration_testing::view_getters::{single_terminal_view_for_tab, workspace_view};
    use crate::terminal::History;

    let workspace = workspace_view(app, window_id);
    let tab_count = workspace.read(app, |workspace, _| workspace.tab_count());
    if tab_count != 3 {
        return AssertionOutcome::failure(format!(
            "旧版 fixture 的三个 Tab 应保留，实际 {tab_count}"
        ));
    }
    single_terminal_view_for_tab(app, window_id, 0).read(app, |terminal, ctx| {
        let Some(session_id) = terminal.active_block_session_id() else {
            return AssertionOutcome::failure("终端应已创建本地会话".to_owned());
        };
        let history = History::as_ref(ctx).commands(session_id);
        let entry = history.as_ref().and_then(|history| {
            history
                .iter()
                .find(|entry| entry.command == LEGACY_HISTORY_COMMAND)
        });
        match entry {
            // 历史文件负责召回，目录元数据必须来自 SQLite summary 的真实 join。
            Some(entry) if entry.pwd.as_deref() == Some(LEGACY_HISTORY_PWD) => {
                AssertionOutcome::Success
            }
            Some(_) => AssertionOutcome::failure(
                "旧历史命令存在，但 SQLite 的目录元数据没有关联到实际历史条目".to_owned(),
            ),
            None => AssertionOutcome::failure(
                "真实无账号启动后应能访问旧版 fixture 的本地命令历史".to_owned(),
            ),
        }
    })
}

/// 对初始化前保存的旧 Agent 默认值，只调整运行投影，不写回偏好。
pub fn assert_shell_runtime_keeps_legacy_agent_preference(
    app: &mut App,
    window_id: WindowId,
) -> AssertionOutcome {
    use settings::Setting as _;
    use warpui_extras::user_preferences::file_backed::FileBackedUserPreferences;
    use warpui_extras::user_preferences::toml_backed::TomlBackedUserPreferences;

    use crate::integration_testing::view_getters::single_terminal_view_for_tab;
    use crate::settings::AISettings;
    use crate::settings::ai::DefaultSessionMode;

    let preference = app.read(|ctx| {
        let ai = AISettings::as_ref(ctx);
        if ai.default_session_mode(ctx) != DefaultSessionMode::Terminal
            || *ai.default_session_mode_internal.value() != DefaultSessionMode::Agent
        {
            return AssertionOutcome::failure(
                "旧 Agent 默认值应保持，运行时必须使用 Terminal".to_owned(),
            );
        }
        let home = PathBuf::from(std::env::var_os("HOME").expect("driver 设置临时 HOME"));
        let path = crate::settings::user_preferences_file_path();
        assert!(path.starts_with(&home), "只读取临时 HOME 的偏好 fixture");
        let native = match FileBackedUserPreferences::new(path) {
            Ok(native) => native,
            Err(err) => {
                return AssertionOutcome::failure(format!("读取旧偏好 fixture 失败：{err}"));
            }
        };
        if DefaultSessionMode::read_from_preferences(&native) != Some(DefaultSessionMode::Agent) {
            return AssertionOutcome::failure("磁盘旧默认模式不能被改写为 Terminal".to_owned());
        }
        if DefaultSessionMode::preferences_for_setting(ctx).is_settings_file() {
            let path = crate::settings::user_preferences_toml_file_path();
            assert!(path.starts_with(home), "只读取临时 HOME 的 TOML fixture");
            let (preferences, error) = TomlBackedUserPreferences::new(path);
            if error.is_some()
                || DefaultSessionMode::read_from_preferences(&preferences)
                    != Some(DefaultSessionMode::Agent)
            {
                return AssertionOutcome::failure(
                    "迁移后的 TOML 仍应保留 Agent 默认模式".to_owned(),
                );
            }
        }
        AssertionOutcome::Success
    });
    if !matches!(preference, AssertionOutcome::Success) {
        return preference;
    }
    single_terminal_view_for_tab(app, window_id, 0).read(app, |terminal, ctx| {
        let input = terminal.input_config(ctx);
        if !input.is_shell()
            || !input.is_locked
            || terminal.agent_view_controller().as_ref(ctx).is_active()
        {
            return AssertionOutcome::failure(
                "首次运行应使用锁定 Shell 输入，且不进入 Agent 视图".to_owned(),
            );
        }
        AssertionOutcome::Success
    })
}

pub fn assert_external_cli_finished_successfully(
    app: &mut App,
    window_id: WindowId,
) -> AssertionOutcome {
    use crate::integration_testing::view_getters::single_terminal_view_for_tab;
    use crate::terminal::model::block::BlockState;
    use crate::terminal::model::blocks::BlockFilter;

    single_terminal_view_for_tab(app, window_id, 0).read(app, |terminal, _| {
        let model = terminal.model.lock();
        let blocks = model.block_list();
        let Some(block) = blocks
            .last_matching_block_by_index(BlockFilter::commands())
            .and_then(|index| blocks.block_at(index))
        else {
            return AssertionOutcome::failure("外部 CLI 命令块尚未生成".to_owned());
        };
        if !matches!(block.state(), BlockState::DoneWithExecution) {
            return AssertionOutcome::failure("外部 CLI 尚未正常完成".to_owned());
        }
        let output = block.output_to_string();
        if block.exit_code().value() != 0
            || !output.lines().any(|line| line == "tinywarp-l2-cli-ready")
            || !output
                .lines()
                .any(|line| line == "tinywarp-l2-stdin:石头鱼 L2 {stdin}")
        {
            return AssertionOutcome::failure(format!(
                "外部 CLI 输出或退出码不符：exit={}，output={output:?}",
                block.exit_code().value()
            ));
        }
        AssertionOutcome::Success
    })
}

/// 真实 PaneView 的类型排除仅保存 Settings 描述的停用占位 pane。
pub fn assert_local_settings_pane_is_open_once(
    app: &mut App,
    window_id: WindowId,
) -> AssertionOutcome {
    use crate::integration_testing::view_getters::workspace_view;
    use crate::pane_group::PaneView;
    use crate::settings_view::SettingsView;

    let tab_count = workspace_view(app, window_id).read(app, |workspace, _| workspace.tab_count());
    if tab_count != 2 {
        return AssertionOutcome::failure(format!(
            "应保留原终端并只打开一个设置标签，实际标签数 {tab_count}",
        ));
    }
    let panes = app
        .views_of_type::<PaneView<SettingsView>>(window_id)
        .unwrap_or_default();
    if panes.len() != 1 {
        return AssertionOutcome::failure(format!(
            "应有一个真实 SettingsPane 包装视图，不能用停用占位 pane 代替，实际 {}",
            panes.len(),
        ));
    }
    let settings = panes[0].read(app, |pane, ctx| pane.child(ctx));
    settings.read(app, |view, _| {
        let section = view.current_settings_section();
        if section.is_available_in_local_terminal() {
            AssertionOutcome::Success
        } else {
            AssertionOutcome::failure(format!("设置应打开可用的本地页面，实际 {section:?}"))
        }
    })
}
