# Neovim status panel — UX design

Expert TUI plan for cargo-runner’s corner status UI.

**Install / custom paths:** [docs/nvim.md](nvim.md)

## Principle

**One non-focusable top-right float** owns all ephemeral status. Full logs live only in center modals. Reliability comes from **fixed width, slot caps, and atomic reflow** — not multi-float stacks.

## Information architecture

| Surface | Content | Lifetime |
|---------|---------|----------|
| Status panel | Active jobs + ephemeral notices | Until empty |
| Peek modal | Live/history stdout | User dismisses |
| Error float | Full fail log + copy/rerun | User dismisses |

Banned in the corner: raw cargo streams, multi-line errors, variable-width cards, second toast windows.

## Geometry

| Token | Value |
|-------|------:|
| Width | 50 cols |
| Max jobs visible | 3 |
| Max notices | 3 |
| Success / info TTL | 2500 ms |
| Warn TTL | 3500 ms |
| Error TTL | 5000 ms |
| Panel z-index | 200 |
| Modal z-index | 250 |
| Spinner interval | 120 ms |

Every line is `fit()`’d to the same display width (pad or truncate with `…`).

## Fail → re-run sequence

1. Job running → job line + spinner  
2. Fail → job line removed; `✗ Failed #N` notice in same panel  
3. Re-run → new job line **above** divider; fail notice remains until TTL  
4. Success → `✓ #M · t` notice prepended; panel closes when all empty  

## Implementation

- [`extensions/nvim/lua/cargo_runner/ui/panel.lua`](../extensions/nvim/lua/cargo_runner/ui/panel.lua) — sole float owner  
- Facades: `ui/hud.lua`, `ui/toast.lua` → panel  
- `jobs.lua` finish path: update status then **one** `push_notice` (no double render)  

## Anti-patterns

- Multiple floats in the same corner  
- Width based on message length  
- `vim.notify` for routine status  
- Focusable status panel  
- Unbounded job lists in the panel  
