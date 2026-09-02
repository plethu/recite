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
    dispose() {
      output.dispose();
    }
  });
}
