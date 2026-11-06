FILESYSTEM_ENGINE.md
1. Introduction

The Filesystem Engine is the core backend responsible for interacting with files, directories, storage devices and filesystem operations.

It must provide a reliable and efficient interface between the Linux filesystem and the rest of the application.

The engine must be independent of GTK and must not contain UI-specific logic.

Primary Objectives
Extremely fast directory enumeration.
Low memory consumption.
Efficient metadata retrieval.
Reliable file operations.
Incremental directory loading.
Efficient filesystem monitoring.
Safe handling of large directories.
Support for different Linux filesystems.
Support for local and future remote filesystems.
Technology
Component	Technology
Language	Rust
Filesystem abstraction	GIO
Linux filesystem integration	GIO / GLib
Async operations	GIO asynchronous APIs
Cancellation	GCancellable
Error handling	Rust Result
Optional indexing	SQLite

Important: GIO should be the primary filesystem abstraction because it provides Linux desktop integration and can support local and remote locations.

For performance-sensitive local filesystem operations, native Rust or Linux APIs may be considered later when benchmarking demonstrates a meaningful advantage.

2. Engine Architecture

The filesystem engine should be divided into independent modules.

text
filesystem/
│
├── mod.rs
│
├── scanner/
│   ├── mod.rs
│   ├── directory_scanner.rs
│   ├── scan_request.rs
│   ├── scan_result.rs
│   └── scan_controller.rs
│
├── entries/
│   ├── mod.rs
│   ├── file_entry.rs
│   ├── file_metadata.rs
│   └── file_type.rs
│
├── operations/
│   ├── mod.rs
│   ├── copy.rs
│   ├── move.rs
│   ├── rename.rs
│   ├── delete.rs
│   ├── create.rs
│   └── trash.rs
│
├── metadata/
│   ├── mod.rs
│   ├── metadata_loader.rs
│   └── metadata_attributes.rs
│
├── monitoring/
│   ├── mod.rs
│   ├── directory_monitor.rs
│   └── change_event.rs
│
├── search/
│   ├── mod.rs
│   ├── filename_search.rs
│   └── recursive_search.rs
│
├── permissions/
│   ├── mod.rs
│   └── permission_handler.rs
│
├── errors/
│   ├── mod.rs
│   └── filesystem_error.rs
│
└── utils/
    ├── mod.rs
    └── path_utils.rs


Modules should be created incrementally. Do not create empty files for every planned component.

3. Directory Scanner

The directory scanner is the most performance-critical component.

Its responsibility is to enumerate directory entries and provide information to the application as efficiently as possible.

3.1 Core Requirements
Open directories quickly.
Enumerate entries incrementally.
Support large directories.
Avoid unnecessary metadata queries.
Support cancellation.
Handle permission errors.
Handle missing directories.
Avoid blocking the GUI.
Avoid loading unnecessary file contents.
3.2 Incremental Scanning

Never require the entire directory to be scanned before displaying the first entries.

Use batch-based enumeration.

Suggested flow:

text
          NAVIGATION REQUEST
                  |
                  v
          Create Scan Request
                  |
                  v
          Start Enumeration
                  |
                  v
          Read Entry Batch
                  |
                  v
          Convert to FileEntry
                  |
                  v
          Send Batch to Controller
                  |
                  v
          Display Available Entries
                  |
                  v
          Read Next Batch
                  |
                  v
          End of Directory


The user should be able to start interacting with available entries while enumeration continues.

3.3 Batch Size

Initial configuration:

Parameter	Initial Value
Initial batch size	64 entries
Normal batch size	128 entries
Large directory batch size	256 entries
Maximum concurrent scans	2

These are starting values, not permanent limits. Benchmark different batch sizes on HDDs, SSDs, network mounts and large directories.

Avoid generating excessively large batches that increase memory usage or delay initial display.

3.4 Scanner Behavior

The scanner must:

Receive a directory location.
Create a cancellable scan request.
Open the directory asynchronously.
Enumerate entries in batches.
Convert results into lightweight file models.
Send batches to the application.
Continue until completion or cancellation.
Release enumerator resources.

Use GIO's File.enumerate_children_async() and FileEnumerator.next_files_async() where appropriate.

References:

<Link ref="turn848855search2" title="GIO asynchronous directory enumeration"/>

<Link ref="turn848855search7" title="GIO batch enumeration"/>

<Link ref="turn848855search0" title="Rust FileEnumerator API"/>

3.5 Scanner Cancellation

