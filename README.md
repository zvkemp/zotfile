# Zotfile

### multi-platform dev machine configuration management

This is a WIP (use at your peril). For personal use, I need a configuration manager that produces *roughly* the same config (but not *exactly* the same) on various flavors of Linux and MacOS.

Usage:

```shell
$ cargo run -- --target manjaro --module tmux
```

Prior to asking for confirmation, this will show a diff of the rendered template(s) and the current state of the target files.
