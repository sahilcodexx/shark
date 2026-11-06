# File manager feature status

The current application uses Rust, GTK4 and GIO. GPUI migration is still outstanding.
This list describes the code currently present, rather than promising feature parity
with established file managers.

| Area | Current implementation | Remaining work |
| --- | --- | --- |
| Appearance | System GTK palette, light/dark theme changes, keyboard focus | Verify across GNOME, KDE and XFCE |
| Browsing | Virtualized grid/list, breadcrumbs, editable paths, history | Tabs, split view, saved scroll positions |
| File operations | Copy, cut, paste, rename, trash, permanent delete, create file/folder | Undo, drag and drop, broader conflict/recovery testing |
| Search | Recursive search palette, streaming results, parent paths, keyboard opening and cancellation | Optional indexing |
| Sidebar | Places, custom bookmarks, mounted local drives with change notifications | Mount/eject controls and remote locations |
| Metadata | File sizes, list metadata and properties dialog | Lazy thumbnails and preview pane |
| Archives | Open with the desktop default application | Integrated create/extract |
| Performance | Incremental scanning and virtualized views | Reproducible RAM/startup and large-directory benchmarks |

The next feature priorities from the requirements are tabs, thumbnails, then archive
and network support. Each needs its own working implementation and checks; a menu
entry alone does not constitute a feature.

## UI verification

Run the normal suite with `cargo test`. With a desktop display available, run:

```sh
cargo test --test ui_tests -- --ignored
```

The UI test checks CSS, selection, switching views and changing between the GTK
light and dark palettes. To save rendered screenshots, set `SHARK_UI_SNAPSHOTS`
to an output directory. Use an isolated `XDG_CONFIG_HOME` when running the tests,
because the existing configuration tests persist bookmarks.
