FILE_OPERATIONS.md

1. Introduction

This document defines the behavior, architecture and implementation requirements for all file operations in the Linux file manager.

The file operations engine is responsible for safely managing files and directories while maintaining excellent performance and low resource consumption.

It must support common file management operations without blocking the user interface.

Primary Objectives
Fast file operations.
Reliable data handling.
Low memory consumption.
Background execution.
Progress reporting.
Cancellation support.
Safe conflict handling.
Correct filesystem behavior.
Minimal unnecessary disk I/O.
Technology
Component Technology
Language Rust
Filesystem API GIO
Async integration GLib / GIO
Cancellation GCancellable
Error handling Rust Result
Progress reporting Application event system 2. File Operations Architecture

File operations must be independent of the GUI.

Suggested directory structure:

text
src/
└── filesystem/
└── operations/
├── mod.rs
├── manager.rs
├── operation.rs
├── operation_id.rs
├── copy.rs
├── move.rs
├── rename.rs
├── delete.rs
├── create.rs
├── trash.rs
├── conflict.rs
├── progress.rs
├── cancellation.rs
├── queue.rs
└── error.rs

Each module should have a clearly defined responsibility.

Do not create unnecessary abstraction layers.

1. Supported Operations

The initial implementation must support:

Operation Priority
Copy files P0
Copy directories P0
Move files P0
Move directories P0
Rename files P0
Rename directories P0
Create directories P0
Create empty files P1
Move to trash P0
Permanent deletion P1
Restore from trash P2
Bulk rename P3

P0 operations are required for the first usable release.

1. Operation Manager

The operation manager coordinates all filesystem operations.

Responsibilities:

Accept operation requests.
Validate requests.
Assign unique operation IDs.
Schedule operations.
Track operation state.
Manage cancellation.
Report progress.
Report completion.
Report errors.
Notify affected directory views.

The manager must not contain GTK widget logic.

4.1 Operation Lifecycle

Every operation should follow this lifecycle:

text
REQUESTED
|
v
VALIDATE
|
v
QUEUED
|
v
RUNNING
|
+----+-----+
| |
v v
PROGRESS CANCELLING
| |
v v
RUNNING CANCELLED
|
+----------+
| |
v v
COMPLETED FAILED

The operation manager must maintain accurate state transitions.

An operation must never be marked completed before the backend confirms success.

4.2 Operation States
rust
# [derive(Debug, Clone, PartialEq)]
pub enum OperationState {
Pending,
Queued,
Running,
Cancelling,
Completed,
Cancelled,
Failed,
}

The final implementation may extend this model to represent partial completion.

1. Copy Operation

Copying is one of the most important file manager operations.

5.1 Requirements
Copy individual files.
Copy multiple files.
Copy directories recursively.
Preserve appropriate file attributes where supported.
Handle large files.
Handle destination conflicts.
Report progress.
Support cancellation.
Handle insufficient storage.
Handle permission failures.
Avoid loading entire file contents into RAM.
5.2 Copy Workflow
text
COPY REQUEST
|
v
Validate Sources
|
v
Validate Destination
|
v
Check Name Conflicts
|
v
Execute Copy
|
v
Report Progress
|
v
Verify Completion
|
v
Update Directory

5.3 Copy Implementation

Use GIO's asynchronous copy functionality where appropriate.

For individual files, use:

GFile.copy_async()
GFile.copy_finish()

For directories, implement controlled recursive traversal.

Important: GIO's basic file-copy operation does not recursively copy directory trees by itself.

Directory copying must therefore be implemented as a separate orchestration operation.

Reference: <https://docs.gtk.org/gio/iface.File.html>

5.4 Copy Rules
Never overwrite files silently.
Avoid unnecessary temporary copies.
Do not load complete file contents into memory.
Limit concurrent copy operations.
Preserve file attributes where possible.
Report partial failures.
Do not follow symbolic links recursively by default. 6. Move Operation
6.1 Requirements
Move individual files.
Move directories.
Move multiple items.
Support same-filesystem moves.
Support cross-filesystem moves.
Handle conflicts.
Report progress.
Support cancellation where safe.
6.2 Move Strategy

Use the following strategy:

Attempt a native move where supported.
Detect whether the operation requires cross-filesystem handling.
Use copy-and-delete fallback when necessary.
Preserve the source until the destination copy is safely completed.
Delete the source only after successful copying.
Report failures without silently losing data.

GIO's move operation can use native filesystem moves or copy-and-delete fallback behavior. However, directory support for fallback operations must be handled explicitly.

Reference: <https://docs.gtk.org/gio/method.File.move.html>

6.3 Atomicity

A move operation is not guaranteed to be atomic.

The application must distinguish between:

Native filesystem rename.
Cross-filesystem copy-and-delete.
Partially completed operations.

Never report an atomic move when the operation actually used a copy-and-delete process.

1. Rename Operation
   7.1 Requirements
   Rename files.
   Rename directories.
   Support Unicode filenames.
   Preserve file extensions unless explicitly changed.
   Detect destination conflicts.
   Handle invalid filenames.
   Handle permission errors.
   7.2 Rename Workflow
   text
   RENAME REQUEST
   |
   v
   Validate Name
   |
   v
   Check Destination
   |
   v
   Execute Rename
   |
   v
   Update File View

