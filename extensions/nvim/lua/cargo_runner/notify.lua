local config = require("cargo_runner.config")

local M = {}

local function panel()
  return require("cargo_runner.ui.panel")
end

---@param msg string
---@param level integer|nil
function M.notify(msg, level)
  if not config.get().notify then
    return
  end
  level = level or vim.log.levels.INFO
  if level == vim.log.levels.ERROR then
    panel().push_notice("error", msg)
  elseif level == vim.log.levels.WARN then
    panel().push_notice("warn", msg)
  else
    panel().push_notice("info", msg)
  end
end

function M.info(msg)
  M.notify(msg, vim.log.levels.INFO)
end

function M.warn(msg)
  M.notify(msg, vim.log.levels.WARN)
end

function M.error(msg)
  M.notify(msg, vim.log.levels.ERROR)
end

-- Progress is panel jobs section only
function M.running(_label) end
function M.phase(_phase) end

---Success notice — no double "OK OK" prefix (panel adds ✓).
---@param label string
---@param elapsed_ms number|nil
function M.success(label, elapsed_ms)
  local secs = ""
  if elapsed_ms and elapsed_ms > 0 then
    secs = string.format(" · %.1fs", elapsed_ms / 1000)
  end
  panel().push_notice("success", (label or "done") .. secs)
end

---Fail notice — panel adds ✗; keep message clean.
---@param label string
function M.failed(label)
  panel().push_notice("error", "Failed " .. (label or "job"), 5000)
end

function M.close()
  panel().clear_notices()
end

return M
