---Unified status toaster (UX-hardened).
---
---Design rules (TUI):
---  * ONE float only — never multi-window stacks in the same corner
---  * FIXED width (50 cols) — every line padded/truncated to the same edge
---  * Slot caps: 3 jobs + 3 notices — overflow counters, no unbounded growth
---  * Jobs (live) above notices (history); discrete reflow on enter/leave
---  * focusable=false — never steals typing
---  * Full logs only in center modals (peek / error_float, z=250)

local kind_mod = require("cargo_runner.kind")

local M = {}

-- ── Spec constants ──────────────────────────────────────────────────────────
local PANEL_WIDTH = 50
local PANEL_MIN_W = 32
local MAX_JOBS_VISIBLE = 3
local MAX_NOTICES = 3
local ZINDEX_PANEL = 200
local SPIN_MS = 120
local EXPIRE_TICK_MS = 250
local TTL_SUCCESS_MS = 2500
local TTL_INFO_MS = 2500
local TTL_WARN_MS = 3500
local TTL_ERROR_MS = 5000
local SPIN = { "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏" }
local SPIN_ASCII = { "|", "/", "-", "\\" }

local win, buf
local spin_timer, expire_timer
local spin_i = 1

---@class PanelNotice
---@field id integer
---@field kind string
---@field text string
---@field expires number

---@type PanelNotice[]
local notices = {}
local next_notice_id = 1

local function cfg()
  local ok, c = pcall(require, "cargo_runner.config")
  if ok then
    return c.get()
  end
  return {}
end

local function no_emoji()
  return cfg().no_emoji == true
end

local function panel_width()
  local cols = vim.o.columns or 80
  return math.min(PANEL_WIDTH, math.max(PANEL_MIN_W, cols - 2))
end

local function now_ms()
  return vim.uv.hrtime() / 1e6
end