7.3 Rename Rules
Do not allow an empty filename.
Do not silently overwrite an existing file.
Preserve the original filename until success.
Handle native Linux filename encoding.
Refresh affected directory entries. 8. Delete Operation

Deletion must distinguish between reversible and permanent actions.

8.1 Move to Trash

This should be the default deletion action.

Requirements:

Use GIO trash functionality.
Allow recovery through the desktop trash system where supported.
Report unsupported trash locations.
Handle permission errors.
Update the directory after success.

Use:

GFile.trash_async()
GFile.trash_finish()

Reference: <https://docs.gtk.org/gio/method.File.trash.html>

8.2 Permanent Delete

Permanent deletion must require explicit user confirmation.

Requirements:

Clearly identify permanent deletion.
Handle non-empty directories recursively.
Avoid following symbolic links unintentionally.
Report individual failures.
Support cancellation where safe.

GIO's basic delete operation only deletes a directory when it is empty. Recursive deletion must be implemented separately.

8.3 Deletion Safety
Never permanently delete files through the normal Trash action.
Never silently convert a failed Trash operation into permanent deletion.
Never silently skip files.
Report partial deletion accurately.
Handle concurrent filesystem changes. 9. Create Operations
9.1 Create Directory

Requirements:

Accept directory name.
Validate filename.
Check destination conflicts.
Create directory.
Report success or failure.

Use GIO directory creation functionality.

9.2 Create File

Requirements:

Create empty files.
Reject existing destinations unless an explicit supported action is selected.
Handle invalid filenames.
Handle permission errors.

Never overwrite existing files during normal creation.

 1. File Conflict Handling

File conflicts occur when a destination already contains an item with the same name.

The application must provide clear choices.

10.1 Conflict Options
Skip.
Replace.
Keep both.
Cancel operation.
10.2 Conflict Rules

Skip

Leave the existing destination unchanged.
Continue with other items where appropriate.

Replace

Require explicit confirmation.
Clearly identify the existing destination.
Do not silently destroy existing data.

Keep Both

Generate a unique destination filename.
Preserve the original file extension.
Check that the generated name does not already exist.

Cancel

Stop the operation.
Preserve already completed results.
Report the operation's partial state.
10.3 Multiple Conflicts

For bulk operations, provide:

Handle individually.
Apply the selected action to all remaining conflicts.

Never assume that two files are identical merely because their names match.

 1. Progress Reporting

Long-running operations must provide progress information.

11.1 Progress Model

Suggested structure:

rust
# [derive(Debug, Clone)]
pub struct OperationProgress {
pub operation_id: u64,
pub completed_items: u64,
pub total_items: Option<u64>,
pub completed_bytes: u64,
pub total_bytes: Option<u64>,
}

11.2 Progress Information

Display:

Operation type.
Source.
Destination.
Completed items.
Total items when known.
Completed bytes.
Total bytes when known.
Current state.
11.3 Progress Rules
Do not update the UI for every individual byte.
Throttle progress notifications.
Keep the main UI thread responsive.
Support unknown total sizes.
Distinguish item progress from byte progress.
Handle operations with many small files.
Display partial completion accurately.

Progress updates should be optimized to avoid excessive UI refreshes.

 1. Cancellation System

Operations should support cancellation where technically safe.

12.1 Requirements
Cancel queued operations immediately.
Cancel running operations when supported.
Use GCancellable for GIO asynchronous operations.
Update operation state.
Clean up temporary resources.
Report partial completion.
Never claim cancellation means all changes were rolled back.
12.2 Cancellation Rules

Cancellation is not the same as rollback.

For example, a directory copy may already have created several destination files before cancellation.

The application must report what was completed and what remains incomplete.

 1. Operation Queue

The operation manager should use a controlled task queue.

Requirements:

Support multiple operations.
Avoid unlimited concurrency.
Preserve dependencies between related operations.
Avoid duplicate operations.
Prioritize interactive tasks appropriately.
Support queued-operation cancellation.

Initial concurrency configuration:

Operation Suggested concurrency
Directory scanning 2
Large file copying 2
Metadata retrieval 4
Thumbnail generation 2

These are initial benchmark settings, not fixed limits.

Avoid running many large copy operations simultaneously if doing so causes unnecessary disk contention.

 1. Filesystem Safety

Filesystem safety is mandatory.

14.1 Symbolic Links
Detect symbolic links.
Do not follow them recursively by default.
Avoid symbolic link loops.
Preserve links where appropriate.
Distinguish link operations from target operations.
14.2 Concurrent Modifications

Files may change while an operation is running.

The engine must handle:

Source files disappearing.
Destination files appearing.
Permissions changing.
Storage becoming unavailable.
Files being modified externally.

Revalidate important assumptions immediately before destructive actions.

