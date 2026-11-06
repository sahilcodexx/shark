ARCHITECTURE.md

1. Architecture Overview
   1.1 Introduction

This document defines the internal technical architecture of the lightweight Linux file manager.

The application is built using:

Rust as the primary programming language.
GTK4 for the graphical user interface.
GLib for the application event loop and utilities.
GIO for filesystem integration and asynchronous operations.
SQLite as an optional persistent search index.

The architecture must prioritize:

Low memory consumption.
Fast filesystem operations.
Responsive UI.
Efficient resource management.
Modular and maintainable code.
Reliable Linux desktop integration.
1.2 Architectural Philosophy

The application should follow a modular, event-driven architecture.

The GUI must remain independent of filesystem operations.

Expensive operations must execute asynchronously or on dedicated background workers.

The application should avoid unnecessary abstraction layers, excessive dependencies and complex infrastructure.

Prefer simple, efficient solutions.

1. High-Level Architecture

The application is divided into the following major components:

text
APPLICATION
|
+-----------+-----------+
| |
GUI LAYER APP CONTROLLER
| |
+-----------+-----------+
|
EVENT / MESSAGE
LAYER
|
+--------------+--------------+
| | |
FILESYSTEM SEARCH THUMBNAIL
ENGINE ENGINE ENGINE
| | |
+--------------+--------------+
|
CACHE MANAGER
|
+-----------+-----------+
| |
GIO / GLib SQLite
|
Linux Filesystem

Each component must have a clearly defined responsibility.

Components should communicate through explicit interfaces and messages rather than accessing unrelated internal state directly.

1. GUI Architecture
   3.1 Technology
   GTK4
   gtk-rs
   GLib
   GIO

The GUI is responsible only for presentation and user interaction.

It must not contain filesystem scanning, indexing or file operation implementations.

3.2 GUI Components
text
gui/
├── mod.rs
├── window.rs
├── header_bar.rs
├── sidebar.rs
├── toolbar.rs
├── breadcrumb.rs
├── file_view.rs
├── file_item.rs
├── context_menu.rs
├── search_bar.rs
├── tab_manager.rs
├── properties_dialog.rs
└── preferences.rs

Responsibilities

window.rs

Main application window.
Main layout.
Component initialization.

sidebar.rs

Home directory.
Standard user directories.
Bookmarks.
Mounted volumes.
Trash.

header_bar.rs

Navigation controls.
Search button.
View controls.
Main actions.

breadcrumb.rs

Current directory path.
Parent navigation.
Direct location navigation.

file_view.rs

Grid and list presentation.
Sorting.
Selection.
Scrolling.

file_item.rs

File icon.
Filename.
File metadata presentation.

context_menu.rs

File context menus.
Directory context menus.
Common actions.

tab_manager.rs

Tab creation.
Tab switching.
Tab closing.
Independent navigation history.
GUI Rules
Never perform blocking filesystem operations in GUI callbacks.
Never create a widget for every entry in a large directory.
Use GTK's virtualized list components.
Avoid unnecessary widget reconstruction.
Update only affected UI elements.
Keep rendering and filesystem operations independent. 4. Application Controller

The application controller coordinates the GUI and backend.

Responsibilities:

Receive user actions.
Dispatch filesystem requests.
Manage application state.
Receive backend results.
Update the relevant UI components.
Coordinate tabs and navigation.

Suggested structure:

text
app/
├── mod.rs
├── controller.rs
├── state.rs
├── events.rs
└── messages.rs

The controller must not implement filesystem operations directly.

It should delegate work to the appropriate backend component.

1. Filesystem Engine

The filesystem engine is one of the most important parts of the application.

It handles directory access, metadata retrieval, filesystem monitoring and file operations.

Suggested structure:

text
filesystem/
├── mod.rs
├── scanner.rs
├── entry.rs
├── metadata.rs
├── operations.rs
├── monitor.rs
├── permissions.rs
└── error.rs

5.1 Directory Scanner

Responsibilities:

Read directory entries.
Return entries progressively.
Avoid unnecessary metadata retrieval.
Handle permission errors.
Handle unavailable directories.
Support cancellation.
Handle large directories efficiently.
Scanning Strategy

Use incremental scanning rather than requiring the entire directory to be processed before showing results.

The scanner should return batches of entries.

Example:

text
Directory Request
|
v
Start Scanner
|
v
Read Directory Entries
|
v
Produce Entry Batch
|
v
Send Batch to Controller
|
v
Update File View
|
v
Continue Scanning

Scanning must be cancellable when the user navigates to another directory.

5.2 Metadata Manager