Every scan must be cancellable.

Cancellation should occur when:

The user navigates to another directory.
The tab is closed.
The application is shutting down.
A newer scan replaces an older request.

Use GIO's GCancellable.

Each scan should also have a unique request identifier.

Results from outdated requests must be discarded rather than inserted into the current directory view.

4. File Entry Model

File entries should remain lightweight.

The application must not retrieve and retain every possible metadata field for every file.

4.1 Basic FileEntry

Suggested conceptual model:

rust
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_directory: bool,
}


This is an illustrative starting model. The final implementation should also account for non-UTF-8 Linux filenames.

4.2 File Metadata

Metadata should be separated from the basic entry model.

Possible fields:

File size.
Modification time.
Creation time where available.
File type.
Permissions.
Owner.
MIME type.
Symbolic link information.

Metadata must be retrieved only when required.

4.3 Filename Handling

Linux filenames are not guaranteed to be valid UTF-8.

Therefore:

Preserve native filenames using OsString or equivalent.
Do not assume every filename can be represented by String.
Use safe display conversion for GTK labels.
Preserve the original filename for filesystem operations.
Never modify a filename merely because it cannot be displayed as UTF-8.

This is a mandatory correctness requirement.

5. Metadata Retrieval

Metadata retrieval can become a major performance bottleneck in directories containing thousands of files.

5.1 Metadata Strategy

Use a progressive metadata strategy.

Level 1: Basic Enumeration

Retrieve information needed for initial display:

Name.
File type.
Directory status.
Level 2: Visible Metadata

Retrieve additional information when needed:

Size.
Modification date.
MIME type.
Level 3: Detailed Metadata

Retrieve on demand:

Permissions.
Ownership.
Extended attributes.
Detailed properties.
5.2 Metadata Rules
Do not query every possible attribute.
Avoid repeated queries for the same file.
Reuse metadata returned by directory enumeration.
Cache metadata only when useful.
Invalidate cached metadata after relevant operations.
Handle metadata failures independently of directory scanning.

Avoid unnecessary metadata retrieval for files that are not currently visible when it would require additional filesystem calls.

6. File Operations Engine

All filesystem operations must execute independently of the UI.

6.1 Supported Operations

Initial operations:

Copy.
Move.
Rename.
Delete.
Create file.
Create directory.
Move to trash.
Restore from trash where supported.
6.2 Operation Architecture
text
             USER REQUEST
                  |
                  v
           Operation Manager
                  |
                  v
           Validate Request
                  |
                  v
          Execute Operation
                  |
          +-------+-------+
          |               |
          v               v
       Progress         Error
          |               |
          v               v
      UI Update       Error Handler
          |
          v
       Completion
          |
          v
    Refresh Affected Views

6.3 Copy

Requirements:

Support files.
Support directories.
Handle large files.
Handle destination conflicts.
Report progress.
Support cancellation where safe.
Handle insufficient storage.
Preserve appropriate metadata where supported.

Prefer GIO's file-copy facilities when suitable.

6.4 Move

Requirements:

Move files and directories.
Handle cross-filesystem moves.
Handle destination conflicts.
Report progress.
Refresh affected directories.

A move across filesystem boundaries may require copying and deleting rather than an atomic rename.

Never assume all moves are atomic.

6.5 Rename

Requirements:

Rename files.
Rename directories.
Handle invalid names.
Detect destination conflicts.
Handle permission errors.
Update affected directory views.
6.6 Delete

Provide two distinct operations:

Move to Trash

Use the desktop-integrated trash mechanism.
Preserve the ability to restore files where supported.

Permanent Delete

Require explicit confirmation.
Report failures.
Handle non-empty directories safely.

Never silently turn a failed trash operation into permanent deletion.

6.7 Create

Support:

New directory.
New empty file.

Validate names and report existing-name conflicts.

7. Operation Progress System

Every long-running operation should provide progress information.

Suggested model:

rust
pub struct OperationProgress {
    pub operation_id: u64,
    pub completed_bytes: u64,
    pub total_bytes: Option<u64>,
    pub completed_items: u64,
    pub total_items: Option<u64>,
}


The model is conceptual and may be extended.

Requirements
Support multiple simultaneous operations.
Provide stable operation identifiers.
Report progress without excessive updates.
Support cancellation where safe.
Report completion only after backend confirmation.
Handle unknown total sizes.
Report partial failures accurately.

Do not estimate progress as 100% simply because the operation has stopped reporting updates.

8. Filesystem Monitoring

