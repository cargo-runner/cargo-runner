---Job HUD is now part of the unified panel. Facade for jobs.lua call sites.

local M = {}

local function panel()
  return require("cargo_runner.ui.panel")
end

function M.close()
  -- Don't wipe notices; just re-render (jobs gone → notices-only or close)
  panel().render()
end

function M.occupied_rows()
  return panel().occupied_rows()
end

function M.is_open()
  return panel().is_open()
end

---@param _jobs table[] ignored — panel reads jobs registry itself
function M.render(_jobs)
  panel().render()
end

return M
