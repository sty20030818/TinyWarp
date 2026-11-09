use settings::schema::SettingSchemaEntry;
use settings::{Setting, SettingSurfaces, SettingsMode};

use super::{IsCloudConversationStorageEnabled, IsCrashReportingEnabled, IsTelemetryEnabled};

#[test]
fn privacy_settings_apply_to_gui_and_tui() {
    for storage_key in [
        IsTelemetryEnabled::toml_key(),
        IsCrashReportingEnabled::toml_key(),
        IsCloudConversationStorageEnabled::toml_key(),
    ] {
        let entry = inventory::iter::<SettingSchemaEntry>
            .into_iter()
            .find(|entry| entry.storage_key == storage_key)
            .unwrap_or_else(|| panic!("missing schema entry for {storage_key}"));
        let surfaces = (entry.surfaces_fn)();

        assert_eq!(surfaces, SettingSurfaces::ALL, "{storage_key}");
        assert!(surfaces.includes(SettingsMode::Gui), "{storage_key}");
        assert!(surfaces.includes(SettingsMode::Tui), "{storage_key}");
    }
}

#[test]
fn local_terminal_initializes_default_secret_rules_without_cloud_load() {
    use warpui::{App, SingletonEntity};

    use super::PrivacySettings;
    use crate::server::cloud_objects::test_utils::initialize_app;
    use crate::server::cloud_objects::update_manager::UpdateManager;

    App::test((), |mut app| async move {
        app.update(crate::local_terminal::enable_for_test);
        initialize_app(&mut app);
        PrivacySettings::handle(&app).update(&mut app, |settings, ctx| {
            assert!(!ctx.has_singleton_model::<UpdateManager>());
            assert!(*settings.has_initialized_default_secret_regexes.value());
            let original_rules = settings
                .user_secret_regex_list
                .iter()
                .map(|rule| (rule.pattern.as_str().to_owned(), rule.name.clone()))
                .collect::<Vec<_>>();
            assert!(!original_rules.is_empty());

            settings.maybe_sync_with_warp_drive_prefs(ctx);
            settings.maybe_sync_with_warp_drive_prefs(ctx);

            let current_rules = settings
                .user_secret_regex_list
                .iter()
                .map(|rule| (rule.pattern.as_str().to_owned(), rule.name.clone()))
                .collect::<Vec<_>>();
            assert_eq!(current_rules, original_rules);
            assert!(*settings.has_initialized_default_secret_regexes.value());
            assert!(!ctx.has_singleton_model::<UpdateManager>());
        });
    });
}

#[test]
fn local_terminal_preserves_previously_initialized_custom_secret_rule_and_name() {
    use regex::Regex;
    use settings::manager::SettingsManager;
    use warpui::{App, SingletonEntity};

    use super::{
        CustomSecretRegex, CustomSecretRegexList, HasInitializedDefaultSecretRegexes,
        PrivacySettings, WarpDrivePrivacySettings,
    };
    use crate::auth::AuthStateProvider;
    use crate::server::server_api::ServerApiProvider;

    App::test((), |mut app| async move {
        app.update(crate::local_terminal::enable_for_test);
        app.add_singleton_model(|_| SettingsManager::default());
        app.update(crate::settings::init_and_register_user_preferences);
        app.add_singleton_model(|_| ServerApiProvider::new_for_test());
        app.add_singleton_model(|_| AuthStateProvider::new_for_test());
        WarpDrivePrivacySettings::register(&mut app);

        let original_rule = CustomSecretRegex {
            pattern: Regex::new(r"local-secret-[0-9]+").expect("有效自定义正则"),
            name: Some("我的本地规则 🔒".to_owned()),
        };
        // 临时非 singleton 仅提供设置写入所需的 ModelContext，所有偏好均在内存。
        let _seed = app.add_model(|ctx| {
            CustomSecretRegexList::new(None)
                .set_value(vec![original_rule.clone()], ctx)
                .expect("保存原自定义规则");
            HasInitializedDefaultSecretRegexes::new(None)
                .set_value(true, ctx)
                .expect("保存已有初始化标记");
            PrivacySettings::mock(ctx)
        });
        app.update(PrivacySettings::register_singleton);

        PrivacySettings::handle(&app).update(&mut app, |settings, ctx| {
            assert!(*settings.has_initialized_default_secret_regexes.value());
            assert_eq!(settings.user_secret_regex_list.len(), 1);
            let preserved = &settings.user_secret_regex_list[0];
            assert_eq!(preserved.pattern.as_str(), original_rule.pattern.as_str());
            assert_eq!(preserved.name, original_rule.name);

            settings.maybe_sync_with_warp_drive_prefs(ctx);
            assert_eq!(settings.user_secret_regex_list.len(), 1);
            assert_eq!(settings.user_secret_regex_list[0].name, original_rule.name);
        });
    });
}
