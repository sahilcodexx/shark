Software Requirements Specification (requirement.md)

1. Project Overview

The project is a lightweight, modern Linux file manager built using Rust and GTK4.

Its purpose is to provide a fast, responsive and resource-efficient alternative to traditional Linux file managers such as GNOME Files and KDE Dolphin.

The application must prioritize:

Low RAM consumption.
Minimal disk usage.
Fast directory navigation.
Fast file searching.
Responsive user interface.
Reliable filesystem operations.
Cross-distribution Linux compatibility.

The application should have a minimal, modern interface inspired by GNOME Files.

1. Target Platform Requirements
   2.1 Supported Platforms

The application must support mainstream Linux distributions, including:

Ubuntu
Debian
Fedora
Arch Linux
Linux Mint
openSUSE
Other mainstream Linux distributions.
2.2 Desktop Environment Compatibility

The application must work on:

GNOME
KDE Plasma
XFCE
Cinnamon
MATE
Other GTK-compatible desktop environments.

The application must not depend on GNOME Shell-specific functionality.

2.3 System Compatibility
Support common Linux filesystems.
Respect Linux file permissions.
Support symbolic links.
Handle removable storage.
Support mounted drives.
Allow future integration with network filesystems.
Respect system theme preferences where possible. 3. Functional Requirements
3.1 Directory Navigation

The application must provide:

Opening directories.
Back navigation.
Forward navigation.
Parent directory navigation.
Breadcrumb navigation.
Home directory shortcut.
Root directory access.
Directory refresh.
Bookmark navigation.
Recent locations.
Acceptance Criteria
Navigation must not freeze the UI.
Directory contents should appear progressively when appropriate.
Previously visited directories may be cached.
Stale cached information must not be presented as guaranteed current information.
3.2 File Display

The application must support:

Grid view.
List view.
File names.
File icons.
File sizes.
Modification dates.
File types.
Folder indicators.
Hidden files.
Sorting Options
Name.
Size.
Modification date.
File type.

Sorting must support ascending and descending order.

Display Requirements
Large directories must be handled efficiently.
Use virtualized lists or equivalent UI recycling.
Do not create a separate UI widget for every file in a large directory.
File metadata should be loaded only when needed.
3.3 File Operations

The application must support:

Operation Requirement
Open Open files using the system default application
Copy Copy files and directories
Move Move files and directories
Rename Rename files and directories
Delete Delete with appropriate confirmation
Create folder Create new directories
Create file Create new empty files
Trash Move files to the system trash
Permanent delete Explicitly confirm destructive operations
File Operation Requirements
Operations must run in the background.
The UI must remain responsive.
Progress must be displayed for long-running operations.
Errors must be reported clearly.
Permission failures must be handled safely.
Interrupted operations must not leave the application in an inconsistent state.
Conflicting destination filenames must be handled through user-selectable options.
3.4 Search System

Search is a major priority.

The application must provide two search capabilities.

Basic Search
Search filenames in the current directory.
Filter visible files instantly.
Support partial filename matching.
Support case-insensitive search.
Allow clearing search quickly.
Advanced Search

Future requirements:

Recursive filename search.
Search by extension.
Search by file type.
Search by size.
Search by modification date.
Optional background indexing.
Optional full-text search.
Search Performance Requirements
Basic filename filtering must not require a database.
Recursive searching must run in the background.
Search must not freeze the GUI.
Indexing must be optional.
Indexing must be controllable by the user.
No mandatory always-running indexing daemon.
3.5 Thumbnail System

The application must support thumbnails for common file types.

Initial targets:

Images.
Videos where supported.
Other previewable file types in future releases.
Requirements
Generate thumbnails lazily.
Prioritize visible items.
Run generation in background workers.
Cache generated thumbnails.
Avoid generating duplicate thumbnails.
Limit concurrent thumbnail processing.
Respect thumbnail cache limits.
Allow users to disable thumbnails.
3.6 Tabs

The application should support:

Opening multiple tabs.
Closing tabs.
Switching between tabs.
Opening a directory in a new tab.
Preserving navigation history per tab.

Tabs must not unnecessarily duplicate large amounts of cached data.

3.7 Sidebar

The sidebar should provide:

Home.
Desktop.
Documents.
Downloads.
Pictures.
Music.
Videos.
Trash.
Bookmarks.
Mounted volumes.

Unavailable locations must be handled gracefully.

3.8 Context Menu

The application must provide appropriate context menus for:

Files
Open.
Open with.
Copy.
Cut.
Rename.
Move to trash.
Delete.
Properties.
Directories
Open.
Open in new tab.
Open in new window.
Copy.
Cut.
Rename.
Move to trash.
Properties.

Context menus must not perform destructive operations without appropriate confirmation.

3.9 File Properties

The application should display:

File name.
File type.
File size.
Location.
Modified date.
Created date where available.
Permissions.
Ownership where accessible.

Properties should be loaded on demand.

3.10 Keyboard Shortcuts

Initial shortcuts:

Shortcut Action
Ctrl + L Enter location
Ctrl + F Search
Ctrl + T New tab
Ctrl + W Close tab
Ctrl + H Show hidden files
Ctrl + C Copy
Ctrl + X Cut
Ctrl + V Paste
Ctrl + A Select all
Delete Move selected items to trash
F2 Rename
Alt + Left Go back
Alt + Right Go forward
Ctrl + R Refresh

Shortcuts should follow common Linux desktop conventions where possible.

1. User Interface Requirements
   4.1 Design

The UI must be:

