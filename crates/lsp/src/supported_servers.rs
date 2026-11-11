#[cfg(all(not(target_arch = "wasm32"), feature = "builtin_servers"))]
use std::path::PathBuf;
use std::sync::Arc;

#[cfg(all(not(target_arch = "wasm32"), feature = "builtin_servers"))]
use command::r#async::Command;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[cfg(all(not(target_arch = "wasm32"), feature = "builtin_servers"))]
use crate::CommandBuilder;
#[cfg(feature = "builtin_servers")]
use crate::servers::clangd::ClangdCandidate;
#[cfg(feature = "builtin_servers")]
use crate::servers::go::GoPlsCandidate;
#[cfg(feature = "builtin_servers")]
use crate::servers::pyright::PyrightCandidate;
#[cfg(feature = "builtin_servers")]
use crate::servers::rust::RustAnalyzerCandidate;
#[cfg(feature = "builtin_servers")]
use crate::servers::typescript_language_server::TypeScriptLanguageServerCandidate;
use crate::{LanguageId, LanguageServerCandidate};

/// Configuration for a custom LSP binary installation.
///
/// For most LSP servers, we just need the binary path. However, for Node.js-based
/// servers like Pyright, we need to run `node langserver.index.js --stdio` instead
/// of relying on the wrapper script (which has a shebang that requires node in PATH).
#[cfg(all(not(target_arch = "wasm32"), feature = "builtin_servers"))]
#[derive(Debug, Clone)]
pub struct CustomBinaryConfig {
    /// The path to the executable (e.g., node binary or rust-analyzer binary)
    pub binary_path: PathBuf,
    /// Additional arguments to pass before any server-specific args (e.g., the JS file path)
    pub prepend_args: Vec<String>,
}

/// Represents the different types of LSP servers supported by Warp.
///
/// This is also used in underlying sqlite type persistence. We should be careful
/// not to rename an existing variant, as it will break persistence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter)]
pub enum LSPServerType {
    RustAnalyzer,
    GoPls,
    Pyright,
    TypeScriptLanguageServer,
    Clangd,
}

/// Provides server-specific configuration for each LSP server type.
impl LSPServerType {
    /// Creates a properly configured Command for this LSP server type.
    ///
    /// Uses `CommandBuilder` to create the command, which ensures `.cmd`/`.bat`
    /// scripts are resolved on Windows and PATH is set correctly.
    ///
    /// If a custom binary config is provided (e.g., from our data_dir installation),
    /// it will be used. Otherwise, falls back to the system PATH.
    #[cfg(all(not(target_arch = "wasm32"), feature = "builtin_servers"))]
    pub(crate) fn create_command(
        &self,
        custom_config: Option<CustomBinaryConfig>,
        executor: &CommandBuilder,
    ) -> Command {
        if let Some(config) = custom_config {
            let mut command = executor.command(&config.binary_path);
            command.args(&config.prepend_args);
            command.args(self.custom_install_args());
            command
        } else {
            let mut command = executor.command(self.binary_name());
            command.args(self.args());
            command
        }
    }

    /// Finds the configuration for a custom-installed binary in the data directory.
    ///
    /// This checks our custom installation location (`{data_dir}/{server_name}/`).
    /// For Node.js-based servers, this returns the node binary path plus the JS file as args.
    ///
    /// # Arguments
    /// * `path_env_var` - The PATH environment variable to use when checking for system dependencies
    ///   (e.g., system node for pyright).
    #[cfg(all(not(target_arch = "wasm32"), feature = "builtin_servers"))]
    pub async fn find_installed_binary_config(
        &self,
        path_env_var: Option<&str>,
    ) -> Option<CustomBinaryConfig> {
        match self {
            LSPServerType::RustAnalyzer => {
                RustAnalyzerCandidate::find_installed_binary_in_data_dir()
                    .await
                    .map(|path| CustomBinaryConfig {
                        binary_path: path,
                        prepend_args: vec![],
                    })
            }
            LSPServerType::GoPls => {
                // gopls doesn't support custom installation yet
                None
            }
            LSPServerType::Pyright => {
                PyrightCandidate::find_installed_binary_config(path_env_var).await
            }
            LSPServerType::TypeScriptLanguageServer => {
                TypeScriptLanguageServerCandidate::find_installed_binary_config(path_env_var).await
            }
            LSPServerType::Clangd => ClangdCandidate::find_installed_binary_in_data_dir()
                .await
                .map(|path| CustomBinaryConfig {
                    binary_path: path,
                    prepend_args: vec![],
                }),
        }
    }

