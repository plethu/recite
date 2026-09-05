use zed_extension_api::{self as zed, LanguageServerId, Result, Worktree, settings::LspSettings};

mod launcher;
#[cfg(test)]
mod tests;

const LANGUAGE_SERVER_ID: &str = "recite-lsp";

struct ReciteExtension;

impl zed::Extension for ReciteExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<zed::Command> {
        if language_server_id.as_ref() != LANGUAGE_SERVER_ID {
            return Err(format!(
                "Recite Zed extension does not provide language server `{language_server_id}`"
            ));
        }

        let settings = LspSettings::for_worktree(LANGUAGE_SERVER_ID, worktree)?;
        let binary = settings.binary;
        let (path, arguments, environment) = binary.map_or_else(
            || (None, Vec::new(), Vec::new()),
            |binary| {
                (
                    binary.path,
                    binary.arguments.unwrap_or_default(),
                    binary.env.unwrap_or_default().into_iter().collect(),
                )
            },
        );
        let path_fallback = path
            .is_none()
            .then(|| worktree.which(LANGUAGE_SERVER_ID))
            .flatten();
        let command = launcher::resolve(
            launcher::LauncherSettings {
                path,
                arguments,
                environment,
            },
            path_fallback,
        )?;

        Ok(zed::Command {
            command: command.command,
            args: command.arguments,
            env: command.environment,
        })
    }
}

zed::register_extension!(ReciteExtension);
