use std::fs::{self, File, OpenOptions};
use std::io::Write as _;
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context as _, Result, ensure};
use async_compat::Compat;
use command::blocking::Command;
use futures::StreamExt as _;
use http_client::{Client, Response, StatusCode};
use serde::{Deserialize, Serialize};

use super::{LocalRelease, private_dir, root, trusted_path, validate_release};

const RELEASES: &str = "https://github.com/sty20030818/TinyWarp/releases";
const NAMESPACE: &str = "tinywarp-updates";
const PUBLIC_KEY: &str = include_str!("../../../resources/tinywarp-updates.pub");
const MAX_DOCUMENT_BYTES: usize = 4096;
const MAX_ARCHIVE_BYTES: usize = 1024 * 1024 * 1024;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SignedRelease {
    payload: String,
    signature: String,
}

fn manifest_name() -> String {
    format!("tinywarp-update-{}.json", std::env::consts::ARCH)
}

fn archive_url(release: &LocalRelease) -> Result<String> {
    validate_release(release)?;
    Ok(format!(
        "{RELEASES}/download/{}/TinyWarp-{}.zip",
        release.version, release.architecture
    ))
}

fn verify_signature(payload: &[u8], signature: &[u8], public_key: &str) -> Result<()> {
    ensure!(
        payload.len() <= MAX_DOCUMENT_BYTES && signature.len() <= MAX_DOCUMENT_BYTES,
        "更新签名或清单过大"
    );
    let directory = tempfile::tempdir()?;
    let allowed = directory.path().join("allowed-signers");
    let signature_path = directory.path().join("manifest.sig");
    fs::write(
        &allowed,
        format!(
            "tinywarp namespaces=\"{NAMESPACE}\" {}\n",
            public_key.trim()
        ),
    )?;
    fs::write(&signature_path, signature)?;
    let mut child = Command::new("/usr/bin/ssh-keygen")
        .args(["-Y", "verify", "-f"])
        .arg(allowed)
        .args(["-I", "tinywarp", "-n", NAMESPACE, "-s"])
        .arg(signature_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .context("签名验证输入不可用")?
        .write_all(payload)?;
    let output = child.wait_with_output()?;
    ensure!(output.status.success(), "TinyWarp 更新签名无效");
    Ok(())
}

fn authenticated_release(document: &SignedRelease, public_key: &str) -> Result<LocalRelease> {
    verify_signature(
        document.payload.as_bytes(),
        document.signature.as_bytes(),
        public_key,
    )?;
    let release = serde_json::from_str(&document.payload)?;
    validate_release(&release)?;
    Ok(release)
}

async fn write_bounded(
    response: Response,
    maximum: usize,
    writer: &mut impl std::io::Write,
) -> Result<()> {
    let response = response.error_for_status()?;
    Compat::new(async {
        let mut stream = response.bytes_stream();
        let mut count = 0usize;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            count = count.checked_add(chunk.len()).context("更新响应长度溢出")?;
            ensure!(count <= maximum, "更新响应超过大小限制");
            writer.write_all(&chunk)?;
        }
        Ok(())
    })
    .await
}

async fn fetch_document(client: &Client, url: &str, allow_missing: bool) -> Result<Option<String>> {
    let response = client
        .get(url)
        .timeout(Duration::from_secs(20))
        .send()
        .await?;
    if allow_missing && response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    write_bounded(response, MAX_DOCUMENT_BYTES, &mut bytes).await?;
    Ok(Some(String::from_utf8(bytes)?))
}

pub(super) async fn fetch_release() -> Result<Option<LocalRelease>> {
    let client = Client::new_external();
    let url = format!("{RELEASES}/latest/download/{}", manifest_name());
    let Some(payload) = fetch_document(&client, &url, true).await? else {
        return Ok(None);
    };
    let signature = fetch_document(&client, &format!("{url}.sig"), false)
        .await?
        .context("更新签名缺失")?;
    let document = SignedRelease { payload, signature };
    let release = authenticated_release(&document, PUBLIC_KEY)?;
    private_dir(&root())?;
    // 一次原子写入原始清单与签名，避免跨次检查产生混合缓存。
    let mut cache = tempfile::NamedTempFile::new_in(root())?;
    serde_json::to_writer(cache.as_file_mut(), &document)?;
    cache.as_file().sync_all()?;
    cache.persist(root().join("verified-release.json"))?;
    Ok(Some(release))
}

pub(super) fn cached_release(version: &str) -> Result<LocalRelease> {
    let path = root().join("verified-release.json");
    trusted_path(&root())?;
    trusted_path(&path)?;
    ensure!(
        fs::metadata(&path)?.len() <= (MAX_DOCUMENT_BYTES * 4) as u64,
        "更新缓存过大"
    );
    let document = serde_json::from_reader(File::open(path)?)?;
    let release = authenticated_release(&document, PUBLIC_KEY)?;
    ensure!(release.version == version, "更新清单已变化，请重新检查");
    Ok(release)
}

pub(super) async fn download_archive(release: &LocalRelease, destination: &Path) -> Result<()> {
    let response = Client::new_external()
        .get(&archive_url(release)?)
        .timeout(Duration::from_secs(600))
        .send()
        .await?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(destination)?;
    write_bounded(response, MAX_ARCHIVE_BYTES, &mut file).await?;
    file.sync_all()?;
    Ok(())
}

#[cfg(test)]
#[path = "tinywarp_remote_tests.rs"]
mod tests;
