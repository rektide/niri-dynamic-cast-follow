# niri-dynamic-cast-follow

Automatically switches niri's dynamic screencast target for a set of windows you specify.

When you focus on a matching window, it becomes the dynamic cast target. No keybinds required.

## Motivation

This tool addresses the need for automatic dynamic casting as discussed in [YaLTeR/niri#3126](https://github.com/YaLTeR/niri/discussions/3126).

When giving live demos or streaming, manually switching cast targets with keybinds between windows (e.g., terminal and browser) is error-prone and easy to forget. This program lets you specify which windows should be cast, and automatically switches to whichever one is currently focused.

## Features

- **Window Tracking**: Monitors window focus changes in real-time via niri's IPC interface
- **Flexible Matching**: Match windows by app-id patterns, title patterns, or exact IDs
- **Regex Support**: Use regular expressions for flexible pattern matching
- **Automatic Switching**: Triggers `set-dynamic-cast-window` action when matching window is focused
- **Verbose Mode**: Optional debug output for troubleshooting

## Installation

### From Source

```bash
cargo build --release
sudo cp target/release/niri-dynamic-cast-follow /usr/local/bin/
```

## Usage

### Basic Examples

**Match by app-id:**

```bash
# Multiple -a flags (tracks windows matching ANY of these app-ids)
niri-dynamic-cast-follow -a "firefox" -a "google-chrome" -a "kitty"

# Track all Google Chrome windows
niri-dynamic-cast-follow --app-id "google-chrome"

# Use regex for more complex matching
niri-dynamic-cast-follow -a "^google-"
```

**Match by window title:**

```bash
# Multiple -t flags (tracks windows matching ANY of these titles)
niri-dynamic-cast-follow -t "YouTube" -t "Twitch" -t "Discord"

# Track windows with "YouTube" in title
niri-dynamic-cast-follow --title "YouTube"

# Track windows matching a pattern
niri-dynamic-cast-follow -t "^Discord - "
```

**Match by exact window ID:**

```bash
# Multiple -i flags (tracks specific window IDs)
niri-dynamic-cast-follow -i 42 -i 137

# Track a specific window by its ID
niri-dynamic-cast-follow --id 42
```

**Combine multiple matching criteria:**

```bash
# Mix short and long flags as preferred
niri-dynamic-cast-follow -a "firefox" -a "google-chrome" -t "YouTube" -i 42
```

**Verbose mode for debugging:**

```bash
# See what's happening under the hood
niri-dynamic-cast-follow --app-id "firefox" --verbose
```

### Finding Window Information

Use `niri msg windows` to find app-id patterns, title patterns, or window IDs for matching:

```bash
# List all windows to see app-ids and titles
niri msg windows

# Get focused window info
niri msg focused-window

# View JSON output for easier parsing
niri msg --json windows | jq
```

## How It Works

1. **Connection**: Connects to niri's IPC socket (reads `$NIRI_SOCKET` environment variable)
2. **Event Stream**: Requests continuous event stream from niri
3. **Window Tracking**: Listens to `WindowOpenedOrChanged` events to maintain window metadata (ID, title, app-id)
4. **Focus Monitoring**: Listens to `WindowFocusChanged` events to detect when window focus changes
5. **Pattern Matching**: When focus changes, checks if the new window matches provided criteria
6. **Action Triggering**: If window matches, sends `SetDynamicCastWindow` action to set it as screencast target
7. **Dual Socket**: Uses separate IPC socket for sending actions (recommended by niri docs)

## Requirements

- niri Wayland compositor running
- `$NIRI_SOCKET` environment variable set (automatically set by niri)
- Rust toolchain for building from source

## Troubleshooting

**"Error: at least one matching criterion must be provided"**

You must specify at least one matching option (`--app-id`, `--title`, or `--id`).

**"Failed to start event stream"**

Make sure niri is running and the `$NIRI_SOCKET` environment variable is set correctly.

**Window doesn't match when expected**

Use `--verbose` flag to see:
- Which windows are being tracked
- What the current focused window is
- Whether pattern matching is working

Check your regex patterns are valid:
```bash
# Test regex patterns separately
echo "google-chrome-stable" | grep "^google-"
```

## Use Cases

- **Streaming**: Automatically switch screencast to your browser or streaming app when you focus it
- **Meetings**: Focus your video conferencing app to immediately start sharing
- **Presentations**: Switch to your presentation window when it becomes focused
- **Multi-monitor setups**: Control which window is being cast to another display

## License

MIT
