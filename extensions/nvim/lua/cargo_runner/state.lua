local M = {}

---@class CargoRunnerLastOutput
---@field exit_code integer
---@field stdout string
---@field stderr string
---@field combined string
---@field file_arg string
---@field shell string|nil
---@field cwd string|nil
---@field started_at number
---@field finished_at number

---@type CargoRunnerLastOutput|nil
M.last = nil

---@param data CargoRunnerLastOutput
function M.set_last(data)
  M.last = data
end

function M.get_last()
  return M.last
end

return M