Minimal.
Modern.
Clean.
Consistent.
Responsive.
Easy to navigate.

The visual direction should be inspired by GNOME Files without copying its implementation.

4.2 Themes

The application must support:

Light theme.
Dark theme.
System theme preference.
4.3 Layout

The main window should contain:

Header bar.
Navigation controls.
Breadcrumb or location entry.
Search control.
Sidebar.
Main file display.
Status information where useful.
4.4 UI Performance
UI rendering must remain independent of expensive filesystem operations.
Avoid unnecessary animations.
Avoid excessive widget creation.
Avoid unnecessary UI updates.
Large lists must use virtualization.
Updates should be applied efficiently. 5. Performance Requirements

Performance is a core product requirement.

The following are proposed targets for development and testing, not guaranteed hardware-independent results.

Metric Initial target
Idle RAM Under 30 MB
Startup time Under 300 ms
Small directory navigation Under 100 ms
Main UI thread blocking No intentional blocking filesystem operations
Installed size Under 20 MB where practical

All measurements must specify the hardware, filesystem, operating environment and test conditions.

5.1 Memory Requirements
Avoid unnecessary allocations.
Use bounded caches.
Release unused resources.
Avoid retaining unnecessary metadata.
Limit background worker concurrency.
Avoid loading unnecessary file contents.
Avoid unbounded thumbnail caches.
5.2 CPU Requirements
Avoid unnecessary background scanning.
Avoid constant polling where filesystem monitoring is available.
Limit concurrent background tasks.
Avoid expensive repeated metadata queries.
Avoid unnecessary thumbnail processing.
5.3 Disk Requirements
Keep the application installation small.
Avoid unnecessary dependencies.
Avoid mandatory databases.
Avoid unnecessary cache files.
Allow users to clear application caches. 6. Filesystem Requirements

The filesystem engine must be independent of the GUI.

It should provide reusable interfaces for:

Directory scanning.
File metadata retrieval.
File operations.
Search.
Directory monitoring.
Thumbnail requests.
Safety Requirements
Respect filesystem permissions.
Handle symbolic links correctly.
Avoid accidental recursive traversal through symbolic links.
Handle unavailable mounts.
Handle disconnected removable devices.
Detect filesystem errors.
Handle filename encoding and special characters correctly.
Avoid unsafe filesystem operations wherever possible. 7. Architecture Requirements

The application should use a modular architecture.

GUI Layer

Responsible for:

Main window.
Sidebar.
Toolbar.
File display.
Context menus.
User interactions.
Filesystem Layer

Responsible for:

Directory scanning.
Metadata.
File operations.
Filesystem monitoring.
Search Layer

Responsible for:

Filename filtering.
Recursive searching.
Optional indexing.
Thumbnail Layer

Responsible for:

Thumbnail generation.
Thumbnail caching.
Background processing.
Cache Layer

Responsible for:

Directory cache.
Metadata cache.
Thumbnail cache.

Components should communicate through clear interfaces and avoid unnecessary coupling.

1. Reliability Requirements
   The application must handle invalid paths gracefully.
   The application must not crash when a mounted device disconnects.
   Errors must be reported without crashing the application.
   File operations must handle interruption.
   UI state must remain consistent after operation failures.
   The application must not silently discard user data.
   Destructive operations must have appropriate safeguards.
2. Accessibility and Usability
   Support keyboard navigation.
   Provide understandable labels.
   Support screen readers where GTK allows.
   Maintain adequate contrast.
   Provide clear error messages.
   Avoid relying only on color to communicate important information.
   Make important actions accessible through keyboard shortcuts.
3. Testing Requirements

Testing must cover:

Functional Testing
Directory navigation.
File operations.
Search.
Sorting.
Hidden files.
Tabs.
Bookmarks.
Performance Testing
1,000 files.
10,000 files.
100,000 files.
Deep directory structures.
Large files.
Large thumbnail workloads.
Reliability Testing
Permission denied.
Missing files.
Disconnected storage.
Read-only filesystems.
Symbolic links.
Filename conflicts.
Interrupted operations.
Compatibility Testing
GNOME.
KDE Plasma.
XFCE.
Multiple Linux distributions. 11. Development Priorities

Implementation should follow this order:

Priority Feature
P0 Application startup and main window
P0 Directory scanning and navigation
P0 Efficient file display
P0 Low-memory architecture
P1 Basic file operations
P1 Sorting and hidden files
P1 Search
P1 Sidebar and bookmarks
P2 Tabs
P2 Thumbnails
P2 Properties
P3 Archive support
P3 Network locations
P3 Advanced indexing

P0 features are essential for the first usable release.

1. Definition of Done

A feature is considered complete only when:

It works correctly.
It handles errors.
It does not block the main UI thread.
It does not introduce unnecessary resource consumption.
It has appropriate tests.
It follows the project's architecture.
Its documentation is updated when necessary. 13. Explicit Non-Requirements

The following are not initial development priorities:

Cloud synchronization.
Built-in media playback.
Built-in document editing.
Heavyweight plugin systems.
Mandatory background indexing.
Automatic cloud uploads.
Electron or web-based application architecture.
Unnecessary desktop environment integrations.

These features must not be added without explicit project approval.

1. Final Product Goal

The final application should provide a modern and reliable file management experience while using significantly fewer system resources than heavyweight alternatives where measurable.

The application should feel fast during everyday tasks, especially opening folders, navigating directories and finding files.

Every engineering decision should support this objective.

Build a file manager that respects the user's hardware, time and resources.
