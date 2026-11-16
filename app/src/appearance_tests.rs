use std::path::Path;

use settings::{PrivatePreferences, PublicPreferences, Setting, SettingsManager};
use warp_core::features::FeatureFlag;
use warp_core::user_preferences::GetUserPreferences;
use warpui::{App, SingletonEntity};
use warpui_extras::user_preferences::in_memory::InMemoryPreferences;
use warpui_extras::user_preferences::{Error, UserPreferences};

use super::sync_dock_icon_preference;
use crate::settings::app_icon::{AppIcon, AppIconSettings, AppIconState};

fn initialize_app(app: &mut App, selected: AppIcon, native: Box<dyn UserPreferences>) {
    app.add_singleton_model(move |_| {
        let preferences = InMemoryPreferences::default();
        preferences
            .write_value("AppIcon", serde_json::to_string(&selected).unwrap())
            .unwrap();
        PublicPreferences::new(Box::new(preferences))
    });
    app.add_singleton_model(move |_| PrivatePreferences::new(native));
    app.add_singleton_model(|_| SettingsManager::default());
    AppIconSettings::register(app);
}

fn assert_icon_mirror(selected: AppIcon, stale: AppIcon) {
    App::test((), |mut app| async move {
        let _settings_file = FeatureFlag::SettingsFile.override_enabled(true);
        let native = InMemoryPreferences::default();
        native
            .write_value("AppIcon", serde_json::to_string(&stale).unwrap())
            .unwrap();
        initialize_app(&mut app, selected, Box::new(native));

        app.read(|ctx| {
            sync_dock_icon_preference(ctx).unwrap();
            assert_eq!(*AppIconSettings::as_ref(ctx).app_icon.value(), selected);
            assert_eq!(
                ctx.private_user_preferences()
                    .read_value("AppIcon")
                    .unwrap(),
                Some(serde_json::to_string(&selected).unwrap()),
                "Dock 插件必须读取当前设置的派生镜像，不能继续使用旧 native 偏好"
            );
            assert_eq!(
                AppIconState::read_from_preferences(AppIconState::preferences_for_setting(ctx)),
                Some(selected),
                "镜像同步不得改写用户的设置来源"
            );
        });
    });
}

#[test]
fn selected_app_icon_replaces_stale_native_mirror() {
    assert_icon_mirror(AppIcon::Aurora, AppIcon::Neon);
}

#[test]
fn default_app_icon_replaces_previous_custom_mirror() {
    assert_icon_mirror(AppIcon::Default, AppIcon::Aurora);
}

#[test]
fn every_custom_app_icon_has_a_png_resource() {
    let resources = Path::new(env!("CARGO_MANIFEST_DIR")).join("DockTilePlugin/Resources");
    let mut checked = 0;
    for icon in enum_iterator::all::<AppIcon>().filter(|icon| *icon != AppIcon::Default) {
        let name = AppIconSettings::get_base_icon_file_name(icon);
        let bytes = std::fs::read(resources.join(format!("{name}.png")))
            .unwrap_or_else(|err| panic!("图标 {icon:?} 的 PNG 资源不可读：{err}"));
        assert!(
            bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
            "图标 {icon:?} 的资源必须有有效 PNG 文件签名"
        );
        checked += 1;
    }
    assert!(checked > 0, "必须实际核对非默认图标资源");
}

struct FailingPreferences;

impl UserPreferences for FailingPreferences {
    fn write_value(&self, _: &str, _: String) -> Result<(), Error> {
        Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied).into())
    }

    fn read_value(&self, _: &str) -> Result<Option<String>, Error> {
        Ok(None)
    }

    fn remove_value(&self, _: &str) -> Result<(), Error> {
        unreachable!("图标镜像同步不删除偏好")
    }
}

#[test]
fn app_icon_mirror_preserves_write_failure() {
    App::test((), |mut app| async move {
        let _settings_file = FeatureFlag::SettingsFile.override_enabled(true);
        initialize_app(&mut app, AppIcon::Neon, Box::new(FailingPreferences));

        app.read(|ctx| {
            let err = sync_dock_icon_preference(ctx).unwrap_err();
            assert!(matches!(
                err.downcast_ref::<Error>(),
                Some(Error::IoError(error))
                    if error.kind() == std::io::ErrorKind::PermissionDenied
            ));
            assert_eq!(
                *AppIconSettings::as_ref(ctx).app_icon.value(),
                AppIcon::Neon
            );
        });
    });
}
