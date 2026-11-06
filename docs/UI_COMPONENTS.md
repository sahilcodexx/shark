UI_COMPONENTS.md

1. Introduction

This document defines the reusable UI component system for the lightweight Linux file manager.

The application uses:

Rust
GTK4
gtk-rs
GLib / GIO

The UI should follow a minimal, modern GNOME-inspired design while maintaining its own visual identity.

All components must prioritize:

Performance.
Reusability.
Consistency.
Accessibility.
Maintainability.

Components should be small, modular and independently understandable.

Avoid unnecessary component abstraction and excessive widget nesting.

1. Component Architecture

The UI must be divided into reusable components.

Suggested directory structure:

text
src/
└── gui/
├── mod.rs
│
├── window/
│ ├── mod.rs
│ └── main_window.rs
│
├── navigation/
│ ├── mod.rs
│ ├── navigation_bar.rs
│ ├── breadcrumb.rs
│ └── location_entry.rs
│
├── sidebar/
│ ├── mod.rs
│ ├── sidebar.rs
│ ├── sidebar_item.rs
│ └── sidebar_section.rs
│
├── file_view/
│ ├── mod.rs
│ ├── file_view.rs
│ ├── grid_view.rs
│ ├── list_view.rs
│ ├── file_item.rs
│ └── selection.rs
│
├── toolbar/
│ ├── mod.rs
│ ├── toolbar.rs
│ └── view_switcher.rs
│
├── tabs/
│ ├── mod.rs
│ ├── tab_bar.rs
│ └── tab_item.rs
│
├── menus/
│ ├── mod.rs
│ ├── file_context_menu.rs
│ └── background_context_menu.rs
│
├── dialogs/
│ ├── mod.rs
│ ├── rename_dialog.rs
│ ├── create_folder_dialog.rs
│ ├── delete_dialog.rs
│ └── properties_dialog.rs
│
├── operations/
│ ├── mod.rs
│ └── operation_progress.rs
│
├── empty_states/
│ ├── mod.rs
│ └── empty_state.rs
│
└── widgets/
├── mod.rs
├── icon_button.rs
├── file_icon.rs
└── loading_indicator.rs

This is a suggested structure. Do not create unnecessary modules or files before they are needed.

1. Main Window Components
   3.1 MainWindow

File: window/main_window.rs

Purpose:

The main application window containing all primary UI components.

Responsibilities:

Initialize the main layout.
Manage the header bar.
Manage the sidebar.
Display the active file view.
Coordinate tabs.
Handle window-level shortcuts.
Manage window state.

Layout:

text
+----------------------------------------------------+
| Header Bar |
+----------------------------------------------------+
| Tab Bar |
+--------------+-------------------------------------+
| | |
| Sidebar | Main File View |
| | |
| | |
+--------------+-------------------------------------+
| Status Information |
+----------------------------------------------------+

Requirements:

Responsive window resizing.
Consistent margins.
No unnecessary nested containers.
No filesystem logic inside the window component.
Avoid rebuilding the complete window when navigation changes.
3.2 HeaderBar

File: toolbar/toolbar.rs

Purpose:

Provide the main navigation and application controls.

Components:

Back button.
Forward button.
Location display.
Search button.
View switcher.
Main menu.

Requirements:

Compact design.
Consistent button sizes.
Tooltips for icon-only controls.
Clear disabled states.
Keyboard accessibility.
3.3 NavigationBar

File: navigation/navigation_bar.rs

Responsibilities:

Back navigation.
Forward navigation.
Parent directory navigation.
Location changes.

The component must send navigation requests to the application controller.

It must not perform directory scanning itself.

3.4 Breadcrumb

File: navigation/breadcrumb.rs

Purpose:

Display the current directory path.

Requirements:

Display path segments.
Allow navigation to parent segments.
Handle long paths.
Support overflow.
Provide a location-entry mode.
Maintain consistent styling. 4. Sidebar Components
4.1 Sidebar

File: sidebar/sidebar.rs

Purpose:

Provide quick navigation to frequently used locations.

Sections:

Personal locations.
Bookmarks.
Mounted devices.
Trash.
Network locations where supported.

Requirements:

Allow collapsing the sidebar.
Highlight the current location.
Support drag-and-drop bookmarking where appropriate.
Update mounted devices dynamically.
Avoid unnecessary filesystem polling.
4.2 SidebarSection

File: sidebar/sidebar_section.rs

Purpose:

Group related sidebar entries.

Examples:

Places.
Bookmarks.
Devices.

Requirements:

Clear section labels.
Optional collapse behavior.
Consistent spacing.
4.3 SidebarItem

File: sidebar/sidebar_item.rs

Purpose:

Represent an individual sidebar destination.

Properties:

Icon.
Label.
Location identifier.
Selection state.
Optional secondary status.

