use std::io::{Read as _, Write as _};
use std::net::TcpListener;
use std::thread;

use super::*;
use crate::autoupdate::tinywarp::checked;

fn payload() -> String {
    serde_json::json!({"version":"v0.2026.11.12.00.00.tinywarp_01","architecture":std::env::consts::ARCH,"sha256":"a".repeat(64)}).to_string()
}

fn key(directory: &Path, name: &str) -> (std::path::PathBuf, String) {
    let path = directory.join(name);
    checked(
        Command::new("/usr/bin/ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-f"])
            .arg(&path),
    )
    .unwrap();
    let public = fs::read_to_string(path.with_extension("pub")).unwrap();
    (path, public)
}

fn signed(directory: &Path, key: &Path, namespace: &str, payload: String) -> SignedRelease {
    let path = directory.join("manifest.json");
    fs::write(&path, &payload).unwrap();
    checked(
        Command::new("/usr/bin/ssh-keygen")
            .args(["-Y", "sign", "-f"])
            .arg(key)
            .args(["-n", namespace])
            .arg(&path),
    )
    .unwrap();
    SignedRelease {
        payload,
        signature: fs::read_to_string(directory.join("manifest.json.sig")).unwrap(),
    }
}

#[test]
fn signed_manifest_roundtrip_and_cached_envelope_preserve_exact_payload() {
    let directory = tempfile::tempdir().unwrap();
    let (key, public) = key(directory.path(), "signing");
    let document = signed(directory.path(), &key, NAMESPACE, payload());
    let encoded = serde_json::to_vec(&document).unwrap();
    let restored: SignedRelease = serde_json::from_slice(&encoded).unwrap();
    let release = authenticated_release(&restored, &public).unwrap();
    assert_eq!(release.sha256, "a".repeat(64));
    assert!(archive_url(&release).unwrap().starts_with(&format!(
        "{RELEASES}/download/v0.2026.11.12.00.00.tinywarp_01/"
    )));
}

#[test]
fn tampered_signed_payload_and_untrusted_key_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let (key, public) = key(directory.path(), "signing");
    let document = signed(directory.path(), &key, NAMESPACE, payload());
    let (_, other_public) = self::key(directory.path(), "other");
    assert!(authenticated_release(&document, &other_public).is_err());
    let changed = SignedRelease {
        payload: format!("{} ", document.payload),
        signature: document.signature,
    };
    assert!(authenticated_release(&changed, &public).is_err());
}

#[test]
fn signatures_from_another_namespace_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let (key, public) = key(directory.path(), "signing");
    let document = signed(directory.path(), &key, "ssh", payload());
    assert!(authenticated_release(&document, &public).is_err());
}

#[test]
fn signed_but_invalid_release_fields_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let (key, public) = key(directory.path(), "signing");
    let document = signed(
        directory.path(),
        &key,
        NAMESPACE,
        payload().replace(&"a".repeat(64), "invalid"),
    );
    assert!(authenticated_release(&document, &public).is_err());
    assert!(verify_signature(&vec![0; MAX_DOCUMENT_BYTES + 1], b"", &public).is_err());
}

fn server(status: u16, body: Vec<u8>) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/manifest", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let handle = thread::spawn(move || {
        let deadline = instant::Instant::now() + Duration::from_secs(5);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(instant::Instant::now() < deadline, "测试 HTTP 连接超时");
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("{error}"),
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut request = Vec::new();
        let mut buffer = [0; 2048];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            let count = stream.read(&mut buffer).unwrap();
            assert!(count > 0);
            request.extend_from_slice(&buffer[..count]);
        }
        write!(
            stream,
            "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .unwrap();
        // 超大响应可能被客户端提前断开。
        let _ = stream.write_all(&body);
        String::from_utf8(request).unwrap()
    });
    (url, handle)
}

#[test]
fn anonymous_document_fetch_and_missing_release_semantics() {
    for (status, allow_missing, expected) in [(200, true, Some("{}")), (404, true, None)] {
        let (url, server) = server(status, b"{}".to_vec());
        let value = futures::executor::block_on(fetch_document(
            &Client::new_external(),
            &url,
            allow_missing,
        ))
        .unwrap();
        assert_eq!(value.as_deref(), expected);
        let request = server.join().unwrap().to_ascii_lowercase();
        assert!(
            !request.contains("authorization:")
                && !request.contains("x-warp-")
                && !request.contains("cookie:")
        );
    }
}

#[test]
fn missing_signature_http_failure_and_oversized_document_are_errors() {
    for (status, body) in [
        (404, b"missing".to_vec()),
        (503, b"failure".to_vec()),
        (200, vec![b'a'; MAX_DOCUMENT_BYTES + 1]),
    ] {
        let (url, server) = server(status, body);
        assert!(
            futures::executor::block_on(fetch_document(&Client::new_external(), &url, false))
                .is_err()
        );
        server.join().unwrap();
    }
}
