---Live / historical output peek for a single job. Focusable modal; q closes.

local M = {}

local win, buf
local follow_timer
local attached_id = nil
local auto_follow = true

local function stop_follow()
  if follow_timer then
    follow_timer:stop()
    follow_timer:close()
    follow_timer = nil
  end
end

function M.close()
  stop_follow()
  attached_id = nil
  if win and vim.api.nvim_win_is_valid(win) then
    pcall(vim.api.nvim_win_close, win, true)
  end
  win = nil
  if buf and vim.api.nvim_buf_is_valid(buf) then
    pcall(vim.api.nvim_buf_delete, buf, { force = true })
  end
  buf = nil
end

local function job_lines(job)
  local header = {
    string.format(
      "#%d %s %s  [%s]  %s",
      job.id,
      job.icon or "",
      job.label or "",
      job.status or "?",
      job.phase or ""
    ),
    job.shell and ("$ " .. job.shell) or "",
    string.rep("─", 40),
  }
  local body = {}
  if job.output_lines and #job.output_lines > 0 then
    body = job.output_lines
  elseif job.combined and job.combined ~= "" then
    body = vim.split(job.combined, "\n", { plain = true })
  else
    body = { "(no output yet — still running or silent)" }
  end
  local out = {}
  vim.list_extend(out, header)
  vim.list_extend(out, body)
  table.insert(out, "")
  table.insert(out, " q close · G bottom · gg top · y copy · x kill · <Space> toggle follow")
  return out
end

---@param job table
function M.open(job)
  if not job then
    return
  end
  M.close()
  attached_id = job.id
  auto_follow = true

  buf = vim.api.nvim_create_buf(false, true)
  vim.bo[buf].buftype = "nofile"
  vim.bo[buf].bufhidden = "wipe"
  vim.bo[buf].swapfile = false
  vim.bo[buf].filetype = "cargo-runner-output"
  vim.bo[buf].modifiable = true

  local width = math.min(100, math.max(50, vim.o.columns - 6))
  local height = math.min(28, math.max(10, vim.o.lines - 6))
  local row = math.floor((vim.o.lines - height) / 2)
  local col = math.floor((vim.o.columns - width) / 2)

  local lines = job_lines(job)
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)

  win = vim.api.nvim_open_win(buf, true, {
    relative = "editor",
    width = width,
    height = height,
    row = row,
    col = col,
    style = "minimal",
    border = "rounded",
    title = string.format(" Cargo Runner #%d · peek ", job.id),
    title_pos = "center",
    zindex = 250,
  })
  vim.wo[win].wrap = false
  vim.wo[win].cursorline = true
  vim.bo[buf].modifiable = false

  local map_opts = { buffer = buf, silent = true, nowait = true }
  vim.keymap.set("n", "q", function()
    M.close()
  end, map_opts)
  vim.keymap.set("n", "<Esc>", function()
    M.close()
  end, map_opts)
  vim.keymap.set("n", "y", function()
    local text = job.combined or table.concat(job.output_lines or {}, "\n")
    vim.fn.setreg("+", text)
    vim.fn.setreg('"', text)
    require("cargo_runner.ui.toast").info("Copied job #" .. job.id)
  end, map_opts)
  vim.keymap.set("n", "x", function()
    require("cargo_runner.jobs").kill(job.id)
  end, map_opts)
  vim.keymap.set("n", "<Space>", function()
    auto_follow = not auto_follow
  end, map_opts)

  -- Live refresh while job runs (does not block typing in other windows after close)
  follow_timer = vim.uv.new_timer()
  follow_timer:start(0, 200, function()
    vim.schedule(function()
      if not attached_id or not buf or not vim.api.nvim_buf_is_valid(buf) then
        stop_follow()
        return
      end
      local jobs = require("cargo_runner.jobs")
      local j = jobs.get(attached_id)
      if not j then
        stop_follow()
        return
      end
      local new_lines = job_lines(j)
      local at_bottom = false
      if win and vim.api.nvim_win_is_valid(win) then
        local cursor = vim.api.nvim_win_get_cursor(win)
        local last = vim.api.nvim_buf_line_count(buf)
        at_bottom = cursor[1] >= last - 2
      end
      vim.bo[buf].modifiable = true
      vim.api.nvim_buf_set_lines(buf, 0, -1, false, new_lines)
      vim.bo[buf].modifiable = false
      if auto_follow and at_bottom and win and vim.api.nvim_win_is_valid(win) then
        pcall(vim.api.nvim_win_set_cursor, win, { #new_lines, 0 })
      end
      if j.status == "done" or j.status == "failed" or j.status == "killed" then
        -- keep open for reading; stop heavy refresh after a few more ticks
      end
    end)
  end)
end

return M
