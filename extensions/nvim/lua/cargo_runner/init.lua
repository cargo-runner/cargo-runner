local config = require("cargo_runner.config")
local run = require("cargo_runner.run")
local override = require("cargo_runner.override")
local jobs = require("cargo_runner.jobs")

local M = {}

local function register_commands()
  vim.api.nvim_create_user_command("CargoRunnerRun", function()
    run.at_cursor()
  end, { desc = "Cargo Runner: run at cursor (non-blocking)" })

  vim.api.nvim_create_user_command("CargoRunnerOverride", function()
    override.at_cursor()
  end, { desc = "Cargo Runner: override at cursor" })

  vim.api.nvim_create_user_command("CargoRunnerDryRun", function()
    run.dry_run_at_cursor()
  end, { desc = "Cargo Runner: dry-run at cursor (async)" })

  vim.api.nvim_create_user_command("CargoRunnerShowOutput", function()
    jobs.show_last_output()
  end, { desc = "Cargo Runner: peek last/focused job output" })

  vim.api.nvim_create_user_command("CargoRunnerPeek", function(opts)
    local id = tonumber(opts.args)
    jobs.peek(id)
  end, { desc = "Cargo Runner: peek job output", nargs = "?" })

  vim.api.nvim_create_user_command("CargoRunnerJobs", function()
    jobs.picker()
  end, { desc = "Cargo Runner: list jobs" })

  vim.api.nvim_create_user_command("CargoRunnerKill", function(opts)
    local id = tonumber(opts.args)
    if opts.args == "all" then
      jobs.kill_all()
    else
      jobs.kill(id)
    end
  end, { desc = "Cargo Runner: kill job (or :CargoRunnerKill all)", nargs = "?" })

  vim.api.nvim_create_user_command("CargoRunnerCopyLastOutput", function()
    jobs.copy_last_output()
  end, { desc = "Cargo Runner: copy focused/last output" })

  vim.api.nvim_create_user_command("CargoRunnerStatus", function()
    run.status()
  end, { desc = "Cargo Runner: status" })
end

local function register_keymaps()
  local cfg = config.get()
  if not cfg.keymaps then
    return
  end

  local map = function(lhs, rhs, desc)
    if not lhs or lhs == "" then
      return
    end
    vim.keymap.set("n", lhs, rhs, {
      desc = desc,
      silent = true,
    })
  end

  -- Run never blocks the editor
  map(cfg.leader_run, function()
    run.at_cursor()
  end, "Cargo Runner: run (async)")

  map(cfg.leader_override, function()
    override.at_cursor()
  end, "Cargo Runner: override")

  -- Job UX
  map(cfg.leader_peek or "<leader>ro", function()
    jobs.peek()
  end, "Cargo Runner: peek output")
  map(cfg.leader_jobs or "<leader>rj", function()
    jobs.picker()
  end, "Cargo Runner: job list")
  map(cfg.leader_kill or "<leader>rk", function()
    jobs.kill()
  end, "Cargo Runner: kill focused job")

  if config.should_map_super() then
    map("<D-r>", function()
      run.at_cursor()
    end, "Cargo Runner: run (Super)")
    map("<D-S-r>", function()
      override.at_cursor()
    end, "Cargo Runner: override (Super)")
  end
end

---@param opts table|nil
function M.setup(opts)
  config.setup(opts)
  require("cargo_runner.cli").ensure_tool_path()
  register_commands()
  register_keymaps()
  vim.g.cargo_runner_setup_done = true
end

M.run = run
M.override = override
M.job = jobs
M.jobs = jobs
M.config = config

return M
