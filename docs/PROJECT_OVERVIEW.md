Project Overview

1. Project Introduction

This project is a lightweight, modern Linux file manager designed to provide a significantly faster and more resource-efficient alternative to traditional desktop file managers such as GNOME Files and KDE Dolphin.

The primary focus is performance, responsiveness, simplicity, and minimal resource consumption.

The application should provide a clean and modern interface inspired by GNOME Files while maintaining excellent compatibility across different Linux distributions and desktop environments.

This is not intended to be a direct clone of an existing file manager. The goal is to build a better file management experience through efficient engineering and thoughtful architecture.

1. Project Vision

Build a file manager that:

Opens directories quickly.
Searches for files with minimal delay.
Consumes very little RAM.
Requires minimal disk space.
Handles directories containing thousands or hundreds of thousands of files efficiently.
Remains responsive during background operations.
Provides a clean, minimal and modern interface.
Works across major Linux distributions and desktop environments.
Core Philosophy

Performance first. Simplicity second. Features third.

Every feature must justify its performance and resource cost.

Avoid unnecessary background processes, excessive caching, heavyweight dependencies and unnecessary UI complexity.

1. Technology Stack
   Component Technology
   Programming language Rust
   GUI framework GTK4
   Desktop integration GIO / GLib
   Build system Cargo
   Search database SQLite (optional)
   Image processing GdkPixbuf
   Testing Rust testing framework
   Technology Principles
   Prefer lightweight and well-maintained dependencies.
   Avoid unnecessary external frameworks.
   Use native Linux capabilities wherever possible.
   Avoid Electron, Chromium-based application shells and web-based UI.
   Do not introduce a garbage collector.
   Keep the application modular and maintainable.
2. Target Platform

The primary target is Linux.

The application should work across:

GNOME
KDE Plasma
XFCE
Cinnamon
MATE
Other GTK-compatible Linux desktop environments.

It should not depend on GNOME Shell or GNOME-specific desktop functionality.

Distribution compatibility should include:

Ubuntu
Debian
Fedora
Arch Linux
Linux Mint
openSUSE
Other mainstream Linux distributions.

Avoid depending on distribution-specific paths or tools wherever possible.

1. Design Philosophy

The interface should take inspiration from GNOME Files.

Visual Principles
Minimal and modern design.
Clean spacing and consistent alignment.
Simple navigation.
Beautiful folder and file icons.
Subtle animations only where useful.
Fast interface rendering.
Proper dark and light themes.
Responsive layouts.
Interface Components

The planned interface includes:

Sidebar for locations, bookmarks and mounted drives.
Top toolbar for navigation and common actions.
Breadcrumb navigation.
Search interface.
Main file display area.
Grid and list views.
Context menus.
File operation progress interface.

The UI must never block while filesystem operations are running.

1. Performance Goals

Performance is the most important aspect of this project.

The following are initial engineering targets, not guaranteed measurements.

Metric Target
Idle RAM Under 30 MB
Startup time Under 300 ms
Small directory navigation Under 100 ms
UI blocking during background work Zero intentional blocking
Installed size Under 20 MB where practical

Actual measurements will depend on hardware, GTK dependencies, filesystem and desktop environment.

Performance Requirements
Directory Loading
Display directory entries as they become available.
Avoid unnecessary metadata queries.
Avoid loading everything into UI widgets.
Handle very large directories efficiently.
Memory Management
Use bounded caches.
Release unnecessary resources.
Avoid retaining unused file metadata.
Reuse UI elements where possible.
Avoid unnecessary memory allocations.
UI Responsiveness
Never perform expensive filesystem operations on the UI thread.
Use asynchronous operations where appropriate.
Keep rendering independent from directory scanning.
Avoid unnecessary animations and effects.
Thumbnail Generation
Generate thumbnails lazily.
Prioritize visible files.
Process thumbnails in background workers.
Cache generated thumbnails efficiently. 7. Planned Features

Features will be implemented incrementally.

