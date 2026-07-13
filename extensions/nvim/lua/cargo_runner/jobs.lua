---Multi-job registry. All runs are fully async (never :wait() on the UI thread).

local cli = require("cargo_runner.cli")
local config = require("cargo_runner.config")
local kind_mod = require("cargo_runner.kind")
local progress = require("cargo_runner.progress")
local notify = require("cargo_runner.notify")
local hud = require("cargo_runner.ui.hud")
local peek = require("cargo_runner.ui.peek")
local error_float = require("cargo_runner.ui.error_float")
local state = require("cargo_runner.state")

local M = {}

---@type table<integer, table>
local jobs = {}
---@type integer[]
local order = {}
local next_id = 1
local MAX_LINES = 8000
local MAX_DONE = 30

local function job_env()
  local env = vim.fn.environ()
  local cfg = config.get()
  if cfg.quiet_cli ~= false then
    env.CARGO_RUNNER_QUIET = "1"
    env.CARGO_RUNNER_NO_EMOJI = env.CARGO_RUNNER_NO_EMOJI or "1"
  end
  env.CARGO_TERM_COLOR = env.CARGO_TERM_COLOR or "never"
  env.NO_COLOR = env.NO_COLOR or "1"
  return env
end

local function append_output(job, chunk)
  if not chunk or chunk == "" then
    return
  end
  job.combined = (job.combined or "") .. chunk
  -- ring buffer of lines for peek
  local parts = vim.split(chunk, "\n", { plain = true })
  job.output_lines = job.output_lines or {}
  -- merge first part into last line if incomplete
  if #job.output_lines > 0 and not job._line_complete then
    job.output_lines[#job.output_lines] = job.output_lines[#job.output_lines] .. (parts[1] or "")
    table.remove(parts, 1)
  end
  for i, p in ipairs(parts) do
    if i < #parts then
      table.insert(job.output_lines, p)
    elseif chunk:sub(-1) == "\n" then
      table.insert(job.output_lines, p)
      job._line_complete = true
    else
      table.insert(job.output_lines, p)
      job._line_complete = false
    end
  end
  while #job.output_lines > MAX_LINES do
    table.remove(job.output_lines, 1)
  end

  local phase = progress.phase_from_chunk(chunk)
  if phase and phase ~= job.phase then
    job.phase = phase
    hud.render(M.list())
  end
end

local function prune_done()
  local done = {}
  for _, id in ipairs(order) do
    local j = jobs[id]
    if j and (j.status == "done" or j.status == "failed" or j.status == "killed") then
      table.insert(done, id)
    end
  end
  while #done > MAX_DONE do
    local id = table.remove(done, 1)
    jobs[id] = nil
    for i, oid in ipairs(order) do
      if oid == id then
        table.remove(order, i)
        break
      end
    end
  end
end

local function finish_job(job, code)
  job.exit_code = code
  job.finished_at = vim.uv.hrtime() / 1e6
  job.handle = nil
  if code == 0 then
    job.status = "done"
    job.phase = "done"
  elseif code == -15 or code == 143 or job.status == "killing" then
    job.status = "killed"
    job.phase = "killed"
  else
    job.status = "failed"
    job.phase = "failed"
  end

  local elapsed = (job.finished_at or 0) - (job.started_at or 0)
  state.set_last({
    exit_code = code,
    stdout = job.combined or "",
    stderr = "",
    combined = job.combined or "",
    file_arg = job.file_arg,
    shell = job.shell,
    cwd = job.cwd,
    started_at = job.started_at,
    finished_at = job.finished_at,
    job_id = job.id,
    kind = job.kind,
  })

  prune_done()

  -- Atomic: job already left "active" set; one panel render via push_notice
  -- (no separate hud.render + toast that could double-paint the corner).
  local icon = job.icon or ""
  local label = string.format("%s #%d %s", icon, job.id, job.label)

  if job.status == "done" then
    notify.success(label, elapsed)
  elseif job.status == "killed" then
    notify.warn("Killed " .. label)
  else
    notify.failed(label)
  end

  if job.status == "failed" then
    local cfg = config.get()
    if cfg.open_error_on_fail and not job.long_running then
      vim.defer_fn(function()
        error_float.open(job.combined or "", {
          title = string.format("Cargo Runner #%d failed (exit %s)", job.id, tostring(code)),
          file_arg = job.file_arg,
          on_rerun = function()
            M.start({
              file_arg = job.file_arg,
              cwd = job.cwd,
              label = job.label,
            })
          end,
        })
      end, 150)
    end
  end
end

local function start_process(job, argv)
  job.status = "running"
  job.phase = job.long_running and "running (indefinite)…" or "Starting build…"
  hud.render(M.list())

  job.handle = vim.system(argv, {
    cwd = job.cwd,
    text = true,
    env = job_env(),
    stdout = function(_, data)
      if data then
        -- schedule UI updates; buffer append is ok from callback via schedule
        vim.schedule(function()
          if jobs[job.id] then
            append_output(job, data)
          end
        end)
      end
    end,
    stderr = function(_, data)
      if data then
        vim.schedule(function()
          if jobs[job.id] then
            append_output(job, data)
          end
        end)
      end
    end,
  }, function(result)
    vim.schedule(function()
      local j = jobs[job.id]
      if not j then
        return
      end
      -- flush any remaining buffered text from result
      if result.stdout and result.stdout ~= "" and #(j.combined or "") == 0 then
        append_output(j, result.stdout)
      end
      if result.stderr and result.stderr ~= "" then
        append_output(j, result.stderr)
      end
      finish_job(j, result.code or -1)
    end)
  end)
end

local function with_quiet(argv)
  local cfg = config.get()
  if cfg.quiet_cli == false then
    return argv
  end
  local insert_at = 2
  if argv[1] and vim.fn.fnamemodify(argv[1], ":t") == "cargo" and argv[2] == "runner" then
    insert_at = 3
  end
  table.insert(argv, insert_at, "--quiet")
  return argv
end

---Start a cargo-runner job. Returns immediately (never blocks typing).
---@param opts { file_arg: string, cwd?: string, label?: string }
---@return integer job_id
function M.start(opts)
  local file_arg = opts.file_arg
  local cwd = opts.cwd or cli.project_cwd()
  local label = opts.label or file_arg

  local cfg = config.get()
  if cfg.save_before_run and vim.bo.modified then
    -- write is quick; still don't run_sync cargo
    pcall(vim.cmd, "write")
  end

  local bin, berr = cli.resolve_binary()
  if not bin then
    notify.error(berr or "cargo-runner not found")
    return -1
  end

  local id = next_id
  next_id = next_id + 1

  ---@type table
  local job = {
    id = id,
    kind = "unknown",
    icon = kind_mod.display_icon("unknown", cfg.no_emoji),
    label = label,
    file_arg = file_arg,
    cwd = cwd,
    shell = nil,
    status = "starting",
    phase = "Resolving…",
    long_running = false,
    combined = "",
    output_lines = {},
    _line_complete = true,
    handle = nil,
    started_at = vim.uv.hrtime() / 1e6,
    finished_at = nil,
    exit_code = nil,
  }
  jobs[id] = job
  table.insert(order, id)
  -- HUD only for progress (no second toast window — that was stacking/overlapping)
  hud.render(M.list())

  -- Fully async dry-run → then async run. UI stays responsive.
  cli.dry_run_async(file_arg, cwd, function(ok, dry)
    local j = jobs[id]
    if not j or j.status == "killed" or j.status == "killing" then
      return
    end

    local shell, strategy
    if ok and type(dry) == "table" then
      shell = dry.shell
      strategy = dry.strategy
      if dry.cwd and dry.cwd ~= "" then
        j.cwd = dry.cwd
      end
    end
    j.shell = shell
    j.kind = kind_mod.classify(shell, strategy)
    local info = kind_mod.info(j.kind)
    j.icon = kind_mod.display_icon(j.kind, cfg.no_emoji)
    j.long_running = info.long_running
    j.phase = "Starting…"
    hud.render(M.list())

    local argv = with_quiet(cli.build_argv({ "run", file_arg }))
    -- Always non-blocking background capture (even servers) so peek works
    -- and the editor never freezes. Optional terminal mode via config.
    if j.long_running and cfg.prefer_terminal_for_long_running and cfg.force_terminal_for_long_running then
      -- explicit opt-in only
      j.phase = "terminal"
      hud.render(M.list())
      local cmd = table.concat(vim.tbl_map(function(a)
        return a:find("%s") and vim.fn.shellescape(a) or a
      end, argv), " ")
      vim.cmd("botright split | resize 12")
      local term_buf = vim.api.nvim_get_current_buf()
      vim.fn.termopen(cmd, { cwd = j.cwd, env = job_env() })
      j.term_buf = term_buf
      j.status = "running"
      j.combined = "(output in terminal buffer)\n" .. cmd
      -- leave insert so user can keep editing other windows
      vim.cmd("stopinsert")
      vim.cmd("wincmd p")
      return
    end

    start_process(j, argv)
  end)

  return id
end

---@return table[]
function M.list()
  local out = {}
  for _, id in ipairs(order) do
    if jobs[id] then
      table.insert(out, jobs[id])
    end
  end
  return out
end

---@param id integer
function M.get(id)
  return jobs[id]
end

function M.get_focused()
  -- prefer running long jobs, else latest running, else latest any
  local list = M.list()
  for i = #list, 1, -1 do
    local j = list[i]
    if (j.status == "running" or j.status == "starting") and j.long_running then
      return j
    end
  end
  for i = #list, 1, -1 do
    local j = list[i]
    if j.status == "running" or j.status == "starting" then
      return j
    end
  end
  return list[#list]
end

---@param id integer|nil
function M.peek(id)
  local j = id and jobs[id] or M.get_focused()
  if not j then
    notify.warn("No jobs yet — press <leader>r first")
    return
  end
  peek.open(j)
end

---@param id integer|nil
function M.kill(id)
  local j = id and jobs[id] or M.get_focused()
  if not j then
    notify.warn("No running job to kill")
    return
  end
  if j.status ~= "running" and j.status ~= "starting" then
    notify.info(string.format("#%d already %s", j.id, j.status))
    return
  end
  j.status = "killing"
  j.phase = "killing…"
  hud.render(M.list())
  if j.handle then
    pcall(function()
      j.handle:kill("sigterm")
    end)
    -- escalate if needed
    vim.defer_fn(function()
      local jj = jobs[j.id]
      if jj and jj.handle then
        pcall(function()
          jj.handle:kill("sigkill")
        end)
      end
    end, 2000)
  end
  if j.term_buf and vim.api.nvim_buf_is_valid(j.term_buf) then
    pcall(vim.api.nvim_buf_delete, j.term_buf, { force = true })
    finish_job(j, -15)
  end
end

function M.kill_all()
  for _, j in ipairs(M.list()) do
    if j.status == "running" or j.status == "starting" then
      M.kill(j.id)
    end
  end
end

function M.picker()
  local list = M.list()
  if #list == 0 then
    notify.warn("No jobs")
    return
  end
  local items = {}
  for _, j in ipairs(list) do
    table.insert(items, string.format(
      "#%d %s %-7s %-10s %s · %s",
      j.id,
      j.icon or "",
      j.kind or "?",
      j.status or "?",
      j.label or "",
      j.phase or ""
    ))
  end
  vim.ui.select(items, { prompt = "Cargo Runner jobs" }, function(choice, idx)
    if not choice or not idx then
      return
    end
    local j = list[idx]
    vim.ui.select({
      "Peek output",
      "Kill",
      "Copy output",
      "Rerun",
    }, { prompt = string.format("Job #%d", j.id) }, function(action)
      if action == "Peek output" then
        M.peek(j.id)
      elseif action == "Kill" then
        M.kill(j.id)
      elseif action == "Copy output" then
        vim.fn.setreg("+", j.combined or "")
        vim.fn.setreg('"', j.combined or "")
        notify.info("Copied #" .. j.id)
      elseif action == "Rerun" then
        M.start({
          file_arg = j.file_arg,
          cwd = j.cwd,
          label = j.label,
        })
      end
    end)
  end)
end

-- Back-compat shims used by older modules
function M.run(opts)
  return M.start(opts)
end

function M.show_last_output()
  local last = state.get_last()
  if last and last.job_id and jobs[last.job_id] then
    M.peek(last.job_id)
    return
  end
  local j = M.get_focused()
  if j then
    M.peek(j.id)
    return
  end
  if last then
    error_float.open(last.combined or "", {
      title = "Cargo Runner — last output",
    })
    return
  end
  notify.warn("No output yet")
end

function M.copy_last_output()
  local j = M.get_focused()
  local text = j and j.combined or (state.get_last() and state.get_last().combined)
  if not text or text == "" then
    notify.warn("No output to copy")
    return
  end
  vim.fn.setreg("+", text)
  vim.fn.setreg("*", text)
  vim.fn.setreg('"', text)
  notify.info("Copied output")
end

return M