Requirements:

Clear hover state.
Clear selected state.
Accessible label.
Tooltip for truncated labels where useful. 5. File View Components

The file view is the most important UI subsystem.

It must handle large directories without creating excessive widgets or consuming unnecessary memory.

5.1 FileView

File: file_view/file_view.rs

Purpose:

Act as the main container for file presentation.

Responsibilities:

Display grid or list mode.
Manage sorting.
Handle selection.
Display loading and empty states.
Receive incremental directory updates.

It must not own the filesystem scanner.

It receives data through the application state or controller.

5.2 GridView

File: file_view/grid_view.rs

Purpose:

Display files and directories in a grid.

Each visible item should contain:

File icon.
Filename.
Optional selection indicator.

Requirements:

Responsive column count.
Consistent item spacing.
Filename wrapping.
Efficient scrolling.
Virtualized rendering.
Minimal visual decoration.

Use GTK4's GtkGridView where appropriate.

5.3 ListView

File: file_view/list_view.rs

Purpose:

Display files in a compact detailed list.

Columns:

Name.
Size.
Type.
Modified date.

Requirements:

Compact row height.
Consistent alignment.
Sorting indicators.
Virtualized rows.
Efficient selection.
Responsive column behavior.

Use GTK4's GtkListView and suitable model components.

5.4 FileItem

File: file_view/file_item.rs

Purpose:

Represent one file or directory in the UI.

Information:

File name.
File icon.
File type.
Selection state.
Optional metadata.

Requirements:

Reusable presentation.
Minimal widget complexity.
No direct filesystem queries.
No thumbnail generation inside the widget.
No expensive processing during rendering.

The component should receive already-prepared display data.

5.5 SelectionManager

File: file_view/selection.rs

Purpose:

Manage file selection.

Supported operations:

Single selection.
Multiple selection.
Range selection.
Select all.
Clear selection.

Selection state should be managed separately from file metadata.

Avoid copying entire file records just to update selection.

1. Search Components
   6.1 SearchBar

File: navigation/search_bar.rs

Purpose:

Provide an interface for searching and filtering files.

Requirements:

Search input.
Clear button.
Search state.
Optional progress indicator.
Empty-result handling.

Behavior:

Open with Ctrl + F.
Allow immediate filename filtering.
Support cancellation.
Return focus to the file view when appropriate.

The component must not implement the search engine.

It sends search requests to the controller.

1. Tab Components
   7.1 TabBar

File: tabs/tab_bar.rs

Purpose:

Display open application tabs.

Requirements:

Display multiple tabs.
Highlight active tab.
Support closing tabs.
Support adding tabs.
Support keyboard navigation.
Avoid excessive horizontal space.
7.2 TabItem

File: tabs/tab_item.rs

Purpose:

Represent one open tab.

Display:

Location or shortened directory name.
Optional loading indicator.
Close control.

Each tab must have independent navigation state.

1. Menu Components
   8.1 FileContextMenu

File: menus/file_context_menu.rs

Displayed when the user right-clicks a file or selected files.

Actions:

Open.
Open with.
Cut.
Copy.
Rename.
Move to trash.
Properties.

Actions must respect the current selection and file permissions.

8.2 BackgroundContextMenu

Displayed when the user right-clicks empty space in a directory.

Actions:

New folder.
New file.
Paste.
Refresh.
Select all.
Change view mode.

Only display actions that are currently relevant.

1. Dialog Components
   9.1 RenameDialog

Purpose:

Allow renaming files and directories.

Requirements:

Display the current name.
Focus the filename input.
Handle invalid names.
Display errors.
Avoid accidental extension removal where appropriate.
9.2 CreateFolderDialog

Purpose:

Create a new directory.

Requirements:

Filename input.
Validation.
Create and cancel actions.
Clear error feedback.
9.3 DeleteDialog

Purpose:

Confirm destructive operations.

Requirements:

Clearly identify the operation.
Display the number of selected items.
Distinguish moving to trash from permanent deletion.
Require explicit confirmation for permanent deletion.
9.4 PropertiesDialog

Purpose:

Display file and directory information.

Information:

Name.
Type.
Location.
Size.
Modified date.
Permissions.
Ownership where accessible.

Metadata should be loaded on demand.

 1. File Operation Components
    10.1 OperationProgress

File: operations/operation_progress.rs

Purpose:

Display progress for long-running filesystem operations.

Supported operations:

Copy.
Move.
Delete.
Trash.
Other future operations.

Display:

Operation name.
Source and destination where useful.
Progress indicator.
Completion state.
Error state.
Cancellation control where supported.

Requirements:

Support multiple operations.
Avoid excessive UI updates.
Do not block the main thread.
Display meaningful progress.
Do not pretend an operation is complete before the backend confirms it. 11. Empty State Components
11.1 EmptyState

