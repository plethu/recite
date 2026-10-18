import { isCallMethod, staticModuleSpecifier, walkWithParents } from "./ui-boundary-ast.mjs";
import {
  isDynamicReflectCall,
  isReflectCall,
  isWindowPropertyLookup,
} from "./ui-boundary-capabilities.mjs";
import {
  collectUiNames,
  isValidUiCall,
  isAllowedUiCallMember,
  isCapabilityMember,
  isCapabilityPattern,
  isUiInjectionAssignment,
  isVisibleSinkMember,
  isDynamicKnownAccess,
  isKnownUiMember,
  isCreateUserInterfaceCall,
  isCapabilityExpression,
  isMessageExpression,
  isMessageMember,
  isMessageCapabilityMember,
  isMessageIdentifier,
  isMessageCallName,
  isRequireOfProtectedModule,
  isDirectPayload,
  staticMethod,
  isProtectedModule,
  MESSAGE_MODULE,
  ADAPTER_MODULE,
  MESSAGE_WRAPPER,
  ADAPTER_FILE,
  VISIBLE_METHODS,
} from "./ui-boundary-receivers.mjs";

export function validateOutside(ast, file, contracts) {
  const uiNames = collectUiNames(ast);
  walkWithParents(ast, (node, parent) => {
    const moduleSpecifier = staticModuleSpecifier(node);
    if (node.type === "ImportDeclaration" && isProtectedModule(moduleSpecifier, MESSAGE_MODULE)) {
      fail(`only ${ADAPTER_FILE} may import ${MESSAGE_WRAPPER} (${file})`);
    }
    if (
      (node.type === "ExportNamedDeclaration" || node.type === "ExportAllDeclaration") &&
      isProtectedModule(moduleSpecifier, MESSAGE_MODULE)
    ) {
      fail(`only ${ADAPTER_FILE} may re-export ${MESSAGE_WRAPPER} (${file})`);
    }
    if (
      node.type === "ImportExpression" &&
      isProtectedModule(moduleSpecifier, MESSAGE_MODULE, ADAPTER_MODULE)
    ) {
      fail(`dynamic import of the UI boundary is outside ${ADAPTER_FILE} (${file})`);
    }

    if (node.type === "CallExpression") {
      if (isMessageCallName(node.callee)) {
        fail(`only ${ADAPTER_FILE} may call ${MESSAGE_WRAPPER} (${file})`);
      }
      if (isRequireOfProtectedModule(node)) {
        fail(`only ${ADAPTER_FILE} may load UI messages (${file})`);
      }
      if (
        (isReflectCall(node) || isDynamicReflectCall(node)) &&
        node.arguments.some(
          (argument) => isCapabilityExpression(argument, uiNames) || isWindowPropertyLookup(node),
        )
      ) {
        fail(`Reflect capability recovery is outside ${ADAPTER_FILE} (${file})`);
      }
      if (isCreateUserInterfaceCall(node)) {
        assert(
          node.arguments.length === 1 && isDirectPayload(node.arguments[0]),
          `UI factory calls must pass one direct host value (${file})`,
        );
      }
      if (!isValidUiCall(node, contracts, uiNames)) {
        const method = staticMethod(node.callee);
        if (method && VISIBLE_METHODS.has(method) && isVisibleSinkMember(node.callee, uiNames)) {
          fail(`visible host sink ${method} is outside ${ADAPTER_FILE} (${file})`);
        }
        if (isKnownUiMember(node.callee, uiNames)) {
          fail(`UI service call has an invalid shape outside ${ADAPTER_FILE} (${file})`);
        }
        if (isDynamicKnownAccess(node.callee, uiNames)) {
          fail(`dynamic UI/window access is outside ${ADAPTER_FILE} (${file})`);
        }
        if (
          (isCallMethod(node, "call") ||
            isCallMethod(node, "apply") ||
            isCallMethod(node, "bind")) &&
          (isCapabilityMember(node.callee.object, contracts, uiNames) ||
            isVisibleSinkMember(node.callee.object, uiNames) ||
            isMessageExpression(node.callee.object))
        ) {
          fail(`visible host sink aliases are outside ${ADAPTER_FILE} (${file})`);
        }
      }
    }

    if (node.type === "VariableDeclarator" || node.type === "AssignmentExpression") {
      const right = node.type === "VariableDeclarator" ? node.init : node.right;
      const left = node.type === "VariableDeclarator" ? node.id : node.left;
      if (
        isMessageIdentifier(left) ||
        isMessageExpression(right) ||
        (!isCreateUserInterfaceCall(right) &&
          isCapabilityExpression(right, uiNames) &&
          (node.type === "VariableDeclarator" || !isUiInjectionAssignment(left, right))) ||
        isCapabilityPattern(left, right, uiNames) ||
        isCapabilityMember(right, contracts, uiNames) ||
        isMessageMember(right) ||
        isMessageCapabilityMember(right)
      ) {
        fail(`UI capability acquisition is outside ${ADAPTER_FILE} (${file})`);
      }
      if (isCapabilityMember(left, contracts, uiNames)) {
        fail(`UI capability reassignment is outside ${ADAPTER_FILE} (${file})`);
      }
    }

    if (node.type === "MemberExpression") {
      if (node.computed && isDynamicKnownAccess(node, uiNames)) {
        fail(`dynamic UI/window access is outside ${ADAPTER_FILE} (${file})`);
      }
      if (isVisibleSinkMember(node, uiNames)) {
        fail(`visible host sink acquisition is outside ${ADAPTER_FILE} (${file})`);
      }
      if (
        isCapabilityMember(node, contracts, uiNames) &&
        !isAllowedUiCallMember(node, parent, contracts, uiNames)
      ) {
        fail(`UI capability acquisition is outside ${ADAPTER_FILE} (${file})`);
      }
    }
  });
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function fail(message) {
  throw new Error(message);
}
