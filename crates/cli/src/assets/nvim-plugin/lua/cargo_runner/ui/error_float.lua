local M = {}

---@param text string
---@param opts { title?: string, file_arg?: string, on_rerun?: function }|nil
function M.open(text, opts)
  opts = opts or {}
  text = text or ""
  if text == "" then
    text = "(no output)"
  end

  local lines = vim.split(text, "\n", { plain = true })
  local buf = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
  vim.bo[buf].buftype = "nofile"
  vim.bo[buf].bufhidden = "wipe"
  vim.bo[buf].swapfile = false
  vim.bo[buf].modifiable = false
  vim.bo[buf].filetype = "cargo-runner-output"

  local width = math.min(100, math.max(40, vim.o.columns - 8))
  local height = math.min(30, math.max(8, vim.o.lines - 6))
  local row = math.floor((vim.o.lines - height) / 2)
  local col = math.floor((vim.o.columns - width) / 2)

  local win = vim.api.nvim_open_win(buf, true, {
    relative = "editor",
    width = width,
    height = height,
    row = row,
    col = col,
    style = "minimal",
    border = "rounded",
    title = opts.title or "Cargo Runner output",
    title_pos = "center",
    zindex = 250, -- above status panel (200); center modal only
  })

  vim.wo[win].wrap = false
  vim.wo[win].cursorline = true
  vim.wo[win].number = false
  vim.wo[win].relativenumber = false

  local function close()
    if vim.api.nvim_win_is_valid(win) then
      vim.api.nvim_win_close(win, true)
    end
  end

  local function yank_all()
    local content = table.concat(lines, "\n")
    vim.fn.setreg("+", content)
    vim.fn.setreg("*", content)
    vim.fn.setreg('"', content)
    -- toast, not cmdline
    pcall(function()
      require("cargo_runner.ui.toast").info("Copied full output")
    end)
  end

  local map_opts = { buffer = buf, silent = true, nowait = true }
  vim.keymap.set("n", "q", close, map_opts)
  vim.keymap.set("n", "<Esc>", close, map_opts)
  vim.keymap.set("n", "y", yank_all, map_opts)
  vim.keymap.set("n", "c", yank_all, map_opts)
  vim.keymap.set("n", "Y", yank_all, map_opts)
  if opts.on_rerun then
    vim.keymap.set("n", "r", function()
      close()
      opts.on_rerun()
    end, map_opts)
  end

  -- Footer in the float title area via winbar-like last line (no cmdline echo)
  vim.bo[buf].modifiable = true
  local help = " q close · y/c copy · r re-run "
  vim.api.nvim_buf_set_lines(buf, -1, -1, false, { "", help })
  vim.bo[buf].modifiable = false
end

return M
