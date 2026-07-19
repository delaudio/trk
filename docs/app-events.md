# Application Events And Actions

Salieri routes external input and runtime updates through a typed application
boundary before mutating `App` state.

## Flow

```text
terminal / playback / MIDI / browser / project / tasks / viewport
                              |
                              v
                           AppEvent
                              |
                        FIFO dispatcher
                              |
                              v
                           AppAction
                              |
                              v
                         App reducer methods
```

`AppEvent` describes something that arrived from outside mutable application
state. `AppAction` is the ordered operation selected for that event. The
dispatcher owns a FIFO queue and drains synchronously on the TUI thread.
Background workers publish typed task updates to this queue without mutating
application state directly.

An action may enqueue another event. Notifications use this path: a playback
stop action updates transport state, enqueues a notification, and the outer
dispatch loop applies that notification afterward. Re-entrant dispatch never
starts a second drain and therefore preserves arrival order.

## Routed Domains

- terminal key input;
- playback position, stop, audio failure, and MIDI output status;
- MIDI input packets and input failures;
- external sample browser completion;
- project load completion;
- task progress, completion results, failures, and cancellation;
- notifications;
- viewport refresh after input, resize, or UI tick.

Filesystem access, AI proposal generation, terminal suspension, and backend
polling remain outside reducer actions. Their owned results cross the boundary
as events. Pure viewport and navigation actions do not carry song mutations.

## Ordering And Async Work

The current queue is process-local, bounded by work produced during one drain,
and deterministic. Event tests cover FIFO ordering, re-entrant notification
delivery, and non-mutating input and viewport actions.

The [task runtime](tasks.md) gives every async result a stable task ID. Its state
machine rejects unknown, duplicate, and post-cancellation updates before they
reach reducers. Issue #139 can split broad terminal input into more granular
user intents and injectable side effects without replacing this event path with
an untyped global bus.
