# System Architecture

LocardX is constructed as a modern desktop application that couples a high-performance, memory-safe Rust backend with a reactive TypeScript frontend through the Tauri desktop application framework.

---

## High-Level Topology

```text
┌────────────────────────────────────────────────────────┐
│               Frontend (React / Vite)                  │
│  - User Interface & Dashboard                          │
│  - Device Selection & Configuration                    │
│  - Progress Visualizers & Real-time Telemetry          │
│  - Report Viewers (Forensic, Sanitization, Audit)      │
└───────────────────────────┬────────────────────────────┘
                            │ Tauri IPC (Command / Event)
┌───────────────────────────▼────────────────────────────┐
│               Tauri Desktop Core Layer                 │
│  - Window Management & System Tray                     │
│  - OS Privilege Elevation Interface                    │
│  - IPC Routing & Payload Validation                    │
└───────────────────────────┬────────────────────────────┘
                            │ Tokio Runtime Channels
┌───────────────────────────▼────────────────────────────┐
│               Rust Application Backend                 │
│  - Domain Engines (Drive Eraser, File Eraser, Carver)  │
│  - Job Orchestrator & Operation Manager                │
│  - Tamper-Evident Hash Chain Audit Engine              │
│  - SQLite Database & Migrations Engine                 │
│  - OS Storage Interface & Low-level Disk Drivers       │
└────────────────────────────────────────────────────────┘
```

---

## Architectural Principles

1. **Safety Separation**: The UI never directly issues raw disk write calls. All destructive actions pass through the validation gate in `crates/security` and the `crates/operation-manager` job scheduler.
2. **Cooperative Cancellation**: Background jobs periodically poll an `AtomicBool` or Tokio cancellation token, enabling safe and immediate aborts without leaving storage in an indeterminate state.
3. **Audit Immutable Record**: Any operation that mutates state, erases data, or exports evidence produces an append-only hash-chained record.
