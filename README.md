# Shark File Manager

A fast, lightweight, modern Linux file manager written in Rust using GTK4. Built with performance, responsiveness, and clean Adwaita design aesthetics at its core.

![Shark File Manager](/home/sahilcodex/.gemini/antigravity-cli/brain/63edc105-d82c-4fba-bf21-43a0e7393604/shark_window_focused.png)

---

## Key Highlights

- **Resource-conscious design**: Incremental scanning and virtualized views; startup time, RAM use, and release size still need reproducible benchmarks.
- **System appearance**: Follows the GTK light/dark palette without forcing a dark theme.
- **Asynchronous, Non-Blocking Engine**: Directory enumeration, recursive file searching, file copying, moving, and trash operations run on background threads with real-time UI batch streaming and cancellation.
- **Virtualized Views**: High-performance `GtkGridView` and `GtkListView` using `gio::ListStore` virtualization capable of smoothly rendering folders containing tens of thousands of files.
- **Full Standard File Operations**: Copy, Cut, Paste, Rename, Move to Trash, Permanent Delete, New Folder, New File, File Properties, Open With Default App.
- **Interactive Navigation**: Clickable breadcrumb path bar with instant toggle to direct editable path entry (`Ctrl+L`), Back/Forward history stack, and Parent navigation.
- **Places & Bookmarks Sidebar**: Instant access to standard XDG user directories (Home, Documents, Downloads, Music, Pictures, Videos, Desktop, Trash, Root), removable drives, and custom user bookmarks with right-click removal.
- **Search palette**: `Ctrl+F` searches the current folder and its subfolders, streams matching filenames and parent paths, and supports keyboard opening and cancellation. Hidden results follow `Ctrl+H`.
- **Flexible Sorting**: Sort by Name, Size, Modified Date, or Type with directories grouped first and toggleable ascending/descending order.
- **Filesystem Monitoring**: Inotify/GIO change tracking automatically refreshes open directories upon external filesystem changes.

---

See [feature status and remaining work](docs/FEATURE_STATUS.md).

## Architecture Overview

```
shark/
├── Cargo.toml                  # Cargo manifest (glib, gio, gtk4, serde, serde_json)
├── assets/
│   ├── icons/                  # Application icons
│   └── styles/
│       └── style.css           # Adwaita-inspired GTK4 CSS styling
├── src/
│   ├── lib.rs                  # Library entry point & exports
│   ├── main.rs                 # Binary entry point
│   ├── app.rs                  # SharkApp initialization & GtkApplication lifecycle
│   ├── config.rs               # JSON configuration persistence (~/.config/shark/config.json)
│   ├── filesystem/
│   │   ├── error.rs            # Filesystem error definitions
│   │   ├── monitor.rs          # Directory change monitoring with GIO FileMonitor
│   │   ├── operations.rs       # Async copy, move, trash, delete, rename, folder/file creation
│   │   ├── scanner.rs          # Incremental async directory scanner & recursive search
│   │   └── mod.rs              # Filesystem module exports
│   ├── gui/
│   │   ├── clipboard.rs        # Internal file clipboard state (copy / cut / paste)
│   │   ├── dialogs.rs          # Modal dialogs (rename, new folder/file, delete, conflict, about)
│   │   ├── file_view.rs        # Virtualized Grid/List views, sorting, filtering, and selection
│   │   ├── properties_dialog.rs# Detailed file/directory properties modal
│   │   ├── sidebar.rs          # Places, Bookmarks, and Drives sidebar
│   │   ├── toolbar.rs          # HeaderBar, breadcrumbs, location entry, search, and action menus
│   │   ├── window.rs           # SharkWindow coordinating views, shortcuts, and status bar
│   │   └── mod.rs              # GUI module exports
│   ├── models/
│   │   ├── file_entry.rs       # FileEntry metadata model (permissions, MIME, Adwaita icon resolver)
│   │   ├── file_object.rs      # GObject wrapper for FileEntry for GTK4 ListStore virtualization
│   │   └── mod.rs              # Models module exports
│   ├── navigation/
│   │   ├── history.rs          # Back/Forward/Parent navigation history stack
│   │   └── mod.rs              # Navigation module exports
│   └── utils/
│       └── mod.rs              # Formatting utilities (file size, dates, permissions, free space)
└── tests/
    ├── config_tests.rs         # Configuration serialization & bookmarks tests
    ├── model_tests.rs          # Icon determination, permissions, size formatting tests
    ├── navigation_tests.rs     # Navigation history stack tests
    ├── operations_tests.rs     # Async file operations (copy, move, delete, create, rename)
    └── scanner_tests.rs        # Directory scanner & recursive search tests
```

---

## Keyboard Shortcuts

| Shortcut | Action |
|:---|:---|
| `Ctrl+L` | Toggle between breadcrumbs and direct location entry bar |
| `Ctrl+F` | Open recursive file search palette |
| `Ctrl+H` | Toggle hidden files and folders |
| `Ctrl+C` | Copy selected file(s) / folder(s) |
| `Ctrl+X` | Cut selected file(s) / folder(s) |
| `Ctrl+V` | Paste clipboard items into current directory |
| `Ctrl+A` | Select all items in the current directory |
| `Ctrl+Shift+N` | Create a new folder |
| `F2` | Rename selected item |
| `Delete` | Move selected item(s) to system Trash |
| `Shift+Delete` | Permanently delete selected item(s) (with confirmation dialog) |
| `Alt+Left` / `Backspace` | Navigate back in history |
| `Alt+Right` | Navigate forward in history |
| `Alt+Up` | Navigate to parent directory |
| `Ctrl+R` / `F5` | Refresh current directory |
| `Enter` | Open directory or launch file with default system application |

---

## Building and Running

### Prerequisites

- Rust 1.70+ (`cargo`, `rustc`)
- GTK4 and GIO development libraries:
  - **Arch Linux:** `sudo pacman -S gtk4 glib2 pkgconf`
  - **Fedora:** `sudo dnf install gtk4-devel glib2-devel pkg-config`
  - **Ubuntu / Debian:** `sudo apt install libgtk-4-dev libglib2.0-dev pkg-config`

### Build & Run

```bash
# Run in debug mode
cargo run

# Build optimized release binary
cargo build --release

# Run release binary
./target/release/shark
```

### Running Test Suite

```bash
cargo test
```

All 17 integration and unit tests execute in ~0.04s.

---

## License

This project is licensed under the MIT License.
