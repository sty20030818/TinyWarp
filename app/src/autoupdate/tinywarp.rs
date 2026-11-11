use std::fs::{self, File, OpenOptions};
use std::io::{Read as _, Write as _};
use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context as _, Result, ensure};
use channel_versions::{ParsedVersion, VersionInfo};
use command::blocking::Command;
use instant::Instant;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use super::DownloadReady;
use crate::ChannelState;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LocalRelease {
    version: String,
    sha256: String,
    architecture: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InstallRequest {
    parent_pid: u32,
    bundle: PathBuf,
    staged: PathBuf,
    version: String,
}

fn root() -> PathBuf {
    warp_core::paths::state_dir().join("tinywarp-updates")
}

fn private_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    trusted_path(path)
}

fn trusted_path(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    ensure!(!metadata.file_type().is_symlink(), "更新路径不能是符号链接");
    ensure!(
        metadata.uid() == unsafe { libc::getuid() },
        "更新文件不属于当前用户"
    );
    ensure!(metadata.mode() & 0o077 == 0, "更新文件必须仅当前用户可访问");
    Ok(())
}

fn release() -> Result<Option<LocalRelease>> {
    let inbox = root().join("inbox");
    if !inbox.join("latest.json").exists() {
        return Ok(None);
    }
    trusted_path(&root())?;
    trusted_path(&inbox)?;
    trusted_path(&inbox.join("latest.json"))?;
    let metadata = fs::metadata(inbox.join("latest.json"))?;
    ensure!(metadata.len() <= 4096, "更新清单过大");
    let release: LocalRelease = serde_json::from_reader(File::open(inbox.join("latest.json"))?)?;
    validate_release(&release)?;
    Ok(Some(release))
}

fn validate_release(release: &LocalRelease) -> Result<()> {
    ParsedVersion::try_from(release.version.as_str())?;
    ensure!(
        release
            .version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._".contains(&b)),
        "无效版本号"
    );
    ensure!(
        release.architecture == std::env::consts::ARCH,
        "更新包架构不匹配"
    );
    ensure!(
        release.sha256.len() == 64 && release.sha256.bytes().all(|b| b.is_ascii_hexdigit()),
        "无效 SHA256"
    );
    Ok(())
}

pub(super) fn fetch_version() -> Result<VersionInfo> {
    let version = match release()? {
        Some(release) => release.version,
        None => ChannelState::app_version()
            .context("开发构建未设置 GIT_RELEASE_TAG")?
            .to_owned(),
    };
    Ok(serde_json::from_value(
        serde_json::json!({"version":version}),
    )?)
}

fn stage_dir(update_id: &str) -> PathBuf {
    root().join(format!("staged-{update_id}"))
}

