local vocabulary = require("recite_error_vocabulary")
local protocol = require("recite.command_protocol")

local function assert_true(value, message)
  assert(value, "Neovim structured error check: " .. message)
end

for code in pairs(vocabulary.errorCodes) do
  assert_true(
    protocol.valid_error({ category = "input", code = code, operation = "validate" }),
    "authoritative error code was rejected: " .. code
  )
end
assert_true(not protocol.valid_error({
  category = "input",
  code = "structured_stderr",
  operation = "validate",
}), "transport stderr error leaked into the authoritative error-code set")
for operation in pairs(vocabulary.operations) do
  assert_true(
    protocol.valid_error({ category = "input", code = "missing_path", operation = operation }),
    "authoritative operation was rejected: " .. operation
  )
end
for _, operation in ipairs({ "control", "dispatch", "export_schema", "render" }) do
  assert_true(
    not protocol.valid_error({ category = "input", code = "missing_path", operation = operation }),
    "non-wire operation was accepted: " .. operation
  )
end
assert_true(not protocol.valid_error({
  category = "input",
  code = "missing_path",
  operation = "validate",
  path = vim.NIL,
}), "explicit null path was accepted")
assert_true(not protocol.valid_error({
  category = "input",
  code = "missing_path",
  operation = "validate",
  related_path = vim.NIL,
}), "explicit null related_path was accepted")
assert_true(not protocol.valid_error({
  category = "input",
  code = "missing_path",
  operation = "validate",
  details = vim.NIL,
}), "explicit null details was accepted")
