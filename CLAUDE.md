# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test

```bash
cargo build                  # build library + lser binary
cargo run --bin lser         # run the CLI tool
cargo test                   # unit tests + doc-tests
cargo test --lib             # lib tests only
```

## Architecture

Cross-platform serial port enumeration library. `src/lib.rs` defines the public API and re-exports the correct platform implementation via `#[cfg]`:

| Module | Platform | Enumeration Source |
|---|---|---|
| `src/win.rs` | Windows | SetupDi API ("Ports" class, `DIGCF_PRESENT`) |
| `src/linux.rs` | Linux | `/sys/class/tty` + `/proc/tty/drivers` |
| `src/macos.rs` | macOS | IOKit (`IOServiceGetMatchingServices`) |

All three modules export `pub fn get_serial_list() -> Vec<SerialInfo>`.

## Key Data Structures (`src/lib.rs`)

- **`SerialInfo`** — name, `valid`, vendor, product, driver (Linux only), usb_info
- **`UsbInfo`** — USB `vid` and `pid` as hex strings

## `valid` Field Behavior

| Platform | Filtering | `valid` |
|---|---|---|
| Windows | Only `COM`-prefixed ports returned | Always `true` |
| Linux | All TTYs with recognized serial prefix returned | Probe result: USB descriptors / of_node / PNP ID found → `true` |
| macOS | All IOKit serial BSD services returned | Always `true` |

## Linux Probe Functions (`src/linux.rs`)

- `probe_usb_serial` — walks up to 3 parent dirs looking for `manufacturer`, `product`, `idVendor`/`idProduct`
- `probe_acm_serial` — delegates to `probe_usb_serial` if subsystem is `usb`
- `probe_builtin_serial` — checks for `of_node` (device tree) or `id` (PNP)
- `get_serial_prefix` — parses `/proc/tty/drivers` with a hardcoded fallback map (`ttyS`, `ttyUSB`, `ttyPS`, `ttyACM`, `ttyAMA`, `ttymxc`, `ttyGS`)

## Git Commits

Write the commit message to a temp file first, then use `git commit -F` to avoid PowerShell/Bash escaping issues:

```bash
# Write message (no trailing blank lines)
Write .commit_msg with the exact content

# Commit
git commit -F .commit_msg

# Or amend
git commit --amend -F .commit_msg

# Clean up
rm .commit_msg
```

Do **not** use PowerShell here-strings (`@'...'@`) for commit messages — the `@` delimiters get included in the message.
