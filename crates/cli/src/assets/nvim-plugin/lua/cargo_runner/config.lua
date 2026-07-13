local M = {}

---@class CargoRunnerConfig
---@field binary string
---@field use_cargo_subcommand boolean
---@field keymaps boolean
---@field map_super "auto"|boolean
---@field leader_run string
---@field leader_override string
---@field notify boolean
---@field open_error_on_fail boolean
---@field save_before_run boolean
---@field prefer_terminal_for_long_running boolean
---@field no_emoji boolean
---@field quiet_cli boolean Pass --quiet / CARGO_RUNNER_QUIET (less banner noise)
---@field toast_timeout_ms integer Auto-dismiss success/info toasts
---@field show_raw_output boolean Reserved; raw cargo log never streams to the UI
---@field leader_peek string
---@field leader_jobs string
---@field leader_kill string
---@field force_terminal_for_long_running boolean Use :terminal split (default false; background + peek is non-blocking)
---@field prefer_terminal_for_long_running boolean Legacy name; only with force_terminal_for_long_running

---@type CargoRunnerConfig
M.defaults = {
  binary = "cargo-runner",
  use_cargo_subcommand = false,
  keymaps = true,
  map_super = "auto",
  leader_run = "<leader>r",
  leader_override = "<leader>R",
  leader_peek = "<leader>ro",
  leader_jobs = "<leader>rj",
  leader_kill = "<leader>rk",
  notify = true,
  open_error_on_fail = true,
  save_before_run = false,
  -- Default: keep servers in background jobs (peek with <leader>ro). Never freezes UI.
  prefer_terminal_for_long_running = false,
  force_terminal_for_long_running = false,
  no_emoji = false,
  quiet_cli = true,
  toast_timeout_ms = 2500, -- success/info notice TTL in unified panel
  show_raw_output = false,
  -- Panel geometry is fixed in ui/panel.lua (width 50, max 3 jobs / 3 notices)
}

---@type CargoRunnerConfig
M.options = vim.deepcopy(M.defaults)

---Merge user opts and global vim.g overrides.
---@param opts CargoRunnerConfig|nil
function M.setup(opts)
  opts = opts or {}
  M.options = vim.tbl_deep_extend("force", vim.deepcopy(M.defaults), opts)

  if vim.g.cargo_runner_binary then
    M.options.binary = vim.g.cargo_runner_binary
  end
  if vim.g.cargo_runner_map_super ~= nil then
    M.options.map_super = vim.g.cargo_runner_map_super
  end
  if vim.g.cargo_runner_no_emoji ~= nil then
    M.options.no_emoji = vim.g.cargo_runner_no_emoji
  end
end

function M.get()
  return M.options
end

---Whether Super/Cmd keymaps should be registered.
function M.should_map_super()
  local ms = M.options.map_super
  if ms == true or ms == 1 then
    return true
  end
  if ms == false or ms == 0 then
    return false
  end
  -- auto
  if vim.g.neovide then
    return true
  end
  if vim.fn.has("gui_running") == 1 then
    return true
  end
  return false
end

return M