The engine should detect external filesystem changes.

Examples:

Files created outside the application.
Files deleted externally.
Files renamed externally.
Files modified externally.
Mounted devices disconnected.
8.1 Technology

Use GIO FileMonitor where appropriate.

8.2 Requirements
Monitor the currently viewed directory.
Update the visible file list when changes occur.
Avoid unnecessary full rescans.
Debounce event bursts.
Handle monitor failures.
Release monitors when no longer needed.
8.3 Event Handling

Suggested events:

text
FilesystemChange
├── Created
├── Deleted
├── Changed
├── Renamed
├── Moved
└── MountChanged


Filesystem monitoring is advisory. Events can be missed, coalesced or unavailable on some filesystems.

Provide a refresh mechanism and use rescanning when necessary to restore consistency.

9. Symbolic Link Handling

Symbolic links require special attention.

Requirements:

Identify symbolic links.
Display appropriate icons.
Avoid unintended recursive traversal.
Handle broken symbolic links.
Distinguish link metadata from target metadata.
Respect GIO query flags when deciding whether to follow links.

The application must never assume a symbolic link points to a valid target.

Recursive operations must follow an explicit and documented link policy.

10. Permissions and Ownership

The engine must respect normal Linux permission rules.

Requirements:

Detect inaccessible directories.
Handle read-only filesystems.
Report permission errors.
Display permissions where requested.
Display ownership where accessible.
Avoid unnecessary privilege escalation.

The application must not automatically request root privileges to bypass filesystem permissions.

11. Path and Location Handling

Paths must be handled safely.

Requirements:

Support absolute paths.
Support relative paths where appropriate.
Support spaces.
Support Unicode.
Support non-UTF-8 filenames.
Handle symbolic links.
Handle root paths.
Handle missing paths.
Handle removable storage paths.

Use PathBuf for native local paths and gio::File as the general filesystem abstraction.

Do not use manual string concatenation to construct filesystem paths.

12. Search Integration

The filesystem engine should expose search capabilities to the search subsystem.

12.1 Current Directory Filtering

Use already-loaded entries whenever possible.

Do not perform additional filesystem operations for basic filtering.

12.2 Recursive Search

Requirements:

Run in background execution.
Support cancellation.
Avoid following symbolic links by default.
Handle inaccessible directories.
Avoid unlimited recursion depth.
Avoid unbounded result accumulation.
Support incremental result delivery.
12.3 Search Index

Optional future functionality.

SQLite indexing must remain separate from basic filesystem operations.

The file manager must work normally without a database.

13. Concurrency and Task Management

The filesystem engine must use controlled concurrency.

13.1 Main Thread

The GTK main thread must handle:

UI events.
Display updates.
Lightweight state changes.

No blocking filesystem operations should execute on this thread.

13.2 Background Tasks

Background execution handles:

Directory scanning.
Recursive search.
File operations.
Metadata retrieval.
Thumbnail requests.
13.3 Task Rules
Use bounded queues.
Avoid unlimited thread creation.
Support cancellation.
Avoid duplicate requests.
Prioritize interactive work.
Limit optional background tasks.
Release resources after completion.

Use GLib/GIO asynchronous facilities for GTK-integrated operations. Do not introduce a second async runtime without a demonstrated requirement.

14. Error Handling

Use explicit error handling.

Suggested categories:

rust
pub enum FilesystemError {
    NotFound,
    PermissionDenied,
    AlreadyExists,
    NotDirectory,
    IsDirectory,
    NotSupported,
    NoSpace,
    Cancelled,
    DeviceUnavailable,
    InvalidFilename,
    IoError(String),
}


This is an initial conceptual model. Preserve useful underlying GIO error information instead of reducing every error to a generic message.

Requirements
Never crash because an individual file cannot be accessed.
Do not silently ignore important failures.
Handle partial operation failures.
Provide useful error context.
Distinguish cancellation from failure.
Keep UI state consistent after errors.
15. Cache Integration

The filesystem engine may use the application's cache manager.

Cache categories:

Directory entries.
File metadata.
File icons.
Filesystem information.
Rules
Cache memory must be bounded.
Invalidate entries after file operations.
Monitor external changes.
Avoid stale data being presented as authoritative.
Do not cache everything by default.
Avoid unnecessary disk writes.

The cache must improve real-world navigation performance without increasing idle resource consumption unnecessarily.

16. Performance Requirements

Performance is the engine's highest engineering priority after correctness and data safety.

16.1 Directory Performance

