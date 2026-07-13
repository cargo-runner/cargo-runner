local cli = require("cargo_runner.cli")
local job = require("cargo_runner.job")
local notify = require("cargo_runner.notify")

local M = {}

local TOKEN_HELP =
  "Tokens: @cmd.sub  +channel  KEY=val  /test-args  --flags  |  @ append  !! clear  !env"

---Simple shell-ish tokenize (handles quotes loosely).
---@param s string
---@return string[]
function M.tokenize(s)
  local tokens = {}
  local i = 1
  local n = #s
  while i <= n do
    while i <= n and s:sub(i, i):match("%s") do
      i = i + 1
    end
    if i > n then
      break
    end
    local c = s:sub(i, i)
    if c == '"' or c == "'" then
      local q = c
      i = i + 1
      local start = i
      while i <= n and s:sub(i, i) ~= q do
        i = i + 1
      end
      table.insert(tokens, s:sub(start, i - 1))
      if i <= n then
        i = i + 1
      end
    else
      local start = i
      while i <= n and not s:sub(i, i):match("%s") do
        i = i + 1
      end
      table.insert(tokens, s:sub(start, i - 1))
    end
  end
  return tokens
end

local ACTIONS = {
  ["Save & Run"] = "save-run",
  ["Preview only (dry-run)"] = "preview",
  ["Save only"] = "save",
}

local function after_save_actions(file_arg, cwd, tokens)
  vim.ui.select({
    "Save & Run",
    "Preview only (dry-run)",
    "Save only",
  }, {
    prompt = "Cargo Runner override — next step",
  }, function(choice)
    if not choice then
      return
    end
    local value = ACTIONS[choice]
    if not value then
      return
    end

    local ok, err = pcall(cli.set_override, file_arg, tokens, cwd)
    if not ok then
      notify.error("Failed to save override: " .. tostring(err))
      return
    end

    if value == "save" then
      notify.success("Override saved")
      return
    end

    if value == "preview" then
      local dok, dry = pcall(cli.dry_run, file_arg, cwd)
      if not dok then
        notify.error("Preview failed: " .. tostring(dry))
        return
      end
      notify.info("Preview: " .. tostring(dry.shell))
      require("cargo_runner.ui.error_float").open(
        "After override:\n" .. (dry.shell or vim.inspect(dry)),
        { title = "Cargo Runner — override preview" }
      )
      return
    end

    -- save-run
    notify.success("Override saved — running")
    local label = vim.fn.fnamemodify(file_arg:gsub(":%d+$", ""), ":t")
      .. ":"
      .. (file_arg:match(":(%d+)$") or "?")
    job.run({ file_arg = file_arg, cwd = cwd, label = label })
  end)
end

function M.at_cursor()
  local file_arg, err = cli.cursor_file_arg()
  if not file_arg then
    notify.error(err or "cannot override")
    return
  end
  local cwd = cli.project_cwd()

  -- Optional dry-run before for context
  local before = nil
  local dok, dry = pcall(cli.dry_run, file_arg, cwd)
  if dok and dry and dry.shell then
    before = dry.shell
  end

  local prompt = TOKEN_HELP
  if before then
    prompt = "Current: " .. before .. "\n" .. TOKEN_HELP
  end

  vim.ui.input({
    prompt = "Cargo Runner Override: ",
    default = "",
  }, function(input)
    if input == nil then
      return -- cancelled
    end

    local trimmed = vim.trim(input)
    local tokens = M.tokenize(trimmed)

    if #tokens == 0 then
      -- empty → run only (VS Code behavior)
      local label = vim.fn.fnamemodify(file_arg:gsub(":%d+$", ""), ":t")
        .. ":"
        .. (file_arg:match(":(%d+)$") or "?")
      job.run({ file_arg = file_arg, cwd = cwd, label = label })
      return
    end

    if #tokens == 1 and (tokens[1] == "-" or tokens[1] == "!!") then
      local ok, oerr = pcall(cli.set_override, file_arg, tokens, cwd)
      if not ok then
        notify.error("Failed to remove override: " .. tostring(oerr))
        return
      end
      notify.success("Override removed")
      return
    end

    if trimmed == "@." then
      notify.error("Incomplete @cmd.sub token")
      return
    end

    -- Show help in echo if useful
    if before then
      vim.api.nvim_echo({ { prompt, "Comment" } }, false, {})
    end

    after_save_actions(file_arg, cwd, tokens)
  end)
end

return M
