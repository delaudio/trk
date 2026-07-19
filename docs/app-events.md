# Application Events And Actions

Salieri routes user intent and runtime updates through a typed application
boundary before mutating `App` state or invoking a side-effecting backend.

## Flow

```text
terminal / typed commands                 playback / MIDI / tasks / filesystem
           |                                             |
           v                                             v
       AppIntent                                    RuntimeEvent
           +--------------------+------------------------+
                                |
                                v
                       FIFO AppEvent dispatcher
                                |
                                v
                           AppAction
                                |
                                v
                         reducer methods
                                |
                                v
                           AppEffect
                                |
                                v
                    injectable effect executor
                                |
                                +------> RuntimeEvent
```

`AppIntent` describes what the user requested. It has domain-specific variants
for commands, tracker edits, navigation, transport, parameter changes, AI work,
and project opening. `RuntimeEvent` describes a typed result or status update
from a backend. `AppEvent` is only the closed wrapper used by the dispatcher;
it is not an open-ended message bus.

`AppAction` is the ordered reducer operation selected for an event. Reducers
may update owned state and return `AppEffect` values, but they do not call the
playback backend, filesystem project loader, or AI task backend directly. The
runtime effect executor owns those calls. Tests substitute a recording
executor to exercise reducers without running those backends.

An action or effect may enqueue another event. Notifications use this path: a
playback stop intent updates transport state, enqueues a notification, emits a
stop effect, and the outer dispatch loop applies the notification afterward.
Re-entrant dispatch never starts a second drain and therefore preserves FIFO
arrival order.

## Routed Domains

- terminal key input and parsed commands;
- tracker edit, navigation, transport, and parameter intents;
- playback position, stop, audio failure, and MIDI output status;
- MIDI input packets and input failures;
- external sample browser completion;
- project load completion;
- task progress, completion results, failures, and cancellation;
- notifications;
- viewport refresh after input, resize, or UI tick.

Filesystem project loading, AI proposal submission, and playback/MIDI commands
are explicit effects. Terminal suspension and backend polling remain runner
responsibilities; their owned results cross the boundary as runtime events.
Pure viewport and navigation actions do not carry song mutations.

## Ordering And Async Work

The current queue is process-local, bounded by work produced during one drain,
and deterministic. Event tests cover FIFO ordering, re-entrant notification
delivery, representative domain intents, and non-mutating viewport actions.

The [task runtime](tasks.md) gives every async result a stable task ID. Its state
machine rejects unknown, duplicate, and post-cancellation updates before they
reach reducers. Filesystem requests use the same rule: each project load has a
monotonic request ID, and a completion is applied only when it matches the
latest pending request. Older results are ignored without changing project
state.
