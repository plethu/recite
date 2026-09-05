import { memberMethod, propertyName } from "./ui-boundary-ast.mjs";

export const COMMAND_UI_METHOD_CONTRACTS = Object.freeze({
  commandNotTrusted: {
    kind: "visible-projection", id: "vscode-command-untrusted", host: "showErrorMessage"
  },
  commandDocumentRequired: { kind: "error", id: "vscode-command-document-required" },
  commandDocumentUnsaved: { kind: "error", id: "vscode-command-document-unsaved" },
  commandUntitledDocument: { kind: "error", id: "vscode-command-document-untitled" },
  commandDocumentChanged: { kind: "error", id: "vscode-command-document-changed" },
  commandDocumentOutsideRoot: { kind: "error", id: "vscode-command-document-outside-root" },
  commandWorkspaceRequired: { kind: "error", id: "vscode-command-workspace-required" },
  cliPathInvalid: { kind: "error", id: "vscode-command-cli-path-invalid" },
  commandInputInvalid: { kind: "error", id: "vscode-command-input-invalid" },
  activeDocument: { kind: "host-value" },
  documentIsOpen: { kind: "host-membership", argument: "document" },
  chooseCompileOutputPath: {
    kind: "host-capability-projection", host: "showSaveDialog", argument: "defaultUri",
    id: "vscode-command-compile-output-title"
  },
  chooseExtractOutputPath: {
    kind: "host-capability-projection", host: "showSaveDialog", argument: "defaultUri",
    id: "vscode-command-extract-output-title"
  },
  chooseAssetPath: {
    kind: "host-capability-projection", host: "showOpenDialog", id: "vscode-command-asset-title",
    filterId: "vscode-command-asset-filter", extension: "recitec"
  },
  chooseBlock: {
    kind: "host-capability-projection", host: "showInputBox", id: "vscode-command-block-title",
    promptId: "vscode-command-block-prompt", placeholderId: "vscode-command-block-placeholder"
  },
  chooseFixturePath: {
    kind: "host-capability-projection", host: "showOpenDialog", id: "vscode-command-fixture-title",
    filterId: "vscode-command-fixture-filter", extension: "toml"
  },
  commandWatchRunning: {
    kind: "visible-projection", id: "vscode-command-watch-running", host: "showErrorMessage"
  },
  commandWatchNotRunning: {
    kind: "visible-projection", id: "vscode-command-watch-not-running", host: "showErrorMessage"
  },
  commandWatchStopTimeout: {
    kind: "visible-projection", id: "vscode-command-watch-stop-timeout", host: "showErrorMessage"
  },
  commandResult: { kind: "projection", id: "vscode-command-result", argument: "detail" },
  commandContentDiagnostics: {
    kind: "projection", id: "vscode-command-content-diagnostics", argument: "detail"
  },
  commandFailure: {
    kind: "visible-projection", id: "vscode-command-failure", argument: "detail", host: "showErrorMessage"
  },
  commandProtocolFailure: {
    kind: "visible-projection", id: "vscode-command-protocol-failure", argument: "detail", host: "showErrorMessage"
  },
  commandWatchStatus: { kind: "projection", id: "vscode-command-watch-status", argument: "detail" }
});

export function validateCommandMethod(property, contract, file) {
  if (contract.kind === "host-capability-projection") {
    assert(property.value.params.length === (contract.argument ? 1 : 0),
      `UI method ${propertyName(property)} has the wrong argument count (${file})`);
    const statement = oneStatement(property.value, file, propertyName(property));
    const call = statement.type === "ReturnStatement" ? statement.argument : undefined;
    assert(isProjectedWindowHostCall(call, contract.host, contract.id, contract.argument),
      `UI method ${propertyName(property)} must return the localized host capability (${file})`);
    const options = call.arguments[0];
    assert(isOptionMessage(options, "title", contract.id),
      `UI method ${propertyName(property)} must localize its title (${file})`);
    if (contract.filterId) {
      assert(isLocalizedFilter(options, contract.filterId, contract.extension),
        `UI method ${propertyName(property)} must localize its file filter (${file})`);
    }
    if (contract.promptId) {
      assert(isOptionMessage(options, "prompt", contract.promptId),
        `UI method ${propertyName(property)} must localize its prompt (${file})`);
    }
    if (contract.placeholderId) {
      assert(isOptionMessage(options, "placeHolder", contract.placeholderId),
        `UI method ${propertyName(property)} must localize its placeholder (${file})`);
    }
    return true;
  }
  if (contract.kind === "host-value") {
    assert(property.value.params.length === 0,
      `UI method ${propertyName(property)} has the wrong argument count (${file})`);
    const statement = oneStatement(property.value, file, propertyName(property));
    const value = statement.type === "ReturnStatement" ? statement.argument : undefined;
    assert(isActiveDocumentValue(value),
      `UI method ${propertyName(property)} must return the active document (${file})`);
    return true;
  }
  if (contract.kind === "host-membership") {
    assert(property.value.params.length === 1 && property.value.params[0].type === "Identifier" &&
      property.value.params[0].name === contract.argument,
    `UI method ${propertyName(property)} has the wrong argument (${file})`);
    const statement = oneStatement(property.value, file, propertyName(property));
    const value = statement.type === "ReturnStatement" ? statement.argument : undefined;
    assert(isDocumentOpenValue(value, contract.argument),
      `UI method ${propertyName(property)} must return canonical document membership (${file})`);
    return true;
  }
  return false;
}

