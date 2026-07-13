local config = require("cargo_runner.config")

local M = {}

---Ensure ~/.cargo/bin (and similar) are on PATH for child jobs.
---GUI / some Terminal sessions omit cargo's bin dir even when the shell has it.
function M.ensure_tool_path()
  local home = vim.fn.expand("~")
  local extras = {
    home .. "/.cargo/bin",
    "/opt/homebrew/bin",
    "/usr/local/bin",
  }
  local path = vim.env.PATH or ""
  local parts = vim.split(path, ":", { plain = true })
  local seen = {}
  for _, p in ipairs(parts) do
    seen[p] = true
  end
  local prepend = {}
  for _, dir in ipairs(extras) do
    if vim.fn.isdirectory(dir) == 1 and not seen[dir] then
      table.insert(prepend, dir)
      seen[dir] = true
    end
  end
  if #prepend > 0 then
    vim.env.PATH = table.concat(prepend, ":") .. ":" .. path
  end
end

---@return string[]
local function candidate_binaries(name)
  local home = vim.fn.expand("~")
  return {
    name,
    home .. "/.cargo/bin/" .. name,
    "/opt/homebrew/bin/" .. name,
    "/usr/local/bin/" .. name,
  }
end

---@param name string
---@return string|nil
local function find_executable(name)
  -- Absolute / configured path first
  if name:find("/") then
    if vim.fn.executable(name) == 1 then
      return name
    end
    return nil
  end
  local exepath = vim.fn.exepath(name)
  if exepath ~= "" then
    return exepath
  end
  for _, cand in ipairs(candidate_binaries(name)) do
    if vim.fn.executable(cand) == 1 then
      return cand
    end
  end
  return nil
end

---@return string|nil path, string|nil err
function M.resolve_binary()
  M.ensure_tool_path()
  local cfg = config.get()
  if cfg.use_cargo_subcommand then
    local cargo = find_executable("cargo")
    if not cargo then
      return nil, "cargo not found on PATH (needed for `cargo runner`)"
    end
    return cargo, nil
  end

  local bin = cfg.binary or "cargo-runner"
  local found = find_executable(bin)
  if found then
    return found, nil
  end

  -- Fall back to `cargo runner …`
  local cargo = find_executable("cargo")
  if cargo then
    return cargo, nil
  end

  local hint = vim.fn.expand("~/.cargo/bin/cargo-runner")
  return nil,
    "cargo-runner not found. Install: cargo binstall cargo-runner-cli\n"
      .. "Or set: require('cargo_runner').setup({ binary = '"
      .. hint
      .. "' })"
end

---Build argv for cargo-runner (handles cargo-runner vs cargo runner).
---@param args string[] subcommand args e.g. {"run", "src/lib.rs:1"}
---@return string[] argv
function M.build_argv(args)
  local cfg = config.get()
  local bin, err = M.resolve_binary()
  if not bin then
    error(err)
  end

  if cfg.use_cargo_subcommand or vim.fn.fnamemodify(bin, ":t") == "cargo" then
    local out = { bin, "runner" }
    vim.list_extend(out, args)
    return out
  end

  local out = { bin }
  vim.list_extend(out, args)
  return out
end

---@class CliResult
---@field code integer
---@field stdout string
---@field stderr string

---Synchronous command — ONLY for short override saves. Prefer run_async.
---@param args string[]
---@param opts { cwd?: string, timeout_ms?: integer }|nil
---@return CliResult
function M.run_sync(args, opts)
  opts = opts or {}
  local argv = M.build_argv(args)
  local cwd = opts.cwd or vim.fn.getcwd()
  local timeout = opts.timeout_ms or 60000

  local result = vim.system(argv, {
    cwd = cwd,
    text = true,
    timeout = timeout,
    env = vim.fn.environ(),
  }):wait()

  return {
    code = result.code or -1,
    stdout = result.stdout or "",
    stderr = result.stderr or "",
  }
end

