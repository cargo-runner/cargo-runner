---Dependency-free helpers, extracted so they can be unit-tested without nvim.
---
---Nothing here may reference `vim.*` — that is the whole point: `busted` can
---load this module directly, while the rest of the plugin cannot.
local M = {}

local LONG_RUNNING_PATTERNS = {
  "serve",
  "watch",
  " dev",
  "dev ",
  "dx serve",
  "leptos watch",
  "tauri dev",
  "trunk serve",
}

---@param shell string|nil
function M.is_long_running(shell)
  if not shell or shell == "" then
    return false
  end
  local lower = shell:lower()
  for _, p in ipairs(LONG_RUNNING_PATTERNS) do
    if lower:find(p, 1, true) then
      return true
    end
  end
  return false
end

---Simple shell-ish tokenize (handles quotes loosely).
---@param s string
---@return string[]
function M.tokenize(s)
  local tokens = {}
  local i = 1
  local n = #s
  while i <= n do
    while i <= n and s:sub(i, i):match("%s") do
      i = i + 1
    end
    if i > n then
      break
    end
    local c = s:sub(i, i)
    if c == '"' or c == "'" then
      local q = c
      i = i + 1
      local start = i
      while i <= n and s:sub(i, i) ~= q do
        i = i + 1
      end
      table.insert(tokens, s:sub(start, i - 1))
      if i <= n then
        i = i + 1
      end
    else
      local start = i
      while i <= n and not s:sub(i, i):match("%s") do
        i = i + 1
      end
      table.insert(tokens, s:sub(start, i - 1))
    end
  end
  return tokens
end

return M
