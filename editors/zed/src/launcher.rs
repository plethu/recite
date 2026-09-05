#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) struct LauncherSettings {
    pub(crate) path: Option<String>,
    pub(crate) arguments: Vec<String>,
    pub(crate) environment: Vec<(String, String)>,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct LauncherCommand {
    pub(crate) command: String,
    pub(crate) arguments: Vec<String>,
    pub(crate) environment: Vec<(String, String)>,
}

pub(crate) fn resolve(
    settings: LauncherSettings,
    path_fallback: Option<String>,
) -> Result<LauncherCommand, String> {
    let command = match settings.path {
        Some(path) if path.trim().is_empty() => {
            return Err(
                "Recite LSP setting `binary.path` is empty; set it to an executable path or remove it to use `recite-lsp` from PATH".to_string(),
            );
        }
        Some(path) => path,
        None => path_fallback.ok_or_else(|| {
            "Recite requires `recite-lsp`. Install it separately and make it available on PATH, or configure `lsp.recite-lsp.binary.path` in Zed settings; the extension does not download or bundle the server.".to_string()
        })?,
    };

    let mut environment = settings.environment;
    environment.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(LauncherCommand {
        command,
        arguments: settings.arguments,
        environment,
    })
}
