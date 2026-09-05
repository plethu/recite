-- Lossless JSON integer lexer for Neovim's Lua number runtime.
local M = {}
-- Wide JSON integers are opaque tokens.  A public `{ raw = ... }` table or a
-- copied table must not be able to impersonate one of these values.
local INTEGER_VALUES = setmetatable({}, { __mode = "k" })

function M.integer(raw)
  local value = {}
  INTEGER_VALUES[value] = raw
  return value
end

function M.is_integer(value)
  return type(value) == "table" and INTEGER_VALUES[value] ~= nil
    or type(value) == "number" and value == math.floor(value)
    and value >= -9007199254740991 and value <= 9007199254740991
end

function M.integer_raw(value)
  if type(value) == "table" and INTEGER_VALUES[value] ~= nil then return INTEGER_VALUES[value] end
  if type(value) ~= "number" or value ~= math.floor(value) or value ~= value
    or value < -9007199254740991 or value > 9007199254740991 then return nil end
  return string.format("%.0f", value)
end

local function canonical(raw)
  if type(raw) ~= "string" or not raw:match("^-?%d+$") then return nil end
  local negative = raw:sub(1, 1) == "-"
  local digits = negative and raw:sub(2) or raw
  if #digits > 1 and digits:sub(1, 1) == "0" then return nil end
  digits = digits:gsub("^0+", "")
  if digits == "" then digits = "0" end
  if digits == "0" then negative = false end
  return (negative and "-" or "") .. digits
end

local function compare_unsigned(left, right)
  left, right = left:gsub("^0+", ""), right:gsub("^0+", "")
  if #left ~= #right then return #left < #right and -1 or 1 end
  if left == right then return 0 end
  return left < right and -1 or 1
end

function M.compare(left, right)
  left = canonical(M.integer_raw(left) or left)
  right = canonical(M.integer_raw(right) or right)
  if not left or not right then return nil end
  local negative_left, negative_right = left:sub(1, 1) == "-", right:sub(1, 1) == "-"
  if negative_left ~= negative_right then return negative_left and -1 or 1 end
  local left_digits = negative_left and left:sub(2) or left
  local right_digits = negative_right and right:sub(2) or right
  local result = compare_unsigned(left_digits, right_digits)
  return negative_left and -result or result
end

function M.in_range(value, minimum, maximum)
  local raw = canonical(M.integer_raw(value) or value)
  return raw ~= nil and M.compare(raw, minimum) >= 0 and M.compare(raw, maximum) <= 0
end

function M.increment(value)
  local raw = canonical(M.integer_raw(value) or value)
  if not raw then return nil end
  local negative = raw:sub(1, 1) == "-"
  if negative then return nil end
  local digits = raw
  local index = #digits
  local carry = 1
  local output = {}
  while index > 0 do
    local digit = digits:byte(index) - 48 + carry
    if digit >= 10 then digit, carry = digit - 10, 1 else carry = 0 end
    output[index] = string.char(48 + digit)
    index = index - 1
  end
  if carry == 1 then table.insert(output, 1, "1") end
  return table.concat(output)
end

local function number_start(line, index)
  local length, start = #line, index
  if line:sub(index, index) == "-" then index = index + 1 end
  local first = line:sub(index, index)
  if first == "" or not first:match("%d") then return nil end
  if first == "0" then
    index = index + 1
    if line:sub(index, index):match("%d") then return false end
  else
    while index <= length and line:sub(index, index):match("%d") do index = index + 1 end
  end
  local integer_end, fraction = index - 1, false
  if line:sub(index, index) == "." then
    fraction, index = true, index + 1
    local digits = index
    while index <= length and line:sub(index, index):match("%d") do index = index + 1 end
    if index == digits then return nil end
  end
  local exponent = false
  if line:sub(index, index):match("[eE]") then
    exponent, index = true, index + 1
    if line:sub(index, index):match("[+-]") then index = index + 1 end
    local digits = index
    while index <= length and line:sub(index, index):match("%d") do index = index + 1 end
    if index == digits then return nil end
  end
  return line:sub(start, index - 1), line:sub(start, integer_end), index, not fraction and not exponent
