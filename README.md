# `sway-gravity`

Autoplacement for floating windows in sway.

# About

I almost never use floating windows in Sway, but the single exception to that is
when I have something playing in Picture-in-Picture ("PiP") mode, and want to
set it to be floating + sticky so that it doesn't take up space from other
windows.

`sway-gravity` is built mainly with this use case in mind. You can snap any
floating window to one of 9 spots on the current workspace, which are
any combination of a vertical alignment (`top`, `middle`, or `bottom`) and a
horizontal alignment (`left`, `middle`, or `right`).

There are also a few options for controlling the window size and padding in the
placement calculations.

# Installation

The easiest way to install `sway-gravity` is via Cargo:

```bash
cargo install --git https://github.com/loganswartz/sway-gravity
```

Alternatively, clone the repo locally and do `cargo install --path .`.

# Usage

See `sway-gravity -h` for a full list of options.

In your Sway config, you simply need to bind your desired keymaps to call
`sway-gravity` with the appropriate options. Here's a simple example:

```
# order matters here:
# the keybinds for the corner directions have to come before the keybinds for
# the side directions, otherwise some of the corner keybinds won't work properly

bindsym Mod1+Ctrl+Up+Right exec sway-gravity top right
bindsym Mod1+Ctrl+Up+Left exec sway-gravity top left
bindsym Mod1+Ctrl+Down+Right exec sway-gravity bottom right
bindsym Mod1+Ctrl+Down+Left exec sway-gravity bottom left

bindsym Mod1+Ctrl+Up exec sway-gravity top middle
bindsym Mod1+Ctrl+Down exec sway-gravity bottom middle
bindsym Mod1+Ctrl+Left exec sway-gravity middle left
bindsym Mod1+Ctrl+Right exec sway-gravity middle right

bindsym Mod1+Ctrl+space exec sway-gravity middle middle
```

`sway-gravity` uses the sway IPC to automatically figure out which window to
control. Only floating windows are considered; when only a single floating
window exists, that window is automatically chosen, even if it's not currently
focused. If more than 1 floating window exists, a window has to be focused
before it can be controlled.

There are some additional customization options available, namely `--padding`
and `--width` and/or `--height`. `--padding` allows you to offset the window
placement from the edge of the workspace (similar to `gaps` in your sway
config).

`--width`/`--height` will automatically resize the window to the given
dimensions. If both are specified, it gets that exact size, otherwise it is
resized to the given dimension at the same ratio that the window previously had.

# Miscellaneous

If you use the PiP mode in Firefox, you can use this line to automatically move
the PiP window to a specified position as soon as it is opened:

```
for_window [app_id="firefox" title="^Picture-in-Picture$"] floating enable, sticky enable, exec sway-gravity bottom right
```