---Non-blocking CLI invoke. Never freezes the editor.
---@param args string[]
---@param opts { cwd?: string, env?: table }|nil
---@param on_done fun(result: CliResult)
---@return userdata|table|nil handle
function M.run_async(args, opts, on_done)
  opts = opts or {}
  local argv = M.build_argv(args)
  local cwd = opts.cwd or vim.fn.getcwd()
  local env = opts.env or vim.fn.environ()

  return vim.system(argv, {
    cwd = cwd,
    text = true,
    env = env,
  }, function(result)
    vim.schedule(function()
      on_done({
        code = result.code or -1,
        stdout = result.stdout or "",
        stderr = result.stderr or "",
      })
    end)
  end)
end

---Async dry-run JSON. Does not block the UI.
---@param file_arg string
---@param cwd string|nil
---@param on_done fun(ok: boolean, dry_or_err: any)
function M.dry_run_async(file_arg, cwd, on_done)
  M.run_async({ "run", file_arg, "--dry-run", "--json" }, { cwd = cwd }, function(result)
    local ok, data = pcall(function()
      return M.parse_json(result)
    end)
    if ok then
      on_done(true, data)
    else
      on_done(false, data)
    end
  end)
end

---Parse JSON stdout; on structured error raise message.
---@param result CliResult
---@return any
function M.parse_json(result)
  local text = vim.trim(result.stdout or "")
  if text == "" then
    if result.code ~= 0 then
      error(vim.trim(result.stderr) ~= "" and result.stderr or ("exit " .. tostring(result.code)))
    end
    error("empty JSON response from cargo-runner")
  end

  local ok, data = pcall(vim.json.decode, text)
  if not ok then
    if result.code ~= 0 then
      error(vim.trim(result.stderr) ~= "" and result.stderr or text)
    end
    error("invalid JSON from cargo-runner: " .. tostring(data))
  end

  if type(data) == "table" and data.error then
    error(data.message or "cargo-runner error")
  end

  if result.code ~= 0 then
    error(vim.trim(result.stderr) ~= "" and result.stderr or text)
  end

  return data
end

---@param file_arg string
---@param cwd string|nil
---@return table dry_run output
function M.dry_run(file_arg, cwd)
  local result = M.run_sync({ "run", file_arg, "--dry-run", "--json" }, {
    cwd = cwd,
    timeout_ms = 60000,
  })
  return M.parse_json(result)
end

---@param file_arg string
---@param tokens string[]
---@param cwd string|nil
function M.set_override(file_arg, tokens, cwd)
  local args = { "override", file_arg, "--" }
  vim.list_extend(args, tokens)
  local result = M.run_sync(args, { cwd = cwd, timeout_ms = 30000 })
  if result.code ~= 0 then
    local msg = vim.trim(result.stderr)
    if msg == "" then
      msg = vim.trim(result.stdout)
    end
    if msg == "" then
      msg = "override failed (exit " .. tostring(result.code) .. ")"
    end
    -- Prefer structured error if present
    local ok, data = pcall(vim.json.decode, vim.trim(result.stdout))
    if ok and type(data) == "table" and data.error and data.message then
      msg = data.message
    end
    error(msg)
  end
end

---Walk up from path looking for Cargo.toml / MODULE.bazel / .git
---@param start string|nil
---@return string
function M.project_cwd(start)
  local dir = start or vim.fn.expand("%:p:h")
  if dir == "" then
    return vim.fn.getcwd()
  end
  local markers = { "Cargo.toml", "MODULE.bazel", "WORKSPACE", "WORKSPACE.bazel", ".git" }
  local cur = dir
  for _ = 1, 40 do
    for _, m in ipairs(markers) do
      if vim.fn.filereadable(cur .. "/" .. m) == 1 or vim.fn.isdirectory(cur .. "/" .. m) == 1 then
        return cur
      end
    end
    local parent = vim.fn.fnamemodify(cur, ":h")
    if parent == cur then
      break
    end
    cur = parent
  end
  return vim.fn.getcwd()
end

---Current buffer as file:line (1-based).
---@return string|nil file_arg, string|nil err
function M.cursor_file_arg()
  local path = vim.fn.expand("%:p")
  if path == "" then
    return nil, "no file in current buffer"
  end
  local ft = vim.bo.filetype
  if ft ~= "rust" and not path:match("%.rs$") then
    return nil, "open a Rust file first"
  end
  local line = vim.fn.line(".")
  return path .. ":" .. tostring(line), nil
end

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

return M