end

local function quoted_end(line, index)
  index = index + 1
  local escaped = false
  while index <= #line do
    local character = line:sub(index, index)
    if escaped then escaped = false
    elseif character == "\\" then escaped = true
    elseif character == '"' then return index + 1 end
    index = index + 1
  end
  return index
end

local function reject_duplicate_keys(line)
  local function whitespace(index)
    while line:sub(index, index):match("%s") do index = index + 1 end
    return index
  end

  local function value(index)
    index = whitespace(index)
    local character = line:sub(index, index)
    if character == '"' then return quoted_end(line, index) end
    if character == "{" then
      index = whitespace(index + 1)
      local seen = {}
      if line:sub(index, index) == "}" then return index + 1 end
      while true do
        if line:sub(index, index) ~= '"' then return index end
        local finish = quoted_end(line, index)
        local ok, key = pcall(vim.json.decode, line:sub(index, finish - 1))
        if not ok or type(key) ~= "string" then return index end
        if seen[key] then error("duplicate JSON object key: " .. key) end
        seen[key] = true
        index = whitespace(finish)
        if line:sub(index, index) ~= ":" then return index end
        index = value(index + 1)
        index = whitespace(index)
        character = line:sub(index, index)
        if character == "}" then return index + 1 end
        if character ~= "," then return index end
        index = whitespace(index + 1)
      end
    end
    if character == "[" then
      index = whitespace(index + 1)
      if line:sub(index, index) == "]" then return index + 1 end
      while true do
        index = value(index)
        index = whitespace(index)
        character = line:sub(index, index)
        if character == "]" then return index + 1 end
        if character ~= "," then return index end
        index = whitespace(index + 1)
      end
    end
    if character == "" then return index end
    while index <= #line and not line:sub(index, index):match("[%s,%]}]") do index = index + 1 end
    return index
  end

  local finish = value(1)
  if whitespace(finish) <= #line then error("invalid JSON trailing data") end
end

function M.parse(line)
  -- Reject duplicates before the generic decoder is allowed to apply
  -- last-write-wins semantics, then decode once to inventory every original
  -- string and object key before choosing deterministic placeholders.
  reject_duplicate_keys(line)
  local original = vim.json.decode(line)
  local occupied = {}
  local function inventory(value)
    if type(value) == "string" then
      occupied[value] = true
    elseif type(value) == "table" then
      for key, child in pairs(value) do
        if type(key) == "string" then occupied[key] = true end
        inventory(child)
      end
    end
  end
  inventory(original)
  local marker = "__recite_lossless_integer__"
  local generated = {}
  local generated_count = 0
  local output, index = {}, 1
  while index <= #line do
    local character = line:sub(index, index)
    if character == '"' then
      local finish = quoted_end(line, index)
      output[#output + 1], index = line:sub(index, finish - 1), finish
    elseif character == "-" or character:match("%d") then
      local token, integer_token, finish, is_integer = number_start(line, index)
      if token == false then error("invalid JSON number") end
      if token then
        if is_integer and not M.in_range(integer_token, "-9007199254740991", "9007199254740991") then
          repeat
            generated_count = generated_count + 1
          until not occupied[marker .. generated_count]
          local placeholder = marker .. generated_count
          occupied[placeholder] = true
          generated[placeholder] = canonical(integer_token)
          output[#output + 1] = vim.json.encode(placeholder)
        else output[#output + 1] = token end
        index = finish
      else output[#output + 1], index = character, index + 1 end
    else output[#output + 1], index = character, index + 1 end
  end
  local decoded = vim.json.decode(table.concat(output))
  local function restore(value)
    if type(value) == "string" and generated[value] ~= nil then
      return M.integer(generated[value])
    elseif type(value) == "table" then
      for key, child in pairs(value) do value[key] = restore(child) end
    end
    return value
  end
  return restore(decoded)
end

return M