14.3 Destination Safety
Validate destination paths.
Detect conflicts.
Avoid accidental overwrites.
Handle read-only destinations.
Handle insufficient storage.
Handle mount disconnection.
14.4 Data Integrity
Never delete a source after an unsuccessful copy.
Never report success after partial failure.
Preserve useful error information.
Avoid silently discarding user data.
Clearly communicate incomplete operations. 15. Directory Refresh Integration

After successful file operations, affected directories must be updated.

Examples:

Operation Directories to Update
Copy Destination
Move Source and destination
Rename Parent directory
Trash Original parent
Delete Original parent
Create folder Destination parent
Create file Destination parent

Prefer targeted updates rather than rescanning the entire directory after every operation.

If filesystem monitoring already reports the change, avoid applying duplicate UI updates.

 1. Undo and Redo

Undo and redo are future capabilities.

Possible initial support:

Undo rename.
Undo move to trash.
Undo supported move operations.
Redo supported reversible operations.

Permanent deletion must not be presented as undoable.

The operation engine should keep enough information to support future undo functionality, but should not implement a complex transaction framework prematurely.

 1. Performance Requirements

Performance is important, but data integrity always takes priority.

RAM
Stream large file copies.
Avoid loading complete files into memory.
Keep progress structures lightweight.
Limit concurrent operations.
Release completed operation state.
CPU
Avoid excessive parallelism.
Avoid unnecessary checksum calculations.
Avoid repeated metadata queries.
Throttle progress processing.
Disk
Avoid unnecessary temporary copies.
Prefer native moves when possible.
Avoid unnecessary synchronization and metadata writes.
Do not create persistent logs containing file contents. 18. Error Handling

Suggested error categories:

rust
pub enum OperationError {
SourceNotFound,
DestinationNotFound,
PermissionDenied,
AlreadyExists,
InvalidFilename,
NotEnoughSpace,
UnsupportedOperation,
DeviceUnavailable,
Cancelled,
PartialFailure,
IoError(String),
}

The actual implementation should preserve underlying GIO errors and provide contextual information.

Errors must distinguish:

Operation failure.
User cancellation.
Partial completion.
Destination conflict.
Unsupported filesystem behavior.

Do not reduce every failure to a generic error message.

 1. Testing Requirements

All operations must have automated tests.

Copy Tests
Copy small file.
Copy large file.
Copy multiple files.
Copy directory.
Copy empty directory.
Copy symbolic link.
Copy into existing destination.
Copy to read-only destination.
Cancel during copying.
Move Tests
Same-filesystem move.
Cross-filesystem move.
Move directory.
Move multiple files.
Destination conflict.
Device disconnection.
Rename Tests
Rename file.
Rename directory.
Unicode filename.
Non-UTF-8 filename.
Invalid filename.
Existing destination.
Delete Tests
Move to trash.
Trash unsupported.
Delete empty file.
Delete empty directory.
Recursive deletion.
Permission failure.
Cancellation.
Reliability Tests
Insufficient storage.
Read-only filesystem.
Concurrent modifications.
Broken symbolic links.
Missing source.
Missing destination.
Interrupted operation.

All destructive tests must use temporary directories and isolated test fixtures.

 1. Implementation Phases
    Phase 1: Basic Operations
    Operation manager.
    Create directory.
    Rename.
    Copy single file.
    Move single file.
    Move to trash.
    Error handling.
    Phase 2: Advanced Operations
    Recursive directory copy.
    Multiple file operations.
    Permanent deletion.
    Conflict handling.
    Progress reporting.
    Phase 3: Reliability
    Cancellation.
    Partial failure handling.
    Device disconnection.
    Concurrent filesystem changes.
    Resource management.
    Phase 4: Advanced Features
    Undo and redo.
    Bulk rename.
    Advanced conflict resolution.
    Operation history.

Do not implement all phases at once.

 1. Rules for AI Coding Agents

Every AI coding agent must follow these rules:

Read this document before implementing file operations.
Keep operations independent of GTK.
Use GIO asynchronous operations where appropriate.
Never block the GTK main thread.
Never silently overwrite existing files.
Never permanently delete files through the normal Trash action.
Support cancellation where safe.
Report partial failures accurately.
Never delete a source before successful cross-filesystem copying.
Handle symbolic links carefully.
Preserve native filenames.
Avoid unnecessary memory allocations.
Limit operation concurrency.
Do not report completion before backend confirmation.
Do not introduce unnecessary dependencies.
Test destructive operations using temporary directories.
Never sacrifice data safety for performance.
Update documentation when operation behavior changes. 22. Definition of Done

The initial file operations engine is complete when:

Files can be copied.
Directories can be copied recursively.
Files can be moved.
Directories can be moved.
Files can be renamed.
Directories can be renamed.
New folders can be created.
Files can be moved to Trash.
Progress can be reported.
Errors are handled properly.
Destination conflicts are handled safely.
Cancellation works where supported.
The UI remains responsive.
Automated tests cover important failure cases. 23. Final Engineering Goal

The file operations engine must provide the reliability users expect from a desktop file manager without unnecessary resource consumption.

Every operation should be predictable, safe, responsive and efficient.

Fast operations are important, but protecting user data is more important.

Never sacrifice data integrity for speed.
