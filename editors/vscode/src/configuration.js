import * as path from "node:path";

export function readConfiguration(api, userInterface) {
  const settings = api.workspace.getConfiguration("recite");
  const command = settings.get("lsp.path", "recite-lsp");
  const args = settings.get("lsp.args", []);
  const projectRoot = settings.get("lsp.projectRoot", "");
  if (typeof command !== "string" || command.trim() === "") {
    throw userInterface.configurationPathInvalid();
  }
  if (!Array.isArray(args) || args.some((arg) => typeof arg !== "string")) {
    throw userInterface.configurationArgsInvalid();
  }
  if (typeof projectRoot !== "string") {
    throw userInterface.configurationProjectRootInvalid();
  }
  const root = projectRootPath(api, projectRoot, userInterface);
  return {
    command: resolveCommand(command, root),
    args,
    cwd: root,
    projectRoot: root,
    projectRootOverridden: Boolean(projectRoot.trim())
  };
}

/**
 * Read the local CLI executable without sharing the LSP's process settings.
 * The project root remains one authority: the existing LSP project-root
 * setting is also the effective root for editor commands.
 */
export function readCliConfiguration(api, userInterface) {
  const settings = api.workspace.getConfiguration("recite");
  const command = settings.get("cli.path", "recite");
  const projectRoot = settings.get("lsp.projectRoot", "");
  if (typeof command !== "string" || command.trim() === "") {
    throw userInterface.cliPathInvalid();
  }
  if (typeof projectRoot !== "string") {
    throw userInterface.configurationProjectRootInvalid();
  }
  const workspace = api.workspace.workspaceFolders?.[0]?.uri.fsPath;
  if (!workspace && (!projectRoot.trim() || !path.isAbsolute(projectRoot))) {
    throw userInterface.commandWorkspaceRequired();
  }
  const root = projectRootPath(api, projectRoot, userInterface);
  if (!root) throw userInterface.commandWorkspaceRequired();
  return {
    command: resolveCommand(command, root),
    cwd: root,
    projectRoot: root,
    projectRootOverridden: Boolean(projectRoot.trim())
  };
}

export function initializeParams(api, root, override = false) {
  const workspaceFolders = override
    ? [{ name: path.basename(root), uri: api.Uri.file(root).toString() }]
    : (api.workspace.workspaceFolders ?? []).map((folder) => ({
    uri: folder.uri.toString(),
    name: folder.name
  }));
  return {
    processId: process.pid,
    rootUri: root ? api.Uri.file(root).toString() : null,
    rootPath: root ?? null,
    workspaceFolders,
    capabilities: {
      general: { positionEncodings: ["utf-16"] },
      workspace: {
        configuration: true,
        didChangeWatchedFiles: { dynamicRegistration: true }
      },
      textDocument: {
        synchronization: { dynamicRegistration: true, willSave: false, willSaveWaitUntil: false, didSave: true },
        completion: { completionItem: { snippetSupport: false } },
        codeAction: {},
        definition: {},
        hover: {},
        references: {}
      }
    },
    trace: "off"
  };
}

export function projectRootPath(api, configured, userInterface) {
  const workspace = api.workspace.workspaceFolders?.[0]?.uri.fsPath;
  if (!configured.trim()) return workspace;
  if (path.isAbsolute(configured)) return path.normalize(configured);
  if (!workspace) {
    throw userInterface.configurationProjectRootNeedsWorkspace();
  }
  return path.resolve(workspace, configured);
}

export function resolveCommand(command, root) {
  if (!root || (!path.isAbsolute(command) && !command.includes("/") && !command.includes("\\"))) return command;
  return path.resolve(root, command);
}
