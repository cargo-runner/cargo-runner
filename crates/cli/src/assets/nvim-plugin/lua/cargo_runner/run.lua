local cli = require("cargo_runner.cli")
local jobs = require("cargo_runner.jobs")
local notify = require("cargo_runner.notify")

local M = {}

function M.at_cursor()
  local file_arg, err = cli.cursor_file_arg()
  if not file_arg then
    notify.error(err or "cannot run")
    return
  end
  -- Returns immediately — dry-run + cargo all async
  local cwd = cli.project_cwd()
  local label = vim.fn.fnamemodify(file_arg:gsub(":%d+$", ""), ":t")
    .. ":"
    .. (file_arg:match(":(%d+)$") or "?")
  jobs.start({
    file_arg = file_arg,
    cwd = cwd,
    label = label,
  })
end

function M.dry_run_at_cursor()
  local file_arg, err = cli.cursor_file_arg()
  if not file_arg then
    notify.error(err or "cannot dry-run")
    return
  end
  local cwd = cli.project_cwd()
  notify.info("Dry-run…")
  -- async so statusline/typing never freeze
  cli.dry_run_async(file_arg, cwd, function(ok, dry)
    if not ok then
      notify.error(tostring(dry))
      return
    end
    local lines = {
      "file: " .. file_arg,
      "cwd:  " .. (dry.cwd or cwd),
      "strategy: " .. tostring(dry.strategy or "?"),
      "shell: " .. tostring(dry.shell or "?"),
    }
    if dry.warnings and #dry.warnings > 0 then
      table.insert(lines, "warnings:")
      for _, w in ipairs(dry.warnings) do
        table.insert(lines, "  - " .. tostring(w))
      end
    end
    require("cargo_runner.ui.error_float").open(table.concat(lines, "\n"), {
      title = "Cargo Runner — dry-run",
    })
  end)
end

function M.status()
  local bin, berr = cli.resolve_binary()
  local lines = {}
  if bin then
    table.insert(lines, "binary: " .. bin)
    -- version check async too
    vim.system({ bin, "--version" }, { text = true }, function(result)
      vim.schedule(function()
        if result.code == 0 then
          table.insert(lines, "version: " .. vim.trim(result.stdout or result.stderr or ""))
        end
        table.insert(lines, "cwd: " .. cli.project_cwd())
        local file_arg = select(1, cli.cursor_file_arg())
        table.insert(lines, "cursor: " .. (file_arg or "(not a rust buffer)"))
        local running = 0
        for _, j in ipairs(require("cargo_runner.jobs").list()) do
          if j.status == "running" or j.status == "starting" then
            running = running + 1
          end
        end
        table.insert(lines, "active jobs: " .. tostring(running))
        table.insert(lines, "keymaps: <leader>r run · <leader>ro peek · <leader>rj jobs · <leader>rk kill")
        require("cargo_runner.ui.error_float").open(table.concat(lines, "\n"), {
          title = "Cargo Runner — status",
        })
      end)
    end)
  else
    table.insert(lines, "binary: MISSING — " .. (berr or ""))
    require("cargo_runner.ui.error_float").open(table.concat(lines, "\n"), {
      title = "Cargo Runner — status",
    })
  end
end

return M
