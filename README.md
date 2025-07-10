# `sway-gravity`

Bring your floating windows back down to earth.

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

In your Sway config, you'll need an `exec_always` call to start the daemon:

```
# start the daemon with your chosen initial state
exec_always sway-gravity -d --natural true --width '35%' --padding 12 bottom right
```

It's pretty safe to use `exec_always` here instead of just `exec`, because any
new-started daemon will automatically shutdown any pre-existing daemons that it
would overlap with (attached to the same sway session). Because of that, you
shouldn't ever have to worry about multiple daemons running at the same time.

I'd recommend using only one of either `--width` or `--height` in the initial
state. This is because if you only specify one, `sway-gravity` can automatically
modify the other to maintain the correct aspect ratio of the window. If both are
specified, that automatic behavior is disabled. All the examples here use
`--width` because that feels more intuitive to me instead of using the height.

Next, you need to bind your desired keymaps to call `sway-gravity` with the
appropriate options.

```
# order matters for these next 9 bindsyms:
# the keybinds for the corner directions have to come before the keybinds for
# the side directions, otherwise some of the corner keybinds won't work properly

bindsym $mod+Ctrl+$up+$right exec sway-gravity top right
bindsym $mod+Ctrl+$up+$left exec sway-gravity top left
bindsym $mod+Ctrl+$down+$right exec sway-gravity bottom right
bindsym $mod+Ctrl+$down+$left exec sway-gravity bottom left

bindsym $mod+Ctrl+$up exec sway-gravity top middle
bindsym $mod+Ctrl+$down exec sway-gravity bottom middle
bindsym $mod+Ctrl+$left exec sway-gravity middle left
bindsym $mod+Ctrl+$right exec sway-gravity middle right

bindsym $mod+Ctrl+space exec sway-gravity middle middle
```

```
# shrink or grow the window, or reset to an absolute size
# bindings like this roughly mimic the +/-/0 bindings in browsers

bindsym $mod+Ctrl+equal exec sway-gravity --width '+5%'
bindsym $mod+Ctrl+minus exec sway-gravity --width '-5%'
bindsym $mod+Ctrl+0 exec sway-gravity --width '35%'
```

`sway-gravity` uses the sway IPC to automatically figure out which window to
control. Only floating windows are considered; when only a single floating
window exists, that window is automatically chosen, even if it's not currently
focused. If more than 1 floating window exists, a window must be focused to be
controlled.

If `--natural true` is specified, the "natural" aspect ratio of the window will
be used. This is convenient for things like perfectly resizing a video PiP
window to the aspect ratio of the underlying video.

There are some additional customization options available, namely `--padding`
and `--width` and/or `--height`. `--padding` allows you to offset the window
placement from the edge of the workspace (similar to `gaps` in your sway
config). Padding must be specified in pixels.

`--width`/`--height` will automatically resize the window to the given
dimensions. If both are specified, it gets that exact size, otherwise it is
resized to the given dimension at the same ratio that the window previously had,
or the natural aspect ratio if `--natural true` was specified.
`--width`/`--height` can accept pixel or percentage values, where the percentage
measures the size in the parent container (usually, the entire output).

```
# resize the window to exactly 1/4 the width of the screen
sway-gravity --width '25%'
```

When specifying the initial state of the daemon, only absolute values (eg.
`300px`, `25%`) are allowed for these flags. However, when using `sway-gravity`
as a client, you can additionally specify relative values (eg. `+50px`, `-5%`)
which will modify the existing state of the window by those amounts.

If you want to manually kill any running daemons, you can use the `--shutdown`
flag.

# Miscellaneous

If you use the PiP mode in Firefox, you can use this line to automatically move
the PiP window to a specified position as soon as it is opened:

```
for_window [app_id="firefox" title="^Picture-in-Picture$"] floating enable, sticky enable, exec sway-gravity bottom right
```
