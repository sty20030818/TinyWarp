use warp_core::channel::{Channel, ChannelState};
use warpui::{AppContext, Entity, SingletonEntity};

use crate::LaunchMode;
use crate::view_components::DismissibleToast;
use crate::workspace::{ToastStack, WorkspaceAction};

struct LocalTerminal {
    enabled: bool,
}

impl Entity for LocalTerminal {
    type Event = ();
}

impl SingletonEntity for LocalTerminal {}

fn enabled_for_launch(channel: Channel, launch_mode: &LaunchMode) -> bool {
    channel == Channel::Oss
        && matches!(
            launch_mode,
            LaunchMode::App { .. }
                | LaunchMode::Test {
                    local_terminal: true,
                    ..
                }
        )
}

pub(crate) fn initialize(launch_mode: &LaunchMode, ctx: &mut AppContext) {
    let enabled = enabled_for_launch(ChannelState::channel(), launch_mode);
    ctx.add_singleton_model(|_| LocalTerminal { enabled });
}

pub(crate) fn is_enabled(ctx: &AppContext) -> bool {
    ctx.has_singleton_model::<LocalTerminal>() && LocalTerminal::as_ref(ctx).enabled
}

pub(crate) fn show_unavailable(ctx: &mut AppContext) {
    let message = warp_i18n::tr(
        "local_terminal.unavailable",
        "TinyWarp local terminal does not provide sign-in, accounts, or cloud services.",
    );
    let window_id = ctx
        .windows()
        .frontmost_window_id()
        .or_else(|| ctx.window_ids().next());
    if let Some(window_id) = window_id
        && ctx.has_singleton_model::<ToastStack>()
    {
        let toast: DismissibleToast<WorkspaceAction> = DismissibleToast::error(message.to_owned());
        ToastStack::handle(ctx).update(ctx, |stack, ctx| {
            stack.add_ephemeral_toast(toast, window_id, ctx);
        });
    } else {
        log::info!("{message}");
    }
}

#[cfg(test)]
pub(crate) fn enable_for_test(ctx: &mut AppContext) {
    ctx.add_singleton_model(|_| LocalTerminal { enabled: true });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_terminal_applies_only_to_oss_gui() {
        let gui = LaunchMode::App {
            args: Default::default(),
            api_key: None,
        };
        assert!(enabled_for_launch(Channel::Oss, &gui));
        assert!(!enabled_for_launch(Channel::Preview, &gui));
        assert!(!enabled_for_launch(Channel::Integration, &gui));
        assert!(!enabled_for_launch(
            Channel::Oss,
            &LaunchMode::RemoteServerProxy
        ));
        assert!(!enabled_for_launch(
            Channel::Oss,
            &LaunchMode::new_for_unit_test()
        ));
    }

    #[test]
    fn local_terminal_integration_requires_explicit_startup_mode() {
        let test = LaunchMode::Test {
            driver: Box::new(None),
            is_integration_test: true,
            local_terminal: true,
        };
        assert!(enabled_for_launch(Channel::Oss, &test));
        assert!(!enabled_for_launch(Channel::Integration, &test));
    }
}

#[cfg(test)]
mod sdk_mode_tests {
    use clap::Parser as _;

    use super::*;

    #[test]
    fn local_terminal_does_not_disable_sdk_gui_authentication() {
        let args =
            warp_cli::Args::try_parse_from(["oz", "agent", "run", "--gui", "--prompt", "fixture"])
                .expect("GUI SDK arguments should parse");
        let Some(warp_cli::Command::CommandLine(command)) = args.command() else {
            panic!("expected SDK command");
        };
        let mode = LaunchMode::command_line(
            command.as_ref().clone(),
            warp_cli::GlobalOptions {
                api_key: Some("fixture-sdk-api-key".to_owned()),
                ..Default::default()
            },
            false,
        );
        assert!(mode.is_gui());
        assert!(!enabled_for_launch(Channel::Oss, &mode));
        assert!(
            matches!(mode.auth_initialization(), crate::AuthInitialization::PendingApiKey(key) if key == "fixture-sdk-api-key")
        );
    }
}
