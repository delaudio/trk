# Background Tasks

`salieri-app` owns a small runtime for work that must not block terminal input
or rendering. A backend receives jobs, while `TaskRuntime` owns the stable task
IDs and user-visible state.

## Lifecycle

Tasks move through these states:

```text
queued -> running -> completed
                  -> failed
queued/running -> cancelling -> cancelled
```

Every update carries its task ID. The runtime accepts updates only when they are
valid for the current state, so unknown updates, duplicate terminal events, and
late completion or progress after cancellation are ignored. Failures retain
structured diagnostics. Cancellation is idempotent and cooperative: the thread
backend marks the task cancelled immediately and also sets a flag that jobs can
check between units of work.

The production backend runs each submitted job on a worker thread and publishes
`Started`, `Progress`, `Completed`, `Failed`, or `Cancelled` updates over a
channel. Tests use the deterministic fake backend to script the same update
stream without starting work.

## Application Boundary

The TUI loop drains runtime updates without waiting for jobs. Accepted updates
become `AppEvent::Runtime(RuntimeEvent::TaskUpdate(...))` values and pass through
the normal FIFO dispatcher before they can mutate application state. Typed
results are represented by `AppTaskResult`, keeping worker code independent
from mutable `App` state.

The first integrated operation is `:ai propose PROMPT`. Its generation and
preview validation run in the background, and only the prepared proposal crosses
back to the app thread.

Use `:tasks` to show recent queued, active, completed, failed, and cancelled jobs.
Use `:task cancel ID` to request cancellation. The footer displays the most
recent active task and its progress percentage when available.
