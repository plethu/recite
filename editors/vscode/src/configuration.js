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

function projectRootPath(api, configured, userInterface) {
  const workspace = api.workspace.workspaceFolders?.[0]?.uri.fsPath;
  if (!configured.trim()) return workspace;
  if (path.isAbsolute(configured)) return path.normalize(configured);
  if (!workspace) {
    throw userInterface.configurationProjectRootNeedsWorkspace();
  }
  return path.resolve(workspace, configured);
}

function resolveCommand(command, root) {
  if (!root || (!path.isAbsolute(command) && !command.includes("/") && !command.includes("\\"))) return command;
  return path.resolve(root, command);
}