Metadata should be retrieved only when required.

Basic information:

Filename.
File type.
File size.
Modification time.
File permissions.
Directory status.

Avoid retrieving expensive metadata for every entry if it is not needed for the current view.

5.3 Filesystem Operations

Responsibilities:

Copy.
Move.
Rename.
Delete.
Create directory.
Create file.
Trash operations.

Operations must run independently of the GUI.

Each operation should support:

Progress reporting.
Cancellation where safe.
Error reporting.
Destination conflict handling.
Operation completion notification.

Use GIO where it provides reliable Linux desktop integration, particularly for trash, mounts and remote filesystems.

1. Search Engine

Suggested structure:

text
search/
├── mod.rs
├── query.rs
├── filter.rs
├── recursive.rs
├── index.rs
└── database.rs

6.1 Search Modes
Mode A: Current Directory Search
Filter currently loaded entries.
Support partial matching.
Support case-insensitive matching.
Avoid database access.
Return results immediately where possible.
Mode B: Recursive Search
Search directory trees in background workers.
Support cancellation.
Handle permission failures.
Avoid blocking navigation.
Mode C: Indexed Search

Optional functionality.

SQLite FTS5 may be used for persistent indexing.

Indexing must be configurable and disabled by default initially.

The application must remain fully usable without an index database.

1. Thumbnail Engine

Suggested structure:

text
thumbnails/
├── mod.rs
├── generator.rs
├── cache.rs
├── scheduler.rs
└── formats.rs

7.1 Responsibilities
Generate image thumbnails.
Schedule thumbnail requests.
Cache generated thumbnails.
Avoid duplicate requests.
Cancel obsolete requests.
Prioritize visible files.
7.2 Scheduling Strategy

Thumbnail generation must be lazy.

Priority order:

Currently visible files.
Files about to become visible.
Other requested files.

Limit concurrent generation to avoid excessive CPU and memory usage.

Thumbnail generation must never block the GUI.

Use the freedesktop thumbnail specification where appropriate for Linux desktop interoperability.

1. Cache Architecture

Suggested structure:

text
cache/
├── mod.rs
├── directory_cache.rs
├── metadata_cache.rs
├── thumbnail_cache.rs
└── policy.rs

The cache system must improve navigation without consuming unlimited RAM.

8.1 Directory Cache

May store:

Recently visited directory entries.
Sorting information.
Directory timestamps.
Relevant navigation state.
8.2 Metadata Cache

May store recently retrieved metadata.

8.3 Thumbnail Cache

Store generated thumbnails using a disk-backed cache where appropriate.

8.4 Cache Rules
All memory caches must have limits.
Old entries must be evicted.
Stale data must be invalidated.
Cache invalidation must account for filesystem changes.
Users should be able to clear caches.
Avoid caching information that is cheap to retrieve.

Do not assume cached filesystem data is always current.

1. Navigation Architecture

Suggested structure:

text
navigation/
├── mod.rs
├── history.rs
├── location.rs
├── breadcrumbs.rs
└── tabs.rs

Each tab should maintain independent navigation state.

Navigation state may contain:

Current location.
Back history.
Forward history.
Scroll position.
Selected items.
View mode.

Navigating to a directory must cancel obsolete scanning requests where appropriate.

 1. Event and Communication Architecture

Components must communicate through explicit events and messages.

Example event types:

text
ApplicationEvent
├── NavigateTo
├── NavigateBack
├── NavigateForward
├── RefreshDirectory
├── SearchRequested
├── FileOperationRequested
├── ThumbnailRequested
└── TabChanged

Example backend messages:

text
BackendMessage
├── DirectoryBatchReceived
├── DirectoryScanCompleted
├── DirectoryScanFailed
├── FileOperationProgress
├── FileOperationCompleted
├── SearchResultsReceived
└── ThumbnailReady

Events must be processed safely on the appropriate execution context.

GTK objects must only be accessed from the GTK main thread.

Background workers should communicate with the GUI through GLib-compatible channels, asynchronous callbacks or other appropriate message-passing mechanisms.

 1. Concurrency Architecture

The application must use background execution carefully.

11.1 Main Thread

Responsible for:

GTK event processing.
Rendering.
User interactions.
UI updates.
Lightweight state changes.
11.2 Background Execution

Responsible for:

Directory scanning.
Recursive search.
Metadata retrieval.
Thumbnail generation.
File operations.
Optional indexing.
11.3 Concurrency Rules
Never block the GTK main thread.
Avoid unlimited thread creation.
Use bounded task queues.
Support task cancellation.
Avoid unnecessary parallelism.
Avoid duplicate background tasks.
Prioritize interactive work over optional background work.

