UI_UX_GUIDELINES.md

1. Design Vision

The application should provide a clean, minimal, modern and intuitive Linux desktop experience.

The visual direction is inspired by GNOME Files (Nautilus), but the application must have its own visual identity.

The interface should feel:

Minimal.
Modern.
Lightweight.
Clean.
Responsive.
Consistent.
Easy to understand.

The design must prioritize usability over decoration.

Core Design Philosophy

Less visual noise. More useful space. Faster interactions.

Every visual element must have a purpose.

Avoid unnecessary borders, shadows, gradients, animations and decorative elements.

1. Design Principles
   2.1 Minimalism
   Avoid unnecessary UI elements.
   Keep the toolbar compact.
   Use whitespace effectively.
   Avoid excessive separators.
   Keep context menus concise.
   Display advanced options only when necessary.
   2.2 Consistency
   Use consistent spacing.
   Maintain consistent icon sizes.
   Use predictable interaction patterns.
   Keep button styles consistent.
   Use consistent typography.
   Maintain consistent visual hierarchy.
   2.3 Responsiveness
   UI interactions should feel immediate.
   Avoid unnecessary transitions.
   Avoid expensive visual effects.
   Never freeze the interface during filesystem operations.
   Update only the UI elements that need updating.
   2.4 Accessibility
   Maintain readable text contrast.
   Support keyboard navigation.
   Provide tooltips for icon-only controls.
   Use clear labels.
   Respect system accessibility settings.
   Do not communicate important information through color alone.
2. Visual Identity
   3.1 Overall Style

The interface should follow a GNOME-inspired design language.

Reference:

GNOME Files.
GNOME Adwaita design.
Modern GTK4 applications.

However, the application must not blindly copy another application's layout.

It should establish its own visual identity while maintaining familiarity for Linux users.

3.2 Color System

Use a consistent semantic color system.

Light Theme
Background: Light neutral.
Primary surface: White or near-white.
Secondary surface: Light gray.
Primary text: Dark neutral.
Secondary text: Muted gray.
Accent: Restrained blue or system accent.
Destructive actions: Red.
Dark Theme
Background: Dark neutral.
Primary surface: Slightly lighter dark surface.
Secondary surface: Dark gray.
Primary text: Near-white.
Secondary text: Muted gray.
Accent: Accessible blue or system accent.
Destructive actions: Red.
Color Rules
Avoid excessive accent colors.
Avoid unnecessary gradients.
Avoid highly saturated backgrounds.
Use color primarily for meaning and interaction.
Prefer system theme integration where possible.

Do not hardcode every color independently throughout the application.

Use centralized design tokens.

1. Typography

Typography should be clean and readable.

Requirements
Prefer the system interface font.
Use consistent font weights.
Avoid unnecessary font variations.
Maintain readable file names.
Use secondary text for metadata.
Prevent long filenames from breaking layouts.
Typography Hierarchy
Element Style
Window title Medium / semibold
Section headings Medium
File names Regular
Metadata Smaller, muted
Buttons Medium
Tooltips Regular

Avoid unnecessarily large typography.

The main file area should maximize usable space.

1. Main Window Layout

The main application window should contain the following structure:

text
+------------------------------------------------------+
| Navigation | Location / Breadcrumb | Search | Menu |
+------------------------------------------------------+
| | |
| Sidebar | |
| | Main File View |
| Home | |
| Recent | [ Folder ] [ Folder ] [ File ] |
| Starred | |
| | [ File ] [ File ] [ File ] |
| | |
| | |
+---------+--------------------------------------------+
| Status Information |
+------------------------------------------------------+

Layout Requirements
Header bar at the top.
Sidebar on the left.
Main content area on the right.
Optional status information at the bottom.
Responsive resizing.
No unnecessary visual containers.

The main file area must receive the majority of available screen space.

1. Header Bar

The header bar should remain compact.

Required Elements
Back button.
Forward button.
Location or breadcrumb navigation.
Search button or search field.
Main menu.
View mode controls where appropriate.
Design Rules
Use symbolic icons for toolbar actions.
Avoid large buttons.
Maintain consistent spacing.
Provide tooltips.
Use accessible button labels.
Disable unavailable navigation actions appropriately.

