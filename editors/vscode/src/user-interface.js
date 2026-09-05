import { clientMessage } from "./messages.js";

// This module is the only owner of VS Code-visible text and the output channel.
// Callers receive semantic operations, never the channel or a generic writer.
export function createUserInterface(api) {
  const output = api.window.createOutputChannel(
    clientMessage(api, "lsp-client-display-name")
  );
  return Object.freeze({
    serverTransportFailure(detail) {
      output.appendLine(clientMessage(api, "lsp-client-transport-failed", detail));
      api.window.showErrorMessage(clientMessage(api, "lsp-client-transport-failed", detail));
    },
    serverProtocolFailure() {
      output.appendLine(clientMessage(api, "lsp-client-protocol-failed"));
      api.window.showErrorMessage(clientMessage(api, "lsp-client-protocol-failed"));
    },
    serverLifecycleFailure(detail) {
      output.appendLine(clientMessage(api, "lsp-client-lifecycle-failed", detail));
      api.window.showErrorMessage(clientMessage(api, "lsp-client-lifecycle-failed", detail));
    },
    serverExited() {
      output.appendLine(clientMessage(api, "lsp-client-exited"));
      api.window.showErrorMessage(clientMessage(api, "lsp-client-exited"));
    },
    restartScheduled(milliseconds) {
      output.appendLine(clientMessage(api, "lsp-client-restart-scheduled", milliseconds));
    },
    restartExhausted() {
      output.appendLine(clientMessage(api, "lsp-client-restart-exhausted"));
      api.window.showErrorMessage(clientMessage(api, "lsp-client-restart-exhausted"));
    },
    actionStale() {
      output.appendLine(clientMessage(api, "lsp-client-action-stale"));
    },
    actionClosed() {
      output.appendLine(clientMessage(api, "lsp-client-action-closed"));
    },
    actionReopened() {
      output.appendLine(clientMessage(api, "lsp-client-action-reopened"));
    },
    actionExpired() {
      output.appendLine(clientMessage(api, "lsp-client-action-expired"));
    },
    actionEvicted() {
      output.appendLine(clientMessage(api, "lsp-client-action-evicted"));
    },
    actionApplyFailed() {
      output.appendLine(clientMessage(api, "lsp-client-action-apply-failed"));
    },
    actionUnknown() {
      output.appendLine(clientMessage(api, "lsp-client-action-unknown"));
    },
    configurationPathInvalid() {
      return new Error(clientMessage(api, "lsp-client-config-path-invalid"));
    },
    configurationArgsInvalid() {
      return new Error(clientMessage(api, "lsp-client-config-args-invalid"));
    },
    configurationProjectRootInvalid() {
      return new Error(clientMessage(api, "lsp-client-config-project-root-invalid"));
    },
    configurationProjectRootNeedsWorkspace() {
      return new Error(clientMessage(api, "lsp-client-config-project-root-needs-workspace"));
    },
    serverNotRunning() {
      return new Error(clientMessage(api, "lsp-client-not-running"));
    },
    renameBusy() {
      output.appendLine(clientMessage(api, "vscode-command-rename-busy"));
    },
    renameDocumentRequired() {
      output.appendLine(clientMessage(api, "vscode-command-rename-document-required"));
      api.window.showErrorMessage(clientMessage(api, "vscode-command-rename-document-required"));
    },
    renameUnavailable() {
      output.appendLine(clientMessage(api, "vscode-command-rename-unavailable"));
      api.window.showWarningMessage(clientMessage(api, "vscode-command-rename-unavailable"));
    },
    renameInvalid() {
      output.appendLine(clientMessage(api, "vscode-command-rename-invalid"));
      api.window.showErrorMessage(clientMessage(api, "vscode-command-rename-invalid"));
    },
    renameStale() {
      output.appendLine(clientMessage(api, "vscode-command-rename-stale"));
      api.window.showWarningMessage(clientMessage(api, "vscode-command-rename-stale"));
    },
    renameApplyFailed() {
      output.appendLine(clientMessage(api, "vscode-command-rename-apply-failed"));
      api.window.showErrorMessage(clientMessage(api, "vscode-command-rename-apply-failed"));
    },
    renameRequestFailed(detail) {
      output.appendLine(clientMessage(api, "vscode-command-rename-request-failed", detail));
      api.window.showErrorMessage(clientMessage(api, "vscode-command-rename-request-failed", detail));
    },
    serverStderr(message) {
      output.append(message);
    },
    serverLogMessage(message) {
      output.appendLine(message);
    },
    serverErrorMessage(message) {
      output.appendLine(message);
      api.window.showErrorMessage(message);
    },
    serverWarningMessage(message) {
      output.appendLine(message);
      api.window.showWarningMessage(message);
    },
    serverInfoMessage(message) {
      output.appendLine(message);
      api.window.showInformationMessage(message);
    },
    commandNotTrusted() {
      output.appendLine(clientMessage(api, "vscode-command-untrusted"));
      api.window.showErrorMessage(clientMessage(api, "vscode-command-untrusted"));
    },
    commandDocumentRequired() {
      return new Error(clientMessage(api, "vscode-command-document-required"));
    },
    commandDocumentUnsaved() {
      return new Error(clientMessage(api, "vscode-command-document-unsaved"));
    },
    commandUntitledDocument() {
      return new Error(clientMessage(api, "vscode-command-document-untitled"));
    },
    commandDocumentChanged() {
      return new Error(clientMessage(api, "vscode-command-document-changed"));
    },
    commandDocumentOutsideRoot() {
      return new Error(clientMessage(api, "vscode-command-document-outside-root"));
    },
    commandWorkspaceRequired() {
      return new Error(clientMessage(api, "vscode-command-workspace-required"));
    },
    cliPathInvalid() {
      return new Error(clientMessage(api, "vscode-command-cli-path-invalid"));
    },
    commandInputInvalid() {
      return new Error(clientMessage(api, "vscode-command-input-invalid"));
    },
    activeDocument() {
      return api.window?.activeTextEditor?.document;
    },
    activeEditor() {
      return api.window?.activeTextEditor;
    },
    documentIsOpen(document) {
      return api.workspace.textDocuments?.includes(document) ?? true;
    },
    chooseCompileOutputPath(defaultUri) {
      return api.window.showSaveDialog({
        defaultUri,
        title: clientMessage(api, "vscode-command-compile-output-title")
      });
    },
    chooseExtractOutputPath(defaultUri) {
      return api.window.showSaveDialog({
        defaultUri,
        title: clientMessage(api, "vscode-command-extract-output-title")
      });
    },
    chooseAssetPath() {
      return api.window.showOpenDialog({
        title: clientMessage(api, "vscode-command-asset-title"),
        filters: { [clientMessage(api, "vscode-command-asset-filter")]: ["recitec"] },
        canSelectFiles: true,
        canSelectFolders: false,
        canSelectMany: false
      });
    },
    chooseBlock() {
      return api.window.showInputBox({
        title: clientMessage(api, "vscode-command-block-title"),
        prompt: clientMessage(api, "vscode-command-block-prompt"),
        placeHolder: clientMessage(api, "vscode-command-block-placeholder")
      });
    },
    chooseRenameName(placeholder) {
      return api.window.showInputBox({
        title: clientMessage(api, "vscode-command-rename-title"),
        prompt: clientMessage(api, "vscode-command-rename-prompt"),
        placeHolder: placeholder || clientMessage(api, "vscode-command-rename-placeholder")
      });
    },
    chooseFixturePath() {
      return api.window.showOpenDialog({
        title: clientMessage(api, "vscode-command-fixture-title"),
        filters: { [clientMessage(api, "vscode-command-fixture-filter")]: ["toml"] },
        canSelectFiles: true,
        canSelectFolders: false,
        canSelectMany: false
      });
    },
    commandWatchRunning() {
      output.appendLine(clientMessage(api, "vscode-command-watch-running"));
      api.window.showErrorMessage(clientMessage(api, "vscode-command-watch-running"));
    },
    commandWatchNotRunning() {
      output.appendLine(clientMessage(api, "vscode-command-watch-not-running"));
      api.window.showErrorMessage(clientMessage(api, "vscode-command-watch-not-running"));
    },
    commandWatchStopTimeout() {
      output.appendLine(clientMessage(api, "vscode-command-watch-stop-timeout"));
      api.window.showErrorMessage(clientMessage(api, "vscode-command-watch-stop-timeout"));
    },
    commandResult(detail) {
      output.appendLine(clientMessage(api, "vscode-command-result", detail));
    },
    commandContentDiagnostics(detail) {
      output.appendLine(clientMessage(api, "vscode-command-content-diagnostics", detail));
    },
    commandFailure(detail) {
      output.appendLine(clientMessage(api, "vscode-command-failure", detail));
      api.window.showErrorMessage(clientMessage(api, "vscode-command-failure", detail));
    },
    commandProtocolFailure(detail) {
      output.appendLine(clientMessage(api, "vscode-command-protocol-failure", detail));
      api.window.showErrorMessage(clientMessage(api, "vscode-command-protocol-failure", detail));
    },
    commandWatchStatus(detail) {
      output.appendLine(clientMessage(api, "vscode-command-watch-status", detail));
    },
    dispose() {
      output.dispose();
    }
  });
}