local function truncate(s, max_w)
  if vim.fn.strdisplaywidth(s) <= max_w then
    return s
  end
  while #s > 0 and vim.fn.strdisplaywidth(s) > max_w - 1 do
    s = s:sub(1, #s - 1)
  end
  return s .. "…"
end

---Pad or truncate to exactly `inner_w` display cells (uniform right edge).
local function fit(s, inner_w)
  s = s or ""
  local w = vim.fn.strdisplaywidth(s)
  if w > inner_w then
    return truncate(s, inner_w)
  end
  if w < inner_w then
    return s .. string.rep(" ", inner_w - w)
  end
  return s
end

local function notice_prefix(kind)
  local ne = no_emoji()
  if kind == "success" then
    return ne and "OK " or "✓ "
  end
  if kind == "error" then
    return ne and "ERR " or "✗ "
  end
  if kind == "warn" then
    return ne and "! " or "⚠ "
  end
  return ne and "· " or "· "
end

local function hl_notice(kind)
  if kind == "error" then
    return "DiagnosticError"
  end
  if kind == "warn" then
    return "DiagnosticWarn"
  end
  if kind == "success" then
    return "DiagnosticOk"
  end
  return "DiagnosticInfo"
end

local function stop_spin()
  if spin_timer then
    pcall(function()
      spin_timer:stop()
      spin_timer:close()
    end)
    spin_timer = nil
  end
end

local function stop_expire()
  if expire_timer then
    pcall(function()
      expire_timer:stop()
      expire_timer:close()
    end)
    expire_timer = nil
  end
end

function M.close()
  stop_spin()
  stop_expire()
  if win and vim.api.nvim_win_is_valid(win) then
    pcall(vim.api.nvim_win_close, win, true)
  end
  win = nil
  if buf and vim.api.nvim_buf_is_valid(buf) then
    pcall(vim.api.nvim_buf_delete, buf, { force = true })
  end
  buf = nil
end

function M.is_open()
  return win ~= nil and vim.api.nvim_win_is_valid(win)
end

function M.occupied_rows()
  if not M.is_open() then
    return 0
  end
  local c = vim.api.nvim_win_get_config(win)
  return (tonumber(c.row) or 0) + (tonumber(c.height) or 1) + 2
end

local function prune_notices()
  local t = now_ms()
  local kept = {}
  for _, n in ipairs(notices) do
    if n.expires > t then
      table.insert(kept, n)
    end
  end
  notices = kept
end

local function active_jobs()
  local ok, jobs_mod = pcall(require, "cargo_runner.jobs")
  if not ok then
    return {}
  end
  local out = {}
  for _, j in ipairs(jobs_mod.list()) do
    if j.status == "starting" or j.status == "running" or j.status == "killing" then
      table.insert(out, j)
    end
  end
  return out
end

---Build job line with priority truncation: keep spin+icon+#id, shrink label then phase.
local function format_job_line(j, inner_w, spin_char)
  local ne = no_emoji()
  local icon = kind_mod.display_icon(j.kind, ne)
  local idpart = string.format("#%d", j.id)
  local label = j.label or "?"
  local phase = j.phase or j.status or ""
  local head = string.format("%s %s %s ", spin_char, icon, idpart)
  local head_w = vim.fn.strdisplaywidth(head)
  local budget = inner_w - 1 - head_w -- leading space in panel line
  if budget < 8 then
    return fit(" " .. truncate(head .. label, inner_w - 1), inner_w)
  end
  local sep = " · "
  local label_w = math.floor(budget * 0.55)
  local phase_w = budget - label_w - vim.fn.strdisplaywidth(sep)
  if phase_w < 4 then
    phase_w = 4
    label_w = budget - phase_w - vim.fn.strdisplaywidth(sep)
  end
  label = truncate(label, math.max(4, label_w))
  phase = truncate(phase, math.max(3, phase_w))
  return fit(" " .. head .. label .. sep .. phase, inner_w)
end

local function build_lines()
  prune_notices()
  local width = panel_width()
  local inner = width -- buffer lines fill full win width; border is chrome outside
  -- With rounded border, content width == win width; pad full width for even edge
  local jobs = active_jobs()
  local lines = {}
  ---@type {row:integer, hl:string}[]
  local hls = {}

  local ne = no_emoji()
  local spin_char
  if ne then
    spin_char = SPIN_ASCII[((spin_i - 1) % #SPIN_ASCII) + 1]
  else
    spin_char = SPIN[((spin_i - 1) % #SPIN) + 1]
  end

  -- Jobs section (no body title — window title is enough)
  local shown = 0
  local total_jobs = #jobs
  for _, j in ipairs(jobs) do
    if shown >= MAX_JOBS_VISIBLE then
      break
    end
    shown = shown + 1
    table.insert(lines, format_job_line(j, inner, spin_char))
    table.insert(hls, { row = #lines - 1, hl = "DiagnosticInfo" })
  end
  if total_jobs > MAX_JOBS_VISIBLE then
    local more = total_jobs - MAX_JOBS_VISIBLE
    table.insert(lines, fit(string.format(" … +%d more · rj", more), inner))
    table.insert(hls, { row = #lines - 1, hl = "Comment" })
  end

  if total_jobs > 0 then
    local hint = inner >= 40 and " ro peek · rj jobs · rk kill" or " ro·rj·rk"
    table.insert(lines, fit(hint, inner))
    table.insert(hls, { row = #lines - 1, hl = "Comment" })
  end

  -- Notices section
  if #notices > 0 then
    if total_jobs > 0 then
      table.insert(lines, fit(" " .. string.rep("─", math.min(20, inner - 2)), inner))
      table.insert(hls, { row = #lines - 1, hl = "Comment" })
    end
    for _, n in ipairs(notices) do
      local line = notice_prefix(n.kind) .. (n.text or "")
      table.insert(lines, fit(" " .. line, inner))
      table.insert(hls, { row = #lines - 1, hl = hl_notice(n.kind) })
    end
  end

  if #lines == 0 then
    return nil, nil, width
  end
  return lines, hls, width
end

function M.render()
  local lines, hls, width = build_lines()
  if not lines then
    M.close()
    return
  end

  if not buf or not vim.api.nvim_buf_is_valid(buf) then
    buf = vim.api.nvim_create_buf(false, true)
    vim.bo[buf].buftype = "nofile"
    vim.bo[buf].bufhidden = "wipe"
    vim.bo[buf].swapfile = false
  end

  local height = #lines
  local col = math.max(0, (vim.o.columns or 80) - width - 1)

  vim.bo[buf].modifiable = true
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
  vim.bo[buf].modifiable = false

  local ns = vim.api.nvim_create_namespace("cargo_runner_panel")
  pcall(vim.api.nvim_buf_clear_namespace, buf, ns, 0, -1)
  if hls then
    for _, h in ipairs(hls) do
      pcall(vim.api.nvim_buf_add_highlight, buf, ns, h.hl, h.row, 0, -1)
    end
  end

  local wincfg = {
    relative = "editor",
    width = width,
    height = height,
    row = 0,
    col = col,
    style = "minimal",
    border = "rounded",
    focusable = false,
    zindex = ZINDEX_PANEL,
    noautocmd = true,
    title = " cargo-runner ",
    title_pos = "right",
  }

  if win and vim.api.nvim_win_is_valid(win) then
    pcall(vim.api.nvim_win_set_config, win, wincfg)
  else
    local ok, w = pcall(vim.api.nvim_open_win, buf, false, wincfg)
    if ok then
      win = w
      pcall(function()
        vim.wo[win].winhl = "Normal:NormalFloat,FloatBorder:FloatBorder"
        vim.wo[win].wrap = false
        vim.wo[win].cursorline = false
        vim.wo[win].number = false
        vim.wo[win].relativenumber = false
        vim.wo[win].signcolumn = "no"
      end)
    end
  end

  local jobs = active_jobs()
  if #jobs > 0 then
    if not spin_timer then
      spin_timer = vim.uv.new_timer()
      spin_timer:start(0, SPIN_MS, function()
        spin_i = spin_i + 1
        vim.schedule(function()
          -- Only re-render if still open / has jobs
          if #active_jobs() > 0 or #notices > 0 then
            M.render()
          else
            stop_spin()
          end
        end)
      end)
    end
  else
    stop_spin()
  end

  if #notices > 0 then
    if not expire_timer then
      expire_timer = vim.uv.new_timer()
      expire_timer:start(EXPIRE_TICK_MS, EXPIRE_TICK_MS, function()
        vim.schedule(function()
          local before = #notices
          prune_notices()
          if #notices ~= before then
            M.render()
          end
          if #notices == 0 then
            stop_expire()
            if #active_jobs() == 0 then
              M.close()
            end
          end
        end)
      end)
    end
  else
    stop_expire()
  end
end

---@param kind string
---@param text string
---@param ttl_ms integer|nil
function M.push_notice(kind, text, ttl_ms)
  if cfg().notify == false then
    return
  end

  local default_ttl = TTL_INFO_MS
  if kind == "success" then
    default_ttl = cfg().toast_timeout_ms or TTL_SUCCESS_MS
  elseif kind == "error" then
    default_ttl = TTL_ERROR_MS
  elseif kind == "warn" then
    default_ttl = TTL_WARN_MS
  else
    default_ttl = cfg().toast_timeout_ms or TTL_INFO_MS
  end

  table.insert(notices, 1, {
    id = next_notice_id,
    kind = kind,
    text = text or "",
    expires = now_ms() + (ttl_ms or default_ttl),
  })
  next_notice_id = next_notice_id + 1
  while #notices > MAX_NOTICES do
    table.remove(notices) -- drop oldest
  end
  M.render()
end

function M.clear_notices()
  notices = {}
  M.render()
end

function M.reflow()
  M.render()
end

function M.close_progress_only() end

return M
