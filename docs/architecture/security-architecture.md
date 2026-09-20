# Security Architecture

LocardX incorporates defense-in-depth principles across all operating layers.

---

## Security Domains

1. **Untrusted UI Layer**: React frontend runs inside the WebView sandbox. It has no direct access to filesystem or raw devices.
2. **IPC Gatekeeper**: Tauri IPC command handlers strictly validate all arguments using Serde deserialization.
3. **Privileged Core**: Rust application processes execute with necessary OS permissions (e.g., Windows SeManageVolumePrivilege or administrative access for raw drive access) but contain internal access controls.
4. **Hardware Interlocks**: Sanitization routines enforce drive identification checks to prevent wiping system disks.