    /// Checks if the binary works on the given PATH by running a version/help command.
    ///
    /// Delegates to each server's candidate implementation.
    /// Returns true only if the binary executes successfully with exit code 0.
    #[cfg(all(not(target_arch = "wasm32"), feature = "builtin_servers"))]
    pub async fn is_working_on_path(
        &self,
        executor: &CommandBuilder,
        client: Arc<http_client::Client>,
    ) -> anyhow::Result<bool> {
        Ok(self.candidate(client)?.is_installed_on_path(executor).await)
    }

    pub fn binary_name(&self) -> &'static str {
        match self {
            LSPServerType::RustAnalyzer => "rust-analyzer",
            LSPServerType::GoPls => "gopls",
            LSPServerType::Pyright => "pyright-langserver",
            LSPServerType::TypeScriptLanguageServer => "typescript-language-server",
            LSPServerType::Clangd => "clangd",
        }
    }

    /// Arguments for running via system PATH.
    #[cfg(all(not(target_arch = "wasm32"), feature = "builtin_servers"))]
    fn args(&self) -> Vec<&'static str> {
        match self {
            LSPServerType::RustAnalyzer | LSPServerType::GoPls | LSPServerType::Clangd => vec![],
            LSPServerType::Pyright | LSPServerType::TypeScriptLanguageServer => vec!["--stdio"],
        }
    }

    /// Arguments for running from a custom installation.
    /// These are added after any prepend_args from CustomBinaryConfig.
    #[cfg(all(not(target_arch = "wasm32"), feature = "builtin_servers"))]
    fn custom_install_args(&self) -> Vec<&'static str> {
        match self {
            LSPServerType::RustAnalyzer => vec![],
            LSPServerType::GoPls => vec![],
            LSPServerType::Pyright => vec!["--stdio"],
            LSPServerType::TypeScriptLanguageServer => vec!["--stdio"],
            LSPServerType::Clangd => vec![],
        }
    }

    /// Returns the languages supported by this LSP server.
    pub fn languages(&self) -> Vec<LanguageId> {
        match self {
            LSPServerType::RustAnalyzer => vec![LanguageId::Rust],
            LSPServerType::GoPls => vec![LanguageId::Go],
            LSPServerType::Pyright => vec![LanguageId::Python],
            LSPServerType::TypeScriptLanguageServer => {
                vec![
                    LanguageId::TypeScript,
                    LanguageId::TypeScriptReact,
                    LanguageId::JavaScript,
                    LanguageId::JavaScriptReact,
                ]
            }
            LSPServerType::Clangd => vec![LanguageId::C, LanguageId::Cpp],
        }
    }

    /// Returns a display name for the languages supported by this server.
    /// For multi-language servers, returns "Language1/Language2".
    pub fn language_name(&self) -> String {
        match self {
            LSPServerType::TypeScriptLanguageServer => "TypeScript/JavaScript".to_string(),
            _ => self
                .languages()
                .iter()
                .map(|lang| {
                    let id = lang.lsp_language_identifier();
                    let mut chars = id.chars();
                    // Capitalize the first character.
                    match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    }
                })
                .join("/"),
        }
    }

    /// 未包含内置服务器的构建显式拒绝发现和安装，避免误用旧配置。
    pub fn candidate(
        &self,
        client: Arc<http_client::Client>,
    ) -> anyhow::Result<Box<dyn LanguageServerCandidate>> {
        #[cfg(feature = "builtin_servers")]
        {
            let candidate: Box<dyn LanguageServerCandidate> = match self {
                LSPServerType::RustAnalyzer => Box::new(RustAnalyzerCandidate::new(client)),
                LSPServerType::GoPls => Box::new(GoPlsCandidate::new(client)),
                LSPServerType::Pyright => Box::new(PyrightCandidate::new(client)),
                LSPServerType::TypeScriptLanguageServer => {
                    Box::new(TypeScriptLanguageServerCandidate::new(client))
                }
                LSPServerType::Clangd => Box::new(ClangdCandidate::new(client)),
            };
            Ok(candidate)
        }
        #[cfg(not(feature = "builtin_servers"))]
        {
            drop(client);
            anyhow::bail!(
                "Built-in language servers are not included in this build: {}",
                self.binary_name()
            )
        }
    }

    pub fn all() -> impl Iterator<Item = LSPServerType> {
        LSPServerType::iter()
    }
}

#[cfg(test)]
#[path = "supported_servers_tests.rs"]
mod tests;
