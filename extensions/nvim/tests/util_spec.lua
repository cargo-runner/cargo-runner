-- Pure-Lua unit tests. These load `cargo_runner.util` and `cargo_runner.kind`
-- directly — neither touches `vim.*`, so no Neovim is required.
package.path = "lua/?.lua;lua/?/init.lua;" .. package.path

local util = require("cargo_runner.util")
local kind = require("cargo_runner.kind")

local failures = 0
local function check(name, ok)
  if ok then
    print("ok   - " .. name)
  else
    print("FAIL - " .. name)
    failures = failures + 1
  end
end

local function eq(a, b)
  if type(a) ~= "table" or type(b) ~= "table" then
    return a == b
  end
  if #a ~= #b then
    return false
  end
  for i = 1, #a do
    if a[i] ~= b[i] then
      return false
    end
  end
  return true
end

-- is_long_running: decides whether a run goes to a terminal buffer.
check("serve is long running", util.is_long_running("dx serve --port 8080") == true)
check("watch is long running", util.is_long_running("cargo leptos watch") == true)
check("test is not long running", util.is_long_running("cargo test --lib") == false)
check("nil is not long running", util.is_long_running(nil) == false)
check("empty is not long running", util.is_long_running("") == false)

-- tokenize: feeds an argv list, never a shell string.
check("splits on whitespace", eq(util.tokenize("a b c"), { "a", "b", "c" }))
check("keeps double-quoted spans", eq(util.tokenize('a "b c" d'), { "a", "b c", "d" }))
check("keeps single-quoted spans", eq(util.tokenize("a 'b c' d"), { "a", "b c", "d" }))
check("collapses repeated whitespace", eq(util.tokenize("  a   b  "), { "a", "b" }))
check("empty input yields no tokens", eq(util.tokenize(""), {}))
-- Metacharacters stay inside one token: they are argv data, not shell syntax.
check("shell metacharacters are not split", eq(util.tokenize("a;id b"), { "a;id", "b" }))

-- kind.classify: drives the icon/long-running UI.
check("classify returns a string", type(kind.classify("cargo test", "cargo")) == "string")
check("classify tolerates nil", type(kind.classify(nil, nil)) == "string")

if failures > 0 then
  print(string.format("\n%d test(s) failed", failures))
  os.exit(1)
end
print("\nall lua tests passed")
