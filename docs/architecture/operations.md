# Operation Manager Architecture & Lifecycle Specification

## 1. Architectural Role

The `locardx-operation-manager` crate serves as the central orchestration and lifecycle management layer for all background forensic and data sanitization tasks within LocardX.

Its primary responsibilities are:
- **Operation Identity**: Assigning globally unique UUIDs to operations.
- **Target Specification**: Enclosing explicit target descriptions (`TargetIdentity`) containing path/identifier, target type, size, and display name.
- **Lifecycle State Machine**: Enforcing strictly valid state transitions across defined lifecycle phases.
- **Cooperative Cancellation**: Providing atomic cancellation tokens checked at non-destructive checkpoints.
- **Progress Telemetry**: Standardizing progress metrics (bounded percentage, bytes processed/total, throughput, ETA).
- **Persistent Audit Correlation**: Maintaining operations in SQLite with full correlation to the tamper-evident SHA-256 audit hash-chain.

---

## 2. Invariants & Scope Boundaries

### Strict Executability Boundaries
Only non-destructive, read-only integrity operations are executable:
- `IntegrityHash`: Read-only cryptographic hashing (SHA-256) of target files using streaming chunks.
- `IntegrityVerify`: Read-only integrity verification against reference digests using constant-time comparisons.

All other operation types remain permanently disabled and return immediate execution errors:
- `FileErasure`: Disabled (Returns `Err(DisabledOperation)`)
- `FolderErasure`: Disabled (Returns `Err(DisabledOperation)`)
- `DriveErasure`: Disabled (Returns `Err(DisabledOperation)`)
- `Recovery`: Disabled (Returns `Err(DisabledOperation)`)

### Target Identification is Not Authorization
Identifying a target path or volume does NOT constitute permission or authority to modify or destroy it. The Operation Manager manages metadata and lifecycle execution; actual access controls, write-blocking checks, and safety guardrails remain enforced by `locardx-security` and `locardx-auth`.

---

## 3. Operation Lifecycle State Machine

Each operation moves through a strictly validated finite state machine. Transitions outside the allowed directed graph are rejected with error.

```
       ┌───────────┐
       │  Created  │
       └─────┬─────┘
             │
             ▼
       ┌───────────┐
       │  Queued   │
       └─────┬─────┘
             │
             ▼
       ┌───────────┐
 ┌────►│  Running  │◄────┐
 │     └─────┬─────┘     │
 │           │           │
 │ (Resume)  ▼ (Cancel)  │ (Pause)
 │     ┌───────────┐     │
 └─────┤  Paused   ├─────┘
       └─────┬─────┘
             │
       ┌─────┴─────┐
       ▼           ▼
 ┌───────────┐ ┌───────────┐
 │ Cancelling│ │  Failed   │
 └─────┬─────┘ └───────────┘
       │
       ▼
 ┌───────────┐ ┌───────────┐
 │ Cancelled │ │ Completed │
 └───────────┘ └───────────┘
```

### State Machine Transition Rules:
| Current State | Valid Next States |
| :--- | :--- |
| `Created` | `Queued`, `Running`, `Cancelled`, `Failed` |
| `Queued` | `Running`, `Cancelled`, `Failed` |
| `Running` | `Paused`, `Cancelling`, `Completed`, `Failed` |
| `Paused` | `Running`, `Cancelling`, `Failed` |
| `Cancelling`| `Cancelled`, `Failed` |
| `Completed`| *(Terminal State)* |
| `Failed` | *(Terminal State)* |
| `Cancelled`| *(Terminal State)* |

---

## 4. Cooperative Cancellation Protocol

LocardX guarantees that background operations do not terminate unpredictably or leave resources locked. Operations are cancelled cooperatively:
1. Operator requests cancellation via `cancel_operation(operation_id)`.
2. The Operation Manager sets the atomic flag on the operation's `CancellationToken`.
3. The operation state immediately transitions to `Cancelling`.
4. The background task checks `cancellation_token.is_cancelled()` at safe, chunk-boundary intervals.
5. Upon detecting cancellation, the executor cleans up volatile state, records an `OPERATION_CANCELLED` audit event, and transitions the operation to terminal `Cancelled`.

---

## 5. Persistence & In-Memory Synchronization

- **SQLite Store**: Operations are persisted in the `operations` table (`crates/database/migrations/004_operations.sql`). State changes, progress percentage, error messages, and completion summaries are written to the database.
- **In-Memory Caching**: Active cancellation tokens and real-time progress updates are maintained in thread-safe concurrent memory (`Arc<RwLock<...>>`) to prevent disk I/O bottlenecks during high-frequency telemetry loops.
- **Audit Hash-Chain**: Every major lifecycle milestone (`OPERATION_CREATED`, `OPERATION_STARTED`, `OPERATION_CANCELLATION_REQUESTED`, `OPERATION_CANCELLED`, `OPERATION_COMPLETED`, `OPERATION_FAILED`) is logged to the append-only SHA-256 hash chain via `AuditService`.
