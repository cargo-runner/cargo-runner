---Notifications go through the unified panel (fixed width, no overlap).
---This module is a thin facade for existing notify.* call sites.

local M = {}

local function panel()
  return require("cargo_runner.ui.panel")
end

function M.reflow()
  panel().render()
end

function M.occupied_rows()
  return panel().occupied_rows()
end

function M.close()
  panel().clear_notices()
end

function M.close_progress_only() end

function M.progress(_label, _phase) end
function M.set_phase(_phase) end

function M.success(msg, timeout_ms)
  panel().push_notice("success", msg or "Done", timeout_ms)
end

function M.error(msg, timeout_ms)
  panel().push_notice("error", msg or "Failed", timeout_ms or 5000)
end

function M.info(msg, timeout_ms)
  panel().push_notice("info", msg or "", timeout_ms)
end

function M.warn(msg, timeout_ms)
  panel().push_notice("warn", msg or "", timeout_ms or 3500)
end

function M.dismiss_newest()
  -- clear all notices is enough for now
  panel().clear_notices()
end

return M
