# cargo-runner Neovim plugin

Thin Neovim adapter for [cargo-runner](https://github.com/cargo-runner/cargo-runner).

**Full docs:** [docs/nvim.md](../../docs/nvim.md) · [status panel UX](../../docs/nvim-status-panel.md)

Requires: Neovim **0.9+**, `cargo-runner` on PATH (or `~/.cargo/bin`).

## Install

```bash
cargo runner nvim install
cargo runner vim install --follow-shell-alias   # if vim → nvim

# Custom config / data (not everyone uses ~/.config/nvim)
cargo runner nvim install --app-name nvim-lazy
cargo runner nvim install --config-dir ~/dotfiles/nvim --data-home ~/dotfiles/.local/share
cargo runner nvim install --pack-dir ~/dotfiles/nvim/pack/cargo-runner/start/cargo-runner
```

| Flag | Meaning |
|------|---------|
| `--config-dir DIR` | Your `init.lua` location (setup hints; install does not edit it) |
| `--data-home DIR` | Override `$XDG_DATA_HOME` for packpath |
| `--app-name NAME` | `$NVIM_APPNAME` (`nvim`, `nvim-lazy`, …) |
| `--pack-dir DIR` | Exact plugin pack directory |
| `--vim-dir DIR` | Classic Vim root |

Default pack: `~/.local/share/nvim/site/pack/cargo-runner/start/cargo-runner/`  
**No `init.lua` edit required** for default keymaps.

Optional setup in **your** config:

```lua
require("cargo_runner").setup({
  binary = vim.fn.expand("~/.cargo/bin/cargo-runner"),
})
```

## Keymaps

| Keys | Action |
|------|--------|
| `<leader>r` | Run (async, multi-job) |
| `<leader>R` | Override modal |
| `<leader>ro` | Peek output |
| `<leader>rj` | Jobs |
| `<leader>rk` | Kill job |

## Status panel

One fixed-width (50) top-right float: live jobs + success/fail notices. No overlapping toasts. Full logs via peek / error modal.

## Commands

`:CargoRunnerRun` · `:CargoRunnerOverride` · `:CargoRunnerPeek` · `:CargoRunnerJobs` · `:CargoRunnerKill` · `:CargoRunnerStatus`