function isWindowHostCall(node, method) {
  return node?.type === "CallExpression" && node.arguments.length === 1 &&
    node.callee.type === "MemberExpression" && !node.callee.computed && !node.callee.optional &&
    node.callee.object.type === "MemberExpression" && !node.callee.object.computed &&
    !node.callee.object.optional && node.callee.object.object.type === "Identifier" &&
    node.callee.object.object.name === "api" && memberMethod(node.callee.object) === "window" &&
    memberMethod(node.callee) === method;
}

function isProjectedWindowHostCall(node, method, id, argument) {
  if (!isWindowHostCall(node, method)) return false;
  const options = node.arguments[0];
  if (options.type !== "ObjectExpression") return false;
  const title = options.properties.find((property) => propertyName(property) === "title");
  if (!isCanonicalMessage(title?.value, id)) return false;
  if (!argument) return true;
  const value = options.properties.find((property) => propertyName(property) === argument);
  return value?.value.type === "Identifier" && value.value.name === argument;
}

function isOptionMessage(options, name, id) {
  const property = options.properties.find((candidate) => propertyName(candidate) === name);
  return isCanonicalMessage(property?.value, id);
}

function isLocalizedFilter(options, id, extension) {
  const property = options.properties.find((candidate) => propertyName(candidate) === "filters");
  const filters = property?.value;
  if (filters?.type !== "ObjectExpression" || filters.properties.length !== 1) return false;
  const filter = filters.properties[0];
  return filter.computed && isCanonicalMessage(filter.key, id) &&
    filter.value.type === "ArrayExpression" && filter.value.elements.length === 1 &&
    filter.value.elements[0].type === "Literal" && filter.value.elements[0].value === extension;
}

function isCanonicalMessage(node, id) {
  return node?.type === "CallExpression" && node.callee.type === "Identifier" &&
    node.callee.name === "clientMessage" && node.arguments.length === 2 &&
    node.arguments[0].type === "Identifier" && node.arguments[0].name === "api" &&
    node.arguments[1].type === "Literal" && node.arguments[1].value === id;
}

function isActiveDocumentValue(node) {
  return node?.type === "ChainExpression" && node.expression.type === "MemberExpression" &&
    node.expression.optional && !node.expression.computed &&
    node.expression.property.type === "Identifier" && node.expression.property.name === "document" &&
    node.expression.object.type === "MemberExpression" && !node.expression.object.computed &&
    node.expression.object.property.type === "Identifier" &&
    node.expression.object.property.name === "activeTextEditor" &&
    node.expression.object.object.type === "MemberExpression" &&
    !node.expression.object.object.computed &&
    node.expression.object.object.object.type === "Identifier" &&
    node.expression.object.object.object.name === "api" &&
    memberMethod(node.expression.object.object) === "window";
}

function isDocumentOpenValue(node, parameter) {
  if (node?.type !== "LogicalExpression" || node.operator !== "??" ||
      node.right?.type !== "Literal" || node.right.value !== true) return false;
  const chain = node.left?.type === "ChainExpression" ? node.left.expression : node.left;
  if (chain?.type !== "CallExpression" || chain.arguments.length !== 1 ||
      chain.arguments[0].type !== "Identifier" || chain.arguments[0].name !== parameter) return false;
  const includes = chain.callee;
  const documents = includes?.type === "MemberExpression" && !includes.computed &&
    includes.property.type === "Identifier" && includes.property.name === "includes"
    ? includes.object : undefined;
  return documents?.type === "MemberExpression" && !documents.computed &&
    documents.property.type === "Identifier" && documents.property.name === "textDocuments" &&
    documents.object?.type === "MemberExpression" && !documents.object.computed &&
    documents.object.property.type === "Identifier" && documents.object.property.name === "workspace" &&
    documents.object.object?.type === "Identifier" && documents.object.object.name === "api";
}

function oneStatement(method, file, name) {
  assert(method.body.type === "BlockStatement" && method.body.body.length === 1,
    `UI adapter method ${name} must have one exact statement (${file})`);
  return method.body.body[0];
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}