Do not introduce Tokio merely for the sake of using an async runtime. Prefer GLib/GIO asynchronous facilities for GTK-integrated operations and add other runtimes only when a concrete requirement justifies them.

 1. Data Models

Suggested structure:

text
models/
├── mod.rs
├── file_entry.rs
├── file_metadata.rs
├── directory.rs
├── search_result.rs
├── operation.rs
└── location.rs

Models should contain data and basic domain behavior.

They should not depend on GUI widgets.

Example conceptual file entry:

rust
struct FileEntry {
name: String,
path: std::path::PathBuf,
is_directory: bool,
}

Additional metadata should be loaded when needed rather than forcing every entry to contain all possible information.

 1. Configuration Architecture

Suggested structure:

text
config/
├── mod.rs
├── settings.rs
├── defaults.rs
└── storage.rs

Settings may include:

Theme preference.
Default view mode.
Show hidden files.
Thumbnail preference.
Cache limits.
Search preferences.
Default sorting.
Confirmation preferences.

Settings must be stored using appropriate Linux user configuration directories.

Avoid writing configuration or cache files into system directories.

 1. Error Handling

Errors should be handled explicitly.

Suggested error categories:

Filesystem errors.
Permission errors.
Invalid paths.
Missing files.
Device disconnection.
Unsupported formats.
Operation conflicts.
Search errors.
Cache errors.

Requirements:

Never silently ignore important failures.
Do not crash because a file cannot be accessed.
Provide understandable error messages.
Preserve application state after recoverable failures.
Log useful diagnostic information without exposing unnecessary personal file contents. 15. Security and Filesystem Safety

The application must:

Respect user permissions.
Avoid unnecessary privilege escalation.
Never execute files automatically just because they are selected.
Treat filenames and file metadata as untrusted input.
Handle symbolic links carefully.
Avoid unintended recursive traversal.
Confirm destructive actions.
Avoid silently overwriting existing files.
Use safe Rust wherever possible.
Keep unsafe code isolated and documented if unavoidable. 16. Resource Management

Resource consumption must be controlled across all components.

RAM
Bounded directory cache.
Bounded metadata cache.
Limited thumbnail workers.
Efficient file models.
Virtualized UI.
Avoid duplicate data storage.
CPU
Avoid unnecessary polling.
Use filesystem monitoring where available.
Limit background workers.
Avoid repeated metadata retrieval.
Prioritize user-initiated operations.
Disk
Optional database.
Limited thumbnail cache.
No mandatory background indexing.
Minimal application configuration.
Reuse standard Linux cache locations. 17. Testing Architecture

Suggested structure:

text
tests/
├── filesystem_tests.rs
├── navigation_tests.rs
├── search_tests.rs
├── operations_tests.rs
├── cache_tests.rs
└── performance_tests.rs

Testing must include:

Small directories.
Large directories.
Deep directory structures.
Hidden files.
Symbolic links.
Permission failures.
Missing paths.
Read-only filesystems.
Interrupted operations.
Large file operations.

Performance testing must measure:

Startup time.
Directory scanning latency.
Time to first visible entries.
Peak RAM.
Idle RAM.
CPU consumption.
Search latency.
Thumbnail generation overhead. 18. Build and Distribution Architecture

The application should use Cargo for development and release builds.

The project should eventually support:

Native distribution packages.
Flatpak.
AppImage where practical.
Other Linux packaging formats as needed.

Do not assume that GTK4 and GIO runtime dependencies are bundled into a small executable.

Document system dependencies and runtime requirements clearly.

 1. Architectural Constraints

All developers and AI coding agents must follow these rules:

Do not mix GUI and filesystem logic.
Do not place all logic in main.rs.
Do not perform blocking operations on the GTK main thread.
Do not create unlimited background workers.
Do not create a widget for every file in large directories.
Do not introduce dependencies without a reason.
Do not add mandatory indexing.
Do not use unbounded caches.
Do not use unsafe Rust without a clear justification.
Do not rewrite working components unnecessarily.
Do not create unnecessary abstraction layers.
Keep modules independently testable.
Maintain compatibility with Linux desktop environments.
Update this document when architectural decisions change. 20. Final Architecture Goal

The architecture must enable the application to remain:

Lightweight.
Fast.
Memory-efficient.
Responsive.
Reliable.
Maintainable.
Extensible.

The architecture should make performance a property of the entire application rather than an optimization added at the end.

The ultimate goal is a file manager that feels instant without wasting the user's system resources.