Phase 1: Core File Management
Directory navigation.
File and folder listing.
Breadcrumb navigation.
Back and forward navigation.
Basic sorting.
Phase 2: File Operations
Copy.
Move.
Delete.
Rename.
Create files and folders.
Progress tracking.
Phase 3: Interface
Grid view.
List view.
Sidebar.
Tabs.
Bookmarks.
Keyboard shortcuts.
Context menus.
Phase 4: Search
Instant filename filtering.
Recursive search.
Optional background indexing.
Search filters.
Phase 5: Previews
Image thumbnails.
Image previews.
Video thumbnails where supported.
Document previews where supported.
Phase 6: Advanced Features
Archive browsing.
Bulk rename.
Network locations.
Mounted drives.
Recent files.
Undo and redo.
File properties. 8. Search Philosophy

Search must be fast without unnecessarily consuming system resources.

The initial implementation should prioritize filename filtering and directory search without requiring a permanent indexing service.

Optional indexing may be introduced later.

Search Requirements
Fast response for filename searches.
Background recursive searching.
Optional persistent indexing.
No mandatory always-running indexing daemon.
Configurable indexing behavior.

SQLite FTS5 can be considered for optional indexed search.

1. Filesystem Architecture

Filesystem operations should be independent of the GUI.

The filesystem engine should handle:

Directory scanning.
File metadata.
File operations.
Directory monitoring.
Search.
Thumbnail requests.

GIO should be used where it provides useful Linux filesystem and virtual filesystem integration.

The architecture should support local filesystems first and allow future integration with remote locations and virtual filesystems.

Filesystem operations must handle errors, permissions, symbolic links and interrupted operations correctly.

 1. Project Architecture

Keep the application modular.

Suggested structure:

text
src/
├── main.rs
├── app.rs
├── config.rs
│
├── gui/
│ ├── mod.rs
│ ├── window.rs
│ ├── sidebar.rs
│ ├── toolbar.rs
│ └── file_view.rs
│
├── filesystem/
│ ├── mod.rs
│ ├── scanner.rs
│ ├── operations.rs
│ └── metadata.rs
│
├── navigation/
│ ├── mod.rs
│ └── history.rs
│
├── models/
│ ├── mod.rs
│ └── file_entry.rs
│
└── utils/
└── mod.rs

Modules should have clear responsibilities and minimal coupling.

Do not place all application logic inside main.rs.

 1. Development Rules

All developers and AI coding agents working on this project must follow these rules:

Prioritize performance in every implementation.
Avoid unnecessary dependencies.
Do not introduce heavyweight frameworks.
Keep filesystem logic separate from UI logic.
Never block the main UI thread with expensive operations.
Use safe Rust wherever possible.
Avoid unnecessary unsafe code.
Write maintainable and idiomatic Rust.
Handle errors properly.
Test performance-sensitive code.
Do not implement features that have not been requested.
Do not rewrite existing architecture without a clear reason.
Update documentation when architecture changes.
Measure performance rather than making unsupported optimization claims. 12. Benchmarking

Performance must be measured against existing file managers, especially current versions of:

GNOME Files (Nautilus).
KDE Dolphin.

Benchmark scenarios should include:

Opening small directories.
Opening directories containing 10,000 files.
Opening directories containing 100,000 files.
Navigating between directories.
Recursive filename searches.
Thumbnail generation.
Bulk copying and moving.
Idle memory consumption.
Peak memory consumption.

Use the same hardware, filesystem and test conditions for fair comparisons.

 1. Project Priorities

Development decisions should follow this priority order:

Correctness and filesystem safety.
Low memory consumption.
Fast directory navigation.
Responsive UI.
Fast searching.
Minimal installation size.
Modern visual design.
Additional features.

A feature should not be added if it introduces significant resource consumption without providing meaningful value.

 1. Long-Term Vision

The ultimate goal is to create a lightweight, reliable and modern Linux file manager that feels instantaneous even on older or resource-constrained hardware.

It should provide the essential capabilities users expect from a modern file manager without the unnecessary complexity and overhead that can make everyday file management feel slow.

The goal is not to have the most features. The goal is to make everyday file management exceptionally fast, simple and efficient.