The header should not become overcrowded.

1. Sidebar

The sidebar should provide quick access to common locations.

Suggested Sections
Personal Locations
Home.
Desktop.
Documents.
Downloads.
Pictures.
Music.
Videos.
Bookmarks

User-defined favorite directories.

Devices
Mounted drives.
Removable storage.
Available volumes.
Other Locations
Network locations where supported.
Trash.
Sidebar Behavior
Support selecting locations.
Support expanding and collapsing sections.
Provide clear selected-state indication.
Allow adding and removing bookmarks.
Avoid unnecessary nested navigation.
Allow sidebar visibility to be toggled.

The sidebar should remain visually quiet.

1. File Display Area

The file display area is the most important part of the application.

It should provide two primary display modes.

8.1 Grid View

Grid view should display:

File or folder icon.
Filename.
Optional selection indicator.

Requirements:

Consistent icon sizes.
Consistent spacing.
Proper filename wrapping.
Efficient scrolling.
Clear selection state.
No unnecessary card decorations.
8.2 List View

List view should display:

Name.
Size.
Type.
Modified date.

Requirements:

Compact row height.
Clear column alignment.
Sorting indicators.
Efficient selection.
Smooth scrolling.
8.3 Selection

Selected items must have a clear visual state.

Support:

Single selection.
Multiple selection.
Range selection.
Select all.
Keyboard-based selection.

Selection must remain visually consistent in both display modes.

1. Icons and Assets

Use the GNOME Adwaita icon theme as the primary visual reference.

Reference repository:

<https://github.com/GNOME/adwaita-icon-theme>

Icon Guidelines
Use symbolic icons for actions.
Use appropriate full-color icons for files and folders.
Maintain consistent icon sizing.
Avoid mixing unrelated icon styles.
Avoid unnecessary decorative icons.
Icon Categories
Navigation
Go back.
Go forward.
Home.
Search.
Menu.
File Operations
Copy.
Cut.
Paste.
Rename.
Delete.
New folder.
Properties.
File Types
Folders.
Images.
Videos.
Audio.
Documents.
Archives.
Executables.
Unknown file types.
Icon Theme Compatibility
Respect the user's installed icon theme where possible.
Provide Adwaita as a fallback.
Use standard icon theme names.
Avoid requiring a complete bundled icon collection. 10. Context Menus

Context menus should be simple and relevant.

File Context Menu
Open.
Open with.
Cut.
Copy.
Rename.
Move to trash.
Properties.
Folder Context Menu
Open.
Open in new tab.
Open in new window.
Cut.
Copy.
Rename.
Move to trash.
Properties.
Context Menu Rules
Avoid excessive menu items.
Use separators only when necessary.
Use icons selectively.
Display destructive actions appropriately.
Disable actions that are not currently available. 11. Search Interface

Search must be easy to access.

Design Requirements
Search should be accessible through Ctrl + F.
Search input should be visually clear.
Provide an obvious way to clear search.
Display search progress when appropriate.
Display empty results clearly.
Allow returning to normal browsing quickly.

Search must never freeze the interface.

The search UI should distinguish between current-directory filtering and recursive searching where applicable.

 1. Tabs

Tabs should be minimal and unobtrusive.

Requirements:

Display the current location or a shortened title.
Provide a close button.
Support keyboard navigation.
Highlight the active tab.
Avoid unnecessarily large tab headers.
Preserve individual tab navigation state.

Do not allow tabs to consume excessive vertical space.

 1. Dialogs and Notifications

Dialogs should follow the application's visual style.

Required dialogs include:

Rename.
Create folder.
File conflict.
Delete confirmation.
Properties.
Preferences.
Notification Guidelines
Use non-intrusive notifications.
Avoid unnecessary popups.
Show meaningful operation progress.
Provide understandable error messages.
Do not repeatedly notify users about the same event.

Long-running operations should use a dedicated progress interface where appropriate.

 1. Animations and Transitions

Animations should be minimal and purposeful.

