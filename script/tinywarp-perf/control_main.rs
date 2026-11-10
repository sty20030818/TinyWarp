use warp_core::AppId;
use warp_core::channel::{Channel, ChannelConfig, ChannelState, OzConfig, WarpServerConfig};
use warp_core::features::FeatureFlag;

// 复用已优化的应用库；仅测量入口额外开放本地控制功能，启用权限仍走原有 UI。
fn main() {
    ChannelState::set(
        ChannelState::new(
            Channel::Oss,
            ChannelConfig {
                app_id: AppId::new("dev", "warp", "TinyWarpL3AControl"),
                logfile_name: "tinywarp-perf-control.log".into(),
                server_config: WarpServerConfig::production(),
                oz_config: OzConfig::production(),
                telemetry_config: None,
                crash_reporting_config: None,
                autoupdate_config: None,
                mcp_static_config: None,
            },
        )
        .with_additional_features(&[FeatureFlag::WarpControlCli]),
    );
    if let Err(error) = warp::run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