Initial testing scenarios:

Directory Size	Expected Behavior
100 files	Immediate interactive display
1,000 files	Fast incremental display
10,000 files	Progressive loading
100,000 files	Responsive incremental loading

These are behavioral targets, not guaranteed timing commitments.

16.2 Memory
Do not load file contents to display filenames.
Avoid unnecessary metadata storage.
Use lightweight models.
Keep caches bounded.
Release completed scan resources.
Avoid duplicate entry collections.
16.3 CPU
Avoid unnecessary scanning.
Avoid repeated metadata retrieval.
Limit worker concurrency.
Avoid excessive sorting.
Avoid unnecessary filesystem polling.
16.4 Disk
Do not write temporary files unnecessarily.
Do not require a search database.
Avoid unnecessary cache persistence.
Reuse standard Linux cache directories.
17. Performance Benchmarking

Benchmark the engine independently from GTK rendering.

Measure:

Time to open a directory.
Time to first entry batch.
Time to complete enumeration.
Total CPU consumption.
Peak memory consumption.
Metadata retrieval overhead.
Search latency.
Cancellation latency.

Test under:

SSD.
HDD.
Network filesystem.
Large directory.
Deep directory hierarchy.
Slow removable storage.

Compare results with GNOME Files and Dolphin using equivalent directory contents and conditions.

Do not claim performance improvements without repeatable measurements.

18. Testing Requirements
18.1 Directory Scanner Tests
Empty directory.
Small directory.
Large directory.
Hidden files.
Unicode filenames.
Non-UTF-8 filenames.
Permission-denied directory.
Missing directory.
Directory changed during scanning.
18.2 File Operation Tests
Copy file.
Copy directory.
Move file.
Cross-filesystem move.
Rename.
Delete.
Trash.
Create directory.
Destination conflicts.
Insufficient storage.
Interrupted operations.
18.3 Monitoring Tests
External file creation.
External file deletion.
External rename.
External modification.
Device disconnection.
18.4 Safety Tests
Broken symbolic links.
Symbolic link loops.
Read-only filesystems.
Inaccessible directories.
Permission changes.
Concurrent filesystem modifications.

Use temporary test directories and never run destructive tests against real user directories.

19. Implementation Phases
Phase 1: Basic Filesystem Engine
FileEntry model.
Directory scanner.
Incremental enumeration.
Cancellation.
Error handling.

<badge color="success">First implementation milestone</badge>

Phase 2: Metadata
Basic metadata.
File type detection.
File size.
Modification date.
Metadata loading on demand.
Phase 3: File Operations
Create.
Rename.
Copy.
Move.
Trash.
Delete.
Phase 4: Monitoring
Directory monitoring.
External change detection.
Refresh behavior.
Monitor cleanup.
Phase 5: Advanced Filesystem Support
Recursive search.
Mounted volumes.
Remote locations.
Advanced permissions.
Additional filesystem integration.

Do not begin advanced features before the basic scanner and operation engine are reliable.

20. Rules for AI Coding Agents

Every AI coding agent working on the filesystem engine must follow these rules:

Read this document before modifying filesystem code.
Keep the filesystem engine independent of GTK.
Use safe Rust wherever possible.
Prefer GIO for desktop-integrated filesystem operations.
Never block the GTK main thread.
Implement incremental directory enumeration.
Support cancellation.
Handle non-UTF-8 filenames correctly.
Avoid unnecessary metadata queries.
Keep memory usage bounded.
Never follow symbolic links recursively without an explicit policy.
Never silently overwrite user data.
Handle filesystem errors explicitly.
Avoid unnecessary dependencies.
Write tests for filesystem behavior.
Benchmark performance-sensitive changes.
Do not implement advanced functionality prematurely.
Update this document when architectural decisions change.
21. Definition of Done

The filesystem engine's first stable version is complete when:

Directories can be opened.
Entries are displayed incrementally.
Large directories remain manageable.
Scanning can be cancelled.
Non-UTF-8 filenames are preserved.
Basic metadata is available.
Filesystem errors are handled safely.
GTK remains responsive.
Resource consumption is measurable.
Tests cover important filesystem edge cases.
22. Final Engineering Goal

The filesystem engine must be the foundation of the application's performance.

It should make opening directories, retrieving file information and managing files feel fast without unnecessary CPU, RAM or disk consumption.

Correctness and user data safety must never be sacrificed for benchmark improvements.

Build the filesystem engine to be efficient by design, not dependent on endless optimization afterward.
