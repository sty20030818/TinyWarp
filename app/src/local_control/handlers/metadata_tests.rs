use super::{SurfaceDestination, surface_unavailable_reason};
use crate::features::FeatureFlag;

#[test]
fn agent_management_surface_reports_feature_flag_unavailable() {
    let flag_guard = FeatureFlag::AgentManagementView.override_enabled(false);
    warpui::App::test((), |mut app| async move {
        assert_eq!(
            app.update(|ctx| {
                surface_unavailable_reason(SurfaceDestination::AgentManagement, ctx)
            }),
            Some("agent management is unavailable or disabled")
        );
    });
    drop(flag_guard);
}

#[test]
fn local_terminal_surface_metadata_disables_features_but_keeps_terminal_controls() {
    warpui::App::test((), |mut app| async move {
        app.update(crate::local_terminal::enable_for_test);
        app.read(|ctx| {
            for surface in [
                SurfaceDestination::WarpDrive,
                SurfaceDestination::AiAssistant,
                SurfaceDestination::CodeReview,
                SurfaceDestination::ProjectExplorer,
                SurfaceDestination::GlobalSearch,
                SurfaceDestination::ConversationList,
                SurfaceDestination::LeftPanel,
                SurfaceDestination::RightPanel,
                SurfaceDestination::AgentManagement,
            ] {
                assert!(surface_unavailable_reason(surface, ctx).is_some());
            }
            for surface in [
                SurfaceDestination::Settings,
                SurfaceDestination::CommandPalette,
                SurfaceDestination::CommandSearch,
                SurfaceDestination::ThemePicker,
                SurfaceDestination::Keybindings,
                SurfaceDestination::ResourceCenter,
            ] {
                assert!(surface_unavailable_reason(surface, ctx).is_none());
            }
        });
    });
}