Allowed:

Subtle hover transitions.
Small selection transitions.
Lightweight menu transitions.
Simple progress animations.

Avoid:

Large animated backgrounds.
Heavy blur effects.
Complex particle effects.
Unnecessary parallax.
Continuous decorative animations.

Animations must respect reduced-motion preferences.

Performance always takes priority over visual effects.

 1. Interaction Guidelines

The application should support standard Linux desktop interaction patterns.

Mouse
Single click to select.
Double click to open.
Right click for context menu.
Drag and drop where appropriate.
Keyboard
Standard navigation shortcuts.
Keyboard selection.
Quick search.
Rename.
Copy, cut and paste.
Tab switching.
Interaction Feedback
Show hover states.
Show selected states.
Indicate disabled actions.
Display progress for long operations.
Provide clear error feedback.

Every interactive control must provide appropriate feedback.

 1. Empty States

Empty states should be simple and helpful.

Examples:

Empty Folder

Display:

A simple folder illustration or icon.
Text indicating that the folder is empty.
No Search Results

Display:

A clear no-results message.
An easy way to clear the search.
Unavailable Location

Display:

An understandable error message.
An option to retry or navigate elsewhere.

Avoid oversized illustrations and unnecessary decoration.

 1. Responsive Design

The interface should adapt to different window sizes.

Requirements:

Sidebar should adapt to available space.
File grid should adjust its column count.
Toolbar controls should remain accessible.
Long filenames should not break layouts.
List view should handle narrow windows.
Dialogs should fit smaller displays.

Do not assume users have large monitors.

 1. Performance-Oriented UI Rules

These rules are mandatory.

Use virtualized GTK list components for large directories.
Avoid unnecessary widget creation.
Avoid rebuilding the entire file view for small changes.
Update only changed items.
Generate thumbnails lazily.
Avoid excessive UI animations.
Avoid unnecessary CSS effects.
Avoid excessive signal connections.
Keep expensive work off the GTK main thread.
Release unused UI and image resources.
Avoid rendering hidden or unnecessary content.
Limit unnecessary metadata displayed in file rows.

The visual system must not undermine the application's performance goals.

 1. GTK4 Implementation Guidelines

Use GTK4 components appropriately.

Potential components include:

UI Element GTK4 Component
Main window GtkApplicationWindow
Header bar GtkHeaderBar
Sidebar GtkListView / GtkStack
File list GtkListView
Grid GtkGridView
Scroll area GtkScrolledWindow
Navigation stack GtkStack
Tabs GtkNotebook or custom tab UI
Dialogs GtkDialog / GtkWindow
Menus GtkPopoverMenu

Use GTK's built-in virtualization rather than manually creating thousands of widgets.

Use CSS only for styling and avoid turning GTK into a heavily customized web-like interface.

 1. Accessibility Requirements
    Keyboard accessibility.
    Screen reader-friendly labels.
    Adequate contrast.
    Visible focus indicators.
    Readable typography.
    Reduced-motion support.
    Accessible context menus.
    Clear action names.
    Consistent keyboard shortcuts.

Accessibility must be considered during implementation rather than added at the end.

 1. Design Consistency Rules for AI Agents

Any AI agent working on the UI must follow these rules:

Read this document before implementing UI components.
Maintain the established visual style.
Do not invent unrelated design systems.
Do not add unnecessary UI components.
Do not introduce excessive colors.
Do not use emoji as file icons.
Do not use oversized buttons.
Do not add decorative animations without approval.
Keep spacing and typography consistent.
Ensure both light and dark themes work.
Keep UI components modular.
Do not compromise performance for visual effects.
Do not redesign existing screens without an explicit request.
Test layouts at different window sizes. 22. Final Design Goal

The final interface should feel familiar to GNOME users while remaining lightweight and efficient.

It should provide:

A clean interface.
Fast navigation.
Comfortable file browsing.
Clear visual hierarchy.
Minimal visual noise.
Consistent interactions.
Excellent keyboard support.
Responsive behavior.

The interface should disappear into the user's workflow. Users should spend their time managing files, not fighting the application.