fn sha256(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let size = file.read(&mut buffer)?;
        if size == 0 {
            break;
        }
        digest.update(&buffer[..size]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn checked(command: &mut Command) -> Result<()> {
    let output = command.output()?;
    ensure!(
        output.status.success(),
        "更新命令失败 {:?}: {}",
        command.get_program(),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn bundle_info(bundle: &Path) -> Result<plist::Dictionary> {
    plist::Value::from_file(bundle.join("Contents/Info.plist"))?
        .into_dictionary()
        .context("无效应用 Info.plist")
}

fn field<'a>(info: &'a plist::Dictionary, name: &str) -> Result<&'a str> {
    info.get(name)
        .and_then(plist::Value::as_string)
        .with_context(|| format!("缺少 {name}"))
}

fn verify_bundle(bundle: &Path, installed: &Path, version: &str) -> Result<()> {
    let info = bundle_info(bundle)?;
    let current = bundle_info(installed)?;
    ensure!(
        field(&info, "CFBundleIdentifier")? == field(&current, "CFBundleIdentifier")?,
        "更新包身份不匹配"
    );
    ensure!(
        field(&info, "CFBundleExecutable")? == "warp-oss",
        "更新包可执行文件不匹配"
    );
    ensure!(
        field(&info, "WarpVersion")? == version,
        "更新包版本与清单不一致"
    );
    let metadata: serde_json::Value = serde_json::from_reader(File::open(
        bundle.join("Contents/Resources/bundled/metadata/version.json"),
    )?)?;
    ensure!(
        metadata["warp_version"].as_str() == Some(version),
        "更新资源版本不一致"
    );
    checked(
        Command::new("/usr/bin/codesign")
            .args(["--verify", "--deep", "--strict"])
            .arg(bundle),
    )?;
    checked(
        Command::new("/usr/bin/lipo")
            .arg(bundle.join("Contents/MacOS/warp-oss"))
            .args([
                "-verify_arch",
                if cfg!(target_arch = "aarch64") {
                    "arm64"
                } else {
                    "x86_64"
                },
            ]),
    )
}

pub(super) fn stage_update(version: &VersionInfo, update_id: &str) -> Result<DownloadReady> {
    let release = release()?.context("本地更新清单不存在")?;
    ensure!(
        release.version == version.version,
        "更新源已变化，请重新检查"
    );
    let bundle = PathBuf::from(warp_core::macos::get_bundle_path()?);
    ensure!(
        permissions::is_writable(&bundle)?,
        "应用不可写，请移动到你可写的目录后重试"
    );
    let parent = bundle.parent().context("应用父目录不存在")?;
    let _probe = tempfile::Builder::new()
        .prefix(".tinywarp-write-check-")
        .tempfile_in(parent)
        .context("应用目录不可写，更新未准备，请移动到可写目录后重试")?;
    let destination = stage_dir(update_id);
    ensure!(!destination.exists(), "更新暂存目录已存在");
    private_dir(&destination)?;
    let result = (|| {
        let archive = root().join("inbox/update.zip");
        trusted_path(&archive)?;
        let snapshot = destination.join("update.zip");
        fs::copy(&archive, &snapshot)?;
        ensure!(
            sha256(&snapshot)?.eq_ignore_ascii_case(&release.sha256),
            "更新包校验失败"
        );
        checked(
            Command::new("/usr/bin/ditto")
                .args(["-x", "-k"])
                .arg(&snapshot)
                .arg(&destination),
        )?;
        verify_bundle(
            &destination.join("TinyWarp.app"),
            &PathBuf::from(warp_core::macos::get_bundle_path()?),
            &version.version,
        )?;
        Ok(DownloadReady::Yes)
    })();
    if result.is_err() {
        fs::remove_dir_all(destination)?;
    }
    result
}

pub(super) fn spawn_installer(version: &VersionInfo, update_id: &str) -> Result<()> {
    let bundle = PathBuf::from(warp_core::macos::get_bundle_path()?);
    let staged = stage_dir(update_id).join("TinyWarp.app");
    verify_bundle(&staged, &bundle, &version.version)?;
    let request = InstallRequest {
        parent_pid: std::process::id(),
        bundle,
        staged,
        version: version.version.clone(),
    };
    private_dir(&root())?;
    let request_path = root().join(format!("install-{update_id}.json"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&request_path)?;
    serde_json::to_writer(&mut file, &request)?;
    file.sync_all()?;
    Command::new(std::env::current_exe()?)
        .arg("--tinywarp-update-helper")
        .arg(request_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

fn exchange(first: &Path, second: &Path) -> Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt as _;
    let first = CString::new(first.as_os_str().as_bytes())?;
    let second = CString::new(second.as_os_str().as_bytes())?;
    nix::errno::Errno::result(unsafe {
        libc::renamex_np(first.as_ptr(), second.as_ptr(), libc::RENAME_SWAP)
    })?;
    Ok(())
}

fn install(request: &InstallRequest, request_path: &Path) -> Result<()> {
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(root().join("install.lock"))?;
    use std::os::fd::AsRawFd as _;
    ensure!(
        unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0,
        "另一个更新正在进行"
    );
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if unsafe { libc::kill(request.parent_pid as i32, 0) } != 0 {
            ensure!(
                nix::errno::Errno::last() == nix::errno::Errno::ESRCH,
                "无法确认原应用是否退出，更新未安装"
            );
            break;
        }
        ensure!(Instant::now() < deadline, "原应用仍在运行，更新未安装");
        std::thread::sleep(Duration::from_millis(200));
    }
    let preparation = (|| {
        verify_bundle(&request.staged, &request.bundle, &request.version)?;
        let parent = request.bundle.parent().context("应用父目录不存在")?;
        let backup = tempfile::Builder::new()
            .prefix(".tinywarp-previous-")
            .tempdir_in(parent)?
            .keep();
        let previous = backup.join("TinyWarp.app");
        checked(
            Command::new("/usr/bin/ditto")
                .arg(&request.staged)
                .arg(&previous),
        )?;
        verify_bundle(&previous, &request.bundle, &request.version)?;
        let mut journal = File::create(request_path.with_extension("result"))?;
        writeln!(
            journal,
            "prepared\n{}\nbackup={}\ntarget={}\n",
            request.version,
            previous.display(),
            request.bundle.display()
        )?;
        journal.sync_all()?;
        exchange(&previous, &request.bundle)?;
        Ok::<_, anyhow::Error>(previous)
    })();
    let previous = match preparation {
        Ok(previous) => previous,
        Err(error) => {
            Command::new(request.bundle.join("Contents/MacOS/warp-oss")).spawn()?;
            return Err(error);
        }
    };
    // 保留完整旧包，包含仍在运行的 helper；不回退任何数据库或用户配置。
    let launch = Command::new(request.bundle.join("Contents/MacOS/warp-oss"))
        .env("TINYWARP_UPDATE_ACK", request_path.with_extension("ready"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    let mut child = match launch {
        Ok(child) => child,
        Err(error) => {
            exchange(&previous, &request.bundle)?;
            Command::new(request.bundle.join("Contents/MacOS/warp-oss")).spawn()?;
            return Err(error.into());
        }
    };
    let deadline = Instant::now() + Duration::from_secs(60);
    let acknowledgement = request_path.with_extension("ready");
    while Instant::now() < deadline {
        if acknowledgement.exists() {
            fs::write(
                request_path.with_extension("result"),
                format!(
                    "installed\n{}\nbackup={}\n",
                    request.version,
                    previous.display()
                ),
            )?;
            return Ok(());
        }
        if child.try_wait()?.is_some() {
            exchange(&previous, &request.bundle)?;
            Command::new(request.bundle.join("Contents/MacOS/warp-oss")).spawn()?;
            anyhow::bail!("新应用启动失败，已回退应用包");
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    // 未响应的进程可能已有用户任务，不能强杀并回退正在使用的应用。
    anyhow::bail!("新应用未确认就绪；旧包保留于 {}", previous.display())
}

pub(crate) fn acknowledge_tinywarp_update() {
    if let Some(path) = std::env::var_os("TINYWARP_UPDATE_ACK") {
        let path = PathBuf::from(path);
        if path.parent() == Some(root().as_path())
            && path.extension().is_some_and(|ext| ext == "ready")
        {
            warp_errors::report_if_error!(
                fs::write(path, std::process::id().to_string()).context("写入更新就绪标记")
            );
        }
    }
}

/// 只处理本机更新 helper；调用发生在 GUI／账号／数据库初始化之前。
pub fn run_tinywarp_update_helper() -> Result<bool> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--tinywarp-update-helper")) {
        return Ok(false);
    }
    let path = PathBuf::from(args.next().context("缺少更新请求")?);
    ensure!(args.next().is_none(), "多余 helper 参数");
    ensure!(
        path.parent() == Some(root().as_path()),
        "更新请求目录不匹配"
    );
    trusted_path(&root())?;
    trusted_path(&path)?;
    let request: InstallRequest = serde_json::from_reader(File::open(&path)?)?;
    ensure!(request.parent_pid > 1, "无效父进程");
    ensure!(
        request.staged.starts_with(root())
            && request.bundle.extension().is_some_and(|ext| ext == "app"),
        "无效应用路径"
    );
    if let Err(error) = install(&request, &path) {
        fs::write(
            path.with_extension("result"),
            format!("failed\n{error:#}\n"),
        )?;
        return Err(error);
    }
    Ok(true)
}

#[cfg(test)]
#[path = "tinywarp_tests.rs"]
mod tests;
