---Map cargo / bazel / rustc log lines → short human phases for the toast.

local M = {}

-- Ordered: first match wins when scanning a line (more specific first).
local RULES = {
  { re = "[Uu]pdating crates%.io", phase = "Updating crate index…" },
  { re = "[Dd]ownload", phase = "Downloading crates…" },
  { re = "[Ii]nstalled", phase = "Installing dependencies…" },
  { re = "[Ii]nstalling", phase = "Installing dependencies…" },
  { re = "Fetched", phase = "Fetching dependencies…" },
  { re = "Blocking waiting", phase = "Waiting on crate lock…" },
  { re = "Compiling", phase = "Compiling…" },
  { re = "Checking", phase = "Checking…" },
  { re = "Building", phase = "Building…" },
  { re = "Finished `", phase = "Build finished…" },
  { re = "Running (unittests|tests|doctests|bin)", phase = "Running…" },
  { re = "running %d+ test", phase = "Running tests…" },
  { re = "test result:", phase = "Tests done…" },
  { re = "Doc%-tests", phase = "Running doctests…" },
  { re = "Analyzing:", phase = "Analyzing (Bazel)…" },
  { re = "INFO: From", phase = "Building (Bazel)…" },
  { re = "Executing tests", phase = "Running tests (Bazel)…" },
  { re = "PASSED", phase = "Passed…" },
  { re = "FAILED", phase = "Failed…" },
  { re = "error%[%w+%]", phase = "Compile error…" },
  { re = "^error:", phase = "Error…" },
}

---Infer phase from a chunk of stdout/stderr. Returns nil if nothing useful.
---@param chunk string
---@return string|nil
function M.phase_from_chunk(chunk)
  if not chunk or chunk == "" then
    return nil
  end
  -- Scan last few lines — most recent signal wins
  local lines = vim.split(chunk, "\n", { plain = true })
  local start = math.max(1, #lines - 12)
  local found = nil
  for i = start, #lines do
    local line = lines[i]
    if line and line ~= "" then
      for _, rule in ipairs(RULES) do
        if line:find(rule.re) then
          found = rule.phase
          break
        end
      end
    end
  end
  return found
end

return M
