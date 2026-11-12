use std::sync::LazyLock;

use anyhow::{Context as _, Result, ensure};
use semver::Version;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BuildInfo {
    version: String,
    build: u64,
}

static BUILD_INFO: LazyLock<BuildInfo> = LazyLock::new(|| {
    let info: BuildInfo = serde_json::from_str(include_str!("../../tinywarp-version.json"))
        .expect("TinyWarp 构建信息必须有效");
    let version = Version::parse(&info.version).expect("TinyWarp 产品版本必须有效");
    assert!(version.pre.is_empty() && version.build.is_empty() && info.build > 0);
    info
});

pub(crate) fn parse(tag: &str) -> Result<Version> {
    ensure!(tag.len() <= 128, "版本号过长");
    let version = Version::parse(tag.strip_prefix('v').context("版本标签必须以 v 开头")?)?;
    ensure!(version.build.is_empty(), "构建号应独立存储");
    if !version.pre.is_empty() {
        let sequence = version
            .pre
            .as_str()
            .strip_prefix("dev.")
            .context("仅支持本地 dev.N 开发版本")?;
        ensure!(
            sequence.bytes().all(|b| b.is_ascii_digit()) && sequence.parse::<u64>()? > 0,
            "无效开发版本序号"
        );
    }
    Ok(version)
}

pub(crate) fn newer(incoming: &str, current: &str) -> Result<bool> {
    Ok(parse(incoming)? > parse(current)?)
}

pub(crate) fn is_legacy(tag: &str) -> bool {
    tag.starts_with("v0.")
        && tag.contains(".tinywarp_")
        && tag
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._".contains(&b))
        && channel_versions::ParsedVersion::try_from(tag).is_ok()
}

pub(crate) fn display_label(tag: &str) -> String {
    format!(
        "{}（构建 {}）",
        tag.strip_prefix('v').unwrap_or(tag),
        BUILD_INFO.build
    )
}

#[cfg(test)]
#[path = "tinywarp_version_tests.rs"]
mod tests;
