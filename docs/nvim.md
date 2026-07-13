# Neovim / Vim adapter

Thin Neovim-first plugin for cargo-runner: run at cursor, override modal, multi-job background execution, and a **single fixed-width status panel**.

| | |
|--|--|
| Sources | [`extensions/nvim/`](../extensions/nvim/) |
| Status UX | [`docs/nvim-status-panel.md`](nvim-status-panel.md) |
| IDE JSON | [`docs/ide-protocol.md`](ide-protocol.md) |

Requires: Neovim **0.9+** (`vim.system`), `cargo-runner` on PATH (or `~/.cargo/bin`).

---

## Install (CLI)

```bash
cargo runner nvim install
cargo runner nvim status
cargo runner nvim uninstall

# shell alias vim → nvim
cargo runner vim install --follow-shell-alias
```

Default packpath (no `init.lua` edit required):

```text
~/.local/share/nvim/site/pack/cargo-runner/start/cargo-runner/
```

### Custom config / data locations

Not everyone uses `~/.config/nvim` + `~/.local/share/nvim`. The CLI accepts:

| Flag | Purpose |
|------|---------|
| `--config-dir DIR` | Where your `init.lua` lives (hints only; pack install does not edit it) |
| `--data-home DIR` | Override `$XDG_DATA_HOME` for packpath (`DIR/{app}/site/pack/…`) |
| `--app-name NAME` | `$NVIM_APPNAME` equivalent (`nvim`, `nvim-lazy`, `astronvim`, …) |
| `--pack-dir DIR` | Exact plugin directory (wins over data-home / app-name) |
| `--vim-dir DIR` | Classic Vim root (default `~/.vim`) |

**Examples:**

```bash
# LazyVim / custom app name (matches Neovim stdpath with NVIM_APPNAME=nvim-lazy)
cargo runner nvim install --app-name nvim-lazy
# → ~/.local/share/nvim-lazy/site/pack/cargo-runner/start/cargo-runner

# Dotfiles: config and data split
cargo runner nvim install \
  --config-dir ~/dotfiles/nvim \
  --data-home ~/dotfiles/.local/share \
  --app-name nvim

# Exact pack path (git submodule / manual pack layout)
cargo runner nvim install \
  --pack-dir ~/dotfiles/nvim/pack/cargo-runner/start/cargo-runner

# Status / uninstall must use the same path flags
cargo runner nvim status --app-name nvim-lazy
cargo runner nvim uninstall --app-name nvim-lazy
```

**Precedence for pack location:**

1. `--pack-dir`
2. `{data-home}/{app-name}/site/pack/cargo-runner/start/cargo-runner`
   - `data-home`: `--data-home` → `$XDG_DATA_HOME` → `~/.local/share`
   - `app-name`: `--app-name` → `$NVIM_APPNAME` → `nvim`
3. Vim: `{vim-dir}/pack/cargo-runner/start/cargo-runner`

`$NVIM_APPNAME` is honored automatically if you export it and omit `--app-name`.

### Optional `init.lua` setup

Packpath loads the plugin without config. For options, add to **your** config dir:

```lua
-- e.g. ~/.config/nvim/init.lua  or  --config-dir path
require("cargo_runner").setup({
  binary = vim.fn.expand("~/.cargo/bin/cargo-runner"), -- if not on PATH
  map_super = "auto",
  leader_run = "<leader>r",
  leader_override = "<leader>R",
  leader_peek = "<leader>ro",
  leader_jobs = "<leader>rj",
  leader_kill = "<leader>rk",
})
```

Print snippets without installing:

```bash
cargo runner nvim install --method print --config-dir ~/.config/nvim
```

---

## Keymaps

| Keys | Action |
|------|--------|
| `<leader>r` | Run at cursor (**async**, multi-job) |
| `<leader>R` | Override modal |
| `<leader>ro` | Peek live/history stdout |
| `<leader>rj` | Job picker |
| `<leader>rk` | Kill focused job |
| `<D-r>` / `<D-S-r>` | Super/Cmd when GUI / Neovide / `map_super=true` |

Terminal Neovim usually does **not** receive Cmd — use leader maps.

---

## Status panel (UX)

One top-right float, fixed width **50**, no overlapping toasts:

- Live jobs (max 3) + spinner
- Notices (success / fail, max 3, TTL)
- Fail → re-run keeps the fail notice under the new job in the **same** panel

Full logs: center modals only (peek / error float). See [nvim-status-panel.md](nvim-status-panel.md).

---

## Commands

- `:CargoRunnerRun` · `:CargoRunnerOverride` · `:CargoRunnerDryRun`
- `:CargoRunnerPeek [id]` · `:CargoRunnerJobs` · `:CargoRunnerKill [id|all]`
- `:CargoRunnerShowOutput` · `:CargoRunnerCopyLastOutput` · `:CargoRunnerStatus`
