---Classify a resolved shell command into a job kind for icons / UI.

local M = {}

---@class JobKindInfo
---@field kind string
---@field icon string
---@field title string
---@field long_running boolean

local KINDS = {
  server = { icon = "🚀", title = "server", long_running = true },
  watch = { icon = "👀", title = "watch", long_running = true },
  test = { icon = "🧪", title = "test", long_running = false },
  bench = { icon = "⏱", title = "bench", long_running = false },
  run = { icon = "▶", title = "run", long_running = false },
  build = { icon = "📦", title = "build", long_running = false },
  doc = { icon = "📄", title = "doc", long_running = false },
  unknown = { icon = "⚙", title = "job", long_running = false },
}

function M.info(kind)
  return KINDS[kind] or KINDS.unknown
end

function M.icon(kind)
  return M.info(kind).icon
end

local function has(s, needle)
  return s:find(needle, 1, true) ~= nil
end

---Infer kind from dry-run shell / strategy.
---@param shell string|nil
---@param strategy string|nil
---@return string kind
function M.classify(shell, strategy)
  local s = (shell or ""):lower()
  local st = (strategy or ""):lower()

  if has(s, "watch") or has(s, "cargo watch") or has(s, "leptos watch") then
    return "watch"
  end

  if has(s, "serve")
    or has(s, "tauri dev")
    or has(s, "dx serve")
    or has(s, "trunk serve")
    or has(s, "spin up")
    or has(s, "spin build --up")
    or s:find("%f[%w]dev%f[%W]")
  then
    return "server"
  end

  if has(s, "bench") or has(s, "criterion") then
    return "bench"
  end

  if has(s, "nextest") or has(s, "test") or has(st, "test") then
    return "test"
  end

  if has(s, "doc") or has(s, "rustdoc") then
    return "doc"
  end

  if has(s, "build") and not has(s, "cargo run") then
    return "build"
  end

  -- plain cargo run / bazel run → treat as long-lived app binary
  if has(s, "cargo run") or has(s, "bazel run") then
    return "server"
  end

  return "run"
end

function M.plain_icons()
  return {
    server = "[S]",
    watch = "[W]",
    test = "[T]",
    bench = "[B]",
    run = "[R]",
    build = "[#]",
    doc = "[D]",
    unknown = "[?]",
  }
end

function M.display_icon(kind, no_emoji)
  if no_emoji then
    return M.plain_icons()[kind] or "[?]"
  end
  return M.icon(kind)
end

return M
