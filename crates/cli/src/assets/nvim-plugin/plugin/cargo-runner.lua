-- Auto-loaded when the pack is on packpath.
-- Safe to re-source; setup is idempotent.
if vim.g.loaded_cargo_runner then
  return
end
vim.g.loaded_cargo_runner = true

-- Defer so user's init.lua can call setup() first if they want.
vim.api.nvim_create_autocmd("VimEnter", {
  once = true,
  callback = function()
    local ok, cr = pcall(require, "cargo_runner")
    if not ok then
      vim.notify("cargo-runner: failed to load: " .. tostring(cr), vim.log.levels.ERROR)
      return
    end
    if not vim.g.cargo_runner_setup_done then
      cr.setup()
    end
  end,
})
