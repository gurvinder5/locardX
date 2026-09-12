# LocardX Secure Drive Eraser (Pillar 1)

This document provides documentation for LocardX physical drive sanitization.

- For the comprehensive Step 10A architectural specification, capability detection engine, TOCTOU safety interlocks, and simulation harness, see [Drive Eraser Architecture](drive-eraser-architecture.md).
- For file and directory logical erasure documentation, see [File & Folder Erasure](file-folder-erasure.md).
- For verification engine details, see [Verification Engine](verification.md).

## Critical Safety Invariant
Real hardware drive erasure is strictly disabled in Step 10A. All operations execute through the verified simulation layer.
