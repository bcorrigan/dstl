<h1 align="center">dstl (Sway Fork)</h1>

<p align="center"><b>Dustin's Simple TUI Launcher</b> - A fast, keyboard-driven application launcher for the terminal with fuzzy search and extensive theming support. This is a fork specialized for better Sway integration and a stateless, Emacs-friendly UX.</p>

## Fork Features

- 🏢 **Sway Integration** - seamless execution via `swayexec` IPC (avoids process hierarchy issues) and smart fullscreen handling (un-fullscreens for launch, restores if cancelled)
- ⌨️ **Stateless UX** - Typing always goes to search, arrow keys always navigate lists. No modes to switch focus.
- 🎹 **Emacs Keybindings** - GNU Readline style shortcuts (`Ctrl-a`, `Ctrl-e`, `Ctrl-k`, `Ctrl-u`, etc.)
- 🎨 **Enhanced Visuals** - Clear distinction between focused and unfocused lists in dual-pane mode via themable colors
- 🚀 **Direct Execution** - Can print selection to stdout or execute directly

## Features (Inherited)

- 🚀 **Fast fuzzy search** - Quickly find applications as you type
- 🎨 **Highly customizable** - Extensive theming with hex color support
- 📱 **Dual view modes** - Switch between single-pane and dual-pane (category + apps) layouts
- 📋 **Recent apps tracking** - Quick access to frequently used applications
- 🎯 **Smart cursor** - Full cursor control with blinking support
- 🔧 **Flexible configuration** - Uses `.rune` config format with import/gather support

## Installation

### From Source

```bash
git clone https://github.com/saltnpepper97/dstl
cd dstl
cargo build --release
sudo cp target/release/dstl /usr/local/bin/
```

## Configuration

dstl looks for configuration in the following locations (in order):
1. `~/.config/dstl/dstl.rune`
2. `/usr/share/doc/dstl/dstl.rune`

### Basic Configuration Example

```rune
dstl:
    # Display mode
    dmenu = false
    startup_mode = "dual"  # or "single"
    search_position = "top"  # or "bottom"
    
    # Sway Integration
    sway = true  # Enable Sway IPC integration
    print_selection = false # Print selected command to stdout instead of launching
    
    # Terminal emulator for terminal apps
    terminal = "foot"
    
    # Recent apps settings
    max_recent_apps = 15
    recent_first = false
    
    # Theme configuration
    theme:
        border = "#ffffff"
        focus = "#00ff00"
        unfocused = "#808080" # Color for selection in non-active pane
        highlight = "#0000ff"
        cursor_color = "#00ff00"  # defaults to focus color
        cursor_shape = "block"  # "block", "underline", or "pipe"
        cursor_blink_interval = 500  # milliseconds, 0 to disable
        border_style = "rounded"  # "plain", "rounded", "thick", "double"
        highlight_type = "background"  # or "foreground"
    end
end
```

### Sway Support

When `sway = true` (or `--sway` flag is used):
1. Upon launch, checks if the current window is fullscreen.
2. If fullscreen, disables it to show the launcher.
3. If an app is launched, it executes via Sway IPC (`exec <cmd>`), preventing the new app from being a child of the launcher/terminal.
4. If cancelled (`Esc` or `Ctrl-g`), restores the original window's fullscreen state.

### Theme System with Gather

dstl supports importing themes using the `gather` statement:

```rune
# Import a theme file
gather "~/.config/dstl/themes/dracula.rune" as theme

dstl:
    terminal = "alacritty"
    # Theme colors will be loaded from the gathered file
    theme:
      cursor_shape = "block"
      cursor_blink_interval = 500
      border_style = "plain"
      highlight_type = "background"
    end
end
```

**Theme Priority:**
1. Aliased gather imports (e.g., `gather "theme.rune" as mytheme`)
2. Top-level theme in main config or non-aliased gather
3. Document named "theme"
4. Built-in defaults

### Color Format

