# LocardX Testing Overview

Testing is central to LocardX because the application executes both data-destructive routines (drive/file sanitization) and forensically sensitive routines (evidence carving).

---

## Testing Principles

1. **Physical Drive Protection**:
   - Never run integration tests or benchmarks against actual host disk devices (`/dev/sd*`, `\\\\.\\PhysicalDrive*`).
   - Mock devices and virtual disk files must always be used.
2. **Deterministic Test Data**:
   - Use synthetic filesystem images (`test-data/filesystem-images/`) with known byte patterns and known file structures.
3. **Evidence Non-Modification**:
   - Tests validating file carving must verify that the SHA-256 hash of the source disk image remains strictly identical before and after the operation.

---

## Test Categories

### 1. Unit Tests (`cargo test --workspace`)
- Test individual algorithmic functions (hashing, signature detection, pattern generators, state machines) in isolation.
- Located within crate `src/` modules or crate `tests/` folders.

### 2. Integration Tests (`tests/integration/`)
- Cross-crate workflows (e.g., executing a mock sanitization job, tracking progress through `operation-manager`, and asserting audit events in `audit`).

### 3. Frontend Tests (`cd frontend && npm test`)
- Component rendering, store state updates (Zustand), input validation, and dialog interlocks.
