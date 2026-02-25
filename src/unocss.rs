use std::{env, fs};
use zed_extension_api::{self as zed, Result};

const SERVER_PATH: &str = "node_modules/@bajrangcoder/unocss-language-server/bin/index.js";
const PACKAGE_NAME: &str = "@bajrangcoder/unocss-language-server";

pub struct UnoCSSExtension {
    did_find_server: bool,
}

impl UnoCSSExtension {
    fn server_exists(&self) -> bool {
        fs::metadata(SERVER_PATH).map_or(false, |stat| stat.is_file())
    }

    fn server_script_path(&mut self, id: &zed::LanguageServerId) -> Result<String> {
        let server_exists = self.server_exists();
        if self.did_find_server && server_exists {
            return Ok(SERVER_PATH.to_string());
        }

        zed::set_language_server_installation_status(
            id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let version = zed::npm_package_latest_version(PACKAGE_NAME)?;
        let installed = zed::npm_package_installed_version(PACKAGE_NAME)?;

        if !server_exists || installed.as_deref() != Some(version.as_str()) {
            zed::set_language_server_installation_status(
                id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            let install_result = zed::npm_install_package(PACKAGE_NAME, &version);
            match install_result {
                Ok(()) => {
                    if !self.server_exists() {
                        return Err(format!(
                            "installed package '{PACKAGE_NAME}' did not contain expected path '{SERVER_PATH}'"
                        ));
                    }
                }
                Err(error) => {
                    if !self.server_exists() {
                        return Err(error);
                    }
                }
            }
        }

        self.did_find_server = true;
        Ok(SERVER_PATH.to_string())
    }

    fn absolute_server_script_path(&mut self, id: &zed::LanguageServerId) -> Result<String> {
        let rel = self.server_script_path(id)?;
        let cwd = env::current_dir().map_err(|e| format!("failed to get extension cwd: {e}"))?;
        Ok(cwd.join(rel).to_string_lossy().to_string())
    }
}

impl zed::Extension for UnoCSSExtension {
    fn new() -> Self {
        Self {
            did_find_server: false,
        }
    }

    fn language_server_command(
        &mut self,
        id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let server_path = self.absolute_server_script_path(id)?;
        Ok(zed::Command {
            command: zed::node_binary_path()?,
            args: vec![server_path],
            env: worktree.shell_env(),
        })
    }

    fn language_server_initialization_options(
        &mut self,
        id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        let lsp = zed::settings::LspSettings::for_worktree(id.as_ref(), worktree)?;
        Ok(lsp.initialization_options)
    }

    fn language_server_workspace_configuration(
        &mut self,
        id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        let lsp = zed::settings::LspSettings::for_worktree(id.as_ref(), worktree)?;
        let Some(settings) = lsp.settings else {
            return Ok(None);
        };

        // Server expects settings under `unocss`.
        if settings.get("unocss").is_some() {
            Ok(Some(settings))
        } else {
            Ok(Some(zed::serde_json::json!({ "unocss": settings })))
        }
    }
}

zed::register_extension!(UnoCSSExtension);