File: empty_states/empty_state.rs

Purpose:

Display helpful information when no files or results are available.

Supported states:

Empty directory.
No search results.
Unavailable location.
Directory access denied.
Loading state.

Requirements:

Simple icon or illustration.
Short explanatory text.
Relevant recovery action where appropriate.
No oversized decorative graphics. 12. Shared Widgets
12.1 IconButton

File: widgets/icon_button.rs

A reusable icon-only button.

Requirements:

Consistent sizing.
Accessible label.
Tooltip.
Hover state.
Disabled state.
Keyboard focus indication.
12.2 FileIcon

File: widgets/file_icon.rs

Purpose:

Display appropriate file and folder icons.

Requirements:

Respect the installed icon theme.
Use Adwaita as a fallback.
Support symbolic and full-color icons.
Avoid unnecessary icon processing.
Support common file types.
Use a generic fallback for unknown types.

Icon lookup should be cached where appropriate.

12.3 LoadingIndicator

File: widgets/loading_indicator.rs

Purpose:

Indicate background activity.

Requirements:

Small visual footprint.
No excessive animation.
Appropriate accessibility description.
Display only when useful. 13. Design Tokens

All components must use a centralized design system.

Suggested structure:

text
assets/
└── styles/
├── colors.css
├── typography.css
├── spacing.css
├── components.css
├── light.css
└── dark.css

13.1 Spacing

Use a consistent spacing scale.

Suggested values:

Token Value
xs 4 px
sm 8 px
md 12 px
lg 16 px
xl 24 px
xxl 32 px

These are initial design values and may be adjusted after testing.

13.2 Corner Radius

Use restrained rounding.

Suggested values:

Element Radius
Small buttons 6 px
Input fields 8 px
Popovers 10 px
Dialogs 12 px

Follow GTK and Adwaita conventions where possible.

13.3 Icon Sizes

Suggested sizes:

Usage Size
Small toolbar actions 16 px
Standard actions 20 px
Sidebar 20 px
Grid file icons 48–64 px
Large previews Context-dependent 14. Component Communication Rules

UI components must not communicate through uncontrolled direct access to each other's internal state.

Use:

Application state.
Explicit callbacks.
GTK signals.
GLib-compatible asynchronous messages.
Shared models where appropriate.

Example:

text
User Action
|
v
UI Component
|
v
Application Controller
|
v
Backend Request
|
v
Backend Result
|
v
Application State Update
|
v
UI Refresh

The GUI must not contain backend implementation logic.

GTK widgets must only be accessed from the GTK main thread.

 1. Performance Requirements

Every UI component must follow these rules:

Avoid unnecessary widget creation.
Avoid unnecessary widget destruction.
Use GTK virtualization for large lists.
Avoid expensive operations during rendering.
Avoid repeated icon lookups.
Avoid unnecessary metadata formatting.
Update only affected UI elements.
Avoid excessive signal connections.
Avoid unnecessary animations.
Release unused resources.
Avoid keeping hidden content unnecessarily alive.
Keep expensive work off the GTK main thread.

The file view must be optimized before implementing advanced visual features.

 1. Component Development Rules

All developers and AI coding agents must follow these rules:

Read this document before creating new UI components.
Reuse existing components whenever practical.
Do not duplicate existing functionality.
Keep each component focused on one responsibility.
Do not mix filesystem logic with presentation.
Avoid unnecessary abstraction.
Follow the established design tokens.
Support light and dark themes.
Maintain accessibility.
Test different window sizes.
Avoid performance regressions.
Document new shared components.
Do not create every planned component in advance.
Implement components incrementally as features require them. 17. Implementation Order

Recommended component development sequence:

Phase 1: Basic Application UI
MainWindow.
HeaderBar.
Sidebar.
Breadcrumb.
FileView.
GridView.
ListView.
Phase 2: Navigation
NavigationBar.
LocationEntry.
SidebarItem.
SidebarSection.
Phase 3: File Interactions
FileItem.
SelectionManager.
FileContextMenu.
BackgroundContextMenu.
Phase 4: Search and Tabs
SearchBar.
TabBar.
TabItem.
Phase 5: Dialogs and Operations
RenameDialog.
CreateFolderDialog.
DeleteDialog.
PropertiesDialog.
OperationProgress.
Phase 6: Polish
EmptyState.
LoadingIndicator.
IconButton.
FileIcon.
Preferences.

Only create and implement components when they are needed.

 1. Final Component Design Goal

The UI component system should remain:

Lightweight.
Reusable.
Consistent.
Accessible.
Easy to maintain.
Easy to extend.
Efficient with large directories.

Components should support the application's primary objective: fast and efficient file management.

Build only what is needed, reuse what already exists, and never let UI complexity compromise application performance.