Colors support multiple hex formats:
- `#RGB` - 3-digit hex (e.g., `#fff`)
- `#RRGGBB` - 6-digit hex (e.g., `#ffffff`)

## Usage

### Launching

```bash
# Launch directly
dstl

# Launch with Sway integration override
dstl --sway
```

# Launch from config (hyprland/sway example)
```
bindsym $mod+d exec foot --app-id dstl -e dstl --sway
```

### Keyboard Shortcuts

#### Global
- `Tab` / `Ctrl-t` - Toggle between single-pane and dual-pane mode
- `Ctrl-g` / `Esc` - Quit without launching
- `Enter` - Launch selected application

#### Navigation (Always Active)
- `↓` - Move down in list
- `↑` - Move up in list
- `←` - Move left (to Categories pane in dual-mode)
- `→` - Move right (to Apps pane in dual-mode)

#### Text Editing (Emacs Style)
- `Type` - Always goes to search bar
- `Ctrl-a` / `Home` - Jump to start
- `Ctrl-e` / `End` - Jump to end
- `Ctrl-b` - Back one char
- `Ctrl-f` - Forward one char
- `Ctrl-w` - Delete previous word
- `Ctrl-u` - Delete to start of line
- `Ctrl-k` - Delete to end of line
- `Ctrl-d` / `Delete` - Delete next char
- `Ctrl-h` / `Backspace` - Delete previous char

## View Modes

### Single-Pane Mode
Shows all applications in one list with fuzzy search filtering across all categories.

### Dual-Pane Mode
- **Left pane**: Categories with app count
- **Right pane**: Applications in selected category
- Search filters both panes simultaneously
- Special "Recent" category shows recently launched apps
- The currently active list (navigable by arrow keys) is highlighted with the `focus` color. The inactive list selection uses the `unfocused` color.

### Advanced Configuration

### Key Settings Explained

- **`dmenu`**: Enable dmenu-like behavior (boolean)
- **`sway`**: Enable Sway IPC integration (boolean)
- **`print_selection`**: Print command to stdout instead of executing (boolean)
- **`search_position`**: Place search bar at `"top"` or `"bottom"`
- **`startup_mode`**: Start in `"single"` or `"dual"` pane mode
- **`timeout`**: Auto-close timeout in milliseconds (0 to disable)
- **`max_recent_apps`**: Maximum number of recent apps to track
- **`recent_first`**: Show recent apps category first
- **`terminal`**: The command used to wrap CLI-based applications.
  - If a single word (e.g., `"alacritty"`), `dstl` automatically appends `-e` before the application command.
  - If multiple words (e.g., `"wezterm start"` or `"foot --app-id launcher"`), `dstl` appends the application command directly. This allows using specific terminal subcommands or existing processes.
  - **Example**: `terminal = "wezterm start"` results in `wezterm start helix` being executed.

### Cursor Customization

- **`cursor_shape`**: Visual style of the cursor
  - `"block"` - Solid block (█)
  - `"underline"` - Underscore (_)
  - `"pipe"` - Vertical bar (|)
- **`cursor_blink_interval`**: Blink speed in milliseconds (0 = no blinking)
- **`cursor_color`**: Hex color for cursor (defaults to focus color)

### Border Styles

- `"plain"` - Simple lines
- `"rounded"` - Rounded corners
- `"thick"` - Bold lines
- `"double"` - Double-line borders

### Highlight Types

- `"background"` - Highlight with background color (selected text is black)
- `"foreground"` - Highlight with foreground color only

## Desktop Entry Detection

dstl automatically scans for `.desktop` files in standard XDG directories to populate the application list. Categories are extracted from desktop entries.

## Tips

- Use fuzzy search to quickly find apps by typing partial names
- The search algorithm scores matches, showing best matches first
- Recent apps are persistent across sessions
- Cursor stays visible and solid while typing or moving
- Navigate between search and lists seamlessly with arrow keys

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
