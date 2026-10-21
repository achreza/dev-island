# Dynamic Island Extended Developer Features — Implementation Roadmap

This document outlines the ordered implementation plan for the 5 extended developer productivity features in the Rust Windows Dynamic Island.

---

## 🧭 Ordered Implementation Phases

```text
Phase 4: Git Status Monitor (Feature 1)
   └── Read local Git HEAD, branch name, dirty file count without spawning heavy subshells.
   └── Shows branch badge in pill and expanded diff overview.

Phase 5: Active Dev Ports & Process Killer (Feature 2)
   └── Query listening TCP ports (3000, 5173, 8080, 8000, etc.) via Win32 IP Helper API (`GetExtendedTcpTable`).
   └── One-click process termination (`OpenProcess` + `TerminateProcess`) for stuck development servers.

Phase 6: Dev Command Runner & Build Status (Feature 3)
   └── Run non-blocking background developer commands (`cargo check`, `npm test`, etc.).
   └── Dynamic pill animation / status badge for in-progress, success, and error states.

Phase 7: Developer Scratchpad & Utilities (Feature 4)
   └── Quick JSON pretty-printer/validator and Base64 encoder/decoder.
   └── Single-click copy formatted output to Windows clipboard.

Phase 8: Compile & System Resource Monitor (Feature 5)
   └── Query CPU and RAM usage via Win32 `GetSystemTimes` and `GlobalMemoryStatusEx`.
   └── On-demand polling only when expanded or when a build is active; 0.0% CPU when idle.
```

---

## 1. Feature 1: Git Status Monitor

### Objectives
* Inspect a configured local repository directory (e.g. current project or configured path).
* Read `.git/HEAD` and `.git/refs/heads/` or run a lightweight status check.
* Display:
  * Active Git branch (e.g. `main`, `feat/phase-2`).
  * Uncommitted changes count: modified (`M`), untracked (`?`), staged (`+`).
  * Live status chip in expanded Dynamic Island and optional branch indicator in the collapsed pill.
* Zero continuous disk thrashing: only poll on expand or with a gentle cache interval (e.g. 5–10s when active).

---

## 2. Feature 2: Active Dev Ports & Process Killer

### Objectives
* Identify running local server ports commonly used by web/backend developers:
  * `3000`, `3001` (React, Next.js)
  * `5173` (Vite)
  * `8000`, `8080`, `8081` (FastAPI, Spring, Flutter web, Laragon)
  * `4200` (Angular)
  * Configurable custom port list.
* Implementation:
  * Uses Windows IP Helper API: `GetExtendedTcpTable` from `iphlpapi.dll`.
  * Finds the owner `PID` for each listening port.
  * Resolves process name (e.g. `node.exe`, `python.exe`, `cargo.exe`).
  * UI allows 1-click **Kill Process** (`TerminateProcess`) to immediately free locked ports.

---

## 3. Feature 3: Dev Command Runner & Build Status

### Objectives
* Allow running configured tasks from the Dynamic Island without opening a new terminal window:
  * `cargo check`
  * `npm test`
  * Custom build script
* Asynchronous execution: runs in a background worker thread (`std::thread` / channel) without blocking the Win32 UI message loop.
* Dynamic Island states:
  * In-progress: Orange animated pulsing dot.
  * Passed: Green checkmark badge `✓ Build passed`.
  * Failed: Red alert badge `✗ Build error` with exit code.

---

## 4. Feature 4: Developer Scratchpad & Utilities — ✅ COMPLETED
* Implementation: [`src/features/utilities.rs`](file:///d:/Flutter%20Project/dynamic_island_dev/src/features/utilities.rs)
* Features:
  * **JSON Formatter & Validator:** pretty-prints compact/unformatted JSON with 2 spaces or minifies.
  * **JWT Inspector:** base64url-decodes token header & claims payload, parses JSON, and displays human-readable expiration time.
  * **UUID v4 Generator:** native RFC 4122 UUID v4 generation using Windows COM `CoCreateGuid`.
  * **Screen Hex Color Picker:** samples pixel color under mouse cursor across all monitors via Win32 GDI `GetPixel` and copies `#RRGGBB` to clipboard.
* UI Integration: 3 micro-tool action chips (`🎨 Hex`, `🆔 UUID`, `⚡ Format`) in expanded Dynamic Island bottom row.
* Unit tests: 4 dedicated tests in `utilities.rs` validating formatting, JWT parsing, UUID v4 format, and color math. All 21 tests passing.

---

## 5. Feature 5: Compile & System Resource Monitor

### Objectives
* Lightweight CPU & RAM load indicator.
* Native Win32 queries:
  * CPU: `GetSystemTimes` (idle vs kernel/user time delta).
  * RAM: `GlobalMemoryStatusEx` (`ullAvailPhys` vs `ullTotalPhys`).
* **Performance Budget:**
  * Only active when the island is expanded or when a background command runner task is running.
  * Completely dormant when collapsed to ensure 0.0% steady-state idle CPU.

---

## 📋 Execution Order
Features will be implemented strictly in order from **Feature 1** to **Feature 5**, with tests and benchmarks updated after each milestone.
