<div align="center">

<img src="docs/icon.png" width="96" alt="">

# Commands

**Your own ⌘K commands that type a line into the current tab.**

[![Haunt plugin](https://img.shields.io/badge/Haunt-plugin-D97757)](https://github.com/highskore/haunt-hackathon)
![Plugin API 0.5](https://img.shields.io/badge/plugin%20API-0.5-111)
![WASM component](https://img.shields.io/badge/WASM-component-654ff0?logo=webassembly&logoColor=white)
![MIT licence](https://img.shields.io/badge/licence-MIT-6b7280)

</div>

Your own ⌘K commands for Haunt, as a first-party plugin (`dev.haunt.commands`). Each command types a line of text into the current tab, and can press Return after it.

## Commands

Add them to `~/.haunt/config.toml`:

```toml
[[plugins."dev.haunt.commands".commands]]
id = "deploy"
name = "Deploy staging"
text = "make deploy ENV=staging"
enter = true

[[plugins."dev.haunt.commands".commands]]
id = "status"
name = "Git status"
text = "git status"
```

| key | | |
|---|---|---|
| `id` | required | 1 to 32 lowercase letters, digits and `-`; unique |
| `name` | required | what the palette shows, up to 64 characters |
| `text` | required | typed into the tab as it is |
| `enter` | `false` | press Return after the text |

A command is skipped and logged if a field is missing, unknown or bad, including a name or text with a control or invisible character, or text over 4096 bytes; the others still load. At most 64 commands. Saving the file applies the change, removed commands included. If config.toml stops parsing, the commands you had stay until it parses again.

Bind a key to a command in `[keys]` by its id:

```toml
[keys]
"dev.haunt.commands/deploy" = "cmd+shift+d"
```

## What Haunt allows

The plugin asks for one capability, `pty_type`: "can type text into the current tab when you run one of its commands". It can't spawn processes or read the terminal. Haunt types only when you pick a command in the palette or press its key, and only into the tab that was focused then, while its terminal is on screen. It types nothing if:

- the agents view or the review screen is up, or
- you switched tabs before it was written, or
- that tab has a permission prompt up, or one that may still be on screen, or
- the text has a newline, an escape (so no bracketed paste end) or another control or invisible character. A tab is fine.

The text goes in as a paste, and Return, when `enter = true`, as a separate keystroke after it.

## Build

```sh
cargo build --release --target wasm32-wasip2
cargo test
haunt plugin install .
```

It needs Haunt with `haunt:plugin@0.5` (`api = "0.5"`) and plugin settings from config.toml. `wit/haunt-plugin.wit` is a copy of Haunt's, pinned at that version.

## Licence

MIT, see `LICENSE`.
