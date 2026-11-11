use std::sync::Arc;

use super::LSPServerType;

#[test]
fn persisted_server_types_remain_readable_without_installers() {
    let names = [
        "RustAnalyzer",
        "GoPls",
        "Pyright",
        "TypeScriptLanguageServer",
        "Clangd",
    ];
    let servers: Vec<_> = LSPServerType::all().collect();
    assert_eq!(servers.len(), names.len());
    for (server, name) in servers.into_iter().zip(names) {
        let encoded = serde_json::to_string(&server).unwrap();
        assert_eq!(encoded, format!("\"{name}\""));
        assert_eq!(
            serde_json::from_str::<LSPServerType>(&encoded).unwrap(),
            server
        );
        assert!(!server.languages().is_empty());
    }
}

#[test]
fn server_candidates_follow_build_capability() {
    let client = Arc::new(http_client::Client::new());
    for server in LSPServerType::all() {
        let result = server.candidate(client.clone());
        #[cfg(feature = "builtin_servers")]
        assert!(result.is_ok());
        #[cfg(not(feature = "builtin_servers"))]
        {
            let error = result
                .err()
                .expect("精简构建必须拒绝内置服务器")
                .to_string();
            assert!(error.contains("not included"));
            assert!(error.contains(server.binary_name()));
        }
    }
}
