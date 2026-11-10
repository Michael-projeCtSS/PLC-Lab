# ADR-0048: Ordered workspace requests and push-button holds

- Status: Accepted for 0.2.0-dev
- Date: 2026-11-10

## Context

The interface changes a workspace through IPC use cases: toolbar and inspector commands
(PLC mode, Simulation Run/Pause/Step/Stop, Build & Load, check, forcing, restart) and
operator gestures in the process view (push-button press and release, selector, target
presence, door). The desktop host runs each request on its own blocking worker, so two
requests sent at the same time can be applied in either order.

Until now every use case went through one rule: while a request was pending, a new one was
ignored. That rule is right for commands; a double-clicked Step must advance one cycle.
It was wrong for gestures. A push-button release made while any other request was in
flight was never sent, and the simulated button stayed pressed. A press could be lost the
same way: Windows CI dropped Start S1 right after the Hold S3 release in the native
sequence session. The process view also lost releases outright. Push buttons ignored
`pointercancel` and capture loss, and closing the view or the workspace during a hold
left the button pressed.

## Decision

### One ordered lane per workspace

Every request that changes a workspace goes through one lane (`requestLane.ts`). A request
is sent only after the previous one settled, so the backend applies gestures and commands
in the order they were made. An idle lane sends at once, in the same task as the click or
gesture. Frame reads are not in the lane; they change nothing.

- **Commands** keep the one-at-a-time rule. A command made while another command is
  pending is ignored, and the toolbar is disabled while one is pending. An accepted
  command waits in the lane behind earlier gestures. A Step clicked while a press is in
  flight is therefore sent after the press, and its cycle sees the button pressed.
- **Gestures** are never ignored. A gesture made while a command or an earlier gesture is
  pending waits in the lane and is sent after it. Gestures do not disable the toolbar.
- **Frames.** Each request takes its frame request number when it is sent, not when it is
  made. A response older than the newest applied frame is still dropped, so the newest
  request's frame wins.
- **Failures.** A failed request is reported (a gesture names the component and the
  action) and the lane continues with the next request.
- **A backend that never answers** cannot block the lane forever. After 30 s without an
  answer, a request stops holding the lane, a warning says so, and later requests are
  sent. Ordering is only guaranteed for requests that answer within that time.

### Push-button holds

The workspace tracks which push buttons the operator holds and what holds each one: a
pointer (by pointer id) or a key (`heldButtons.ts`). A press starts a hold only if the
button is not held yet. Only a matching holder ends it. Every press therefore gets exactly
one release, and a second pointer, a repeated `pointerup` or a capture loss after the
release sends nothing.

A hold ends on:

| Event                                                     | Ends                                              |
| --------------------------------------------------------- | ------------------------------------------------- |
| `pointerup`, `pointercancel`, `lostpointercapture`        | the hold of that pointer                          |
| `keyup` of Space or Enter                                 | the hold of that key (key repeat is ignored)      |
| focus leaving the button                                  | a key hold                                        |
| the window losing focus                                   | every hold                                        |
| the process view closing (another tab, another workspace) | every hold                                        |
| the workspace closing                                     | every hold, released before the close in the lane |

`pointerdown` captures the pointer, so letting go away from the button still releases it.
`pointerleave` does not release: a held button stays pressed until the operator lets go.
Only the primary button presses.

When the workspace closes, the backend saves the project, so held buttons are released
first and the close is sent after every earlier request. Requests carry their workspace
id. A request for a workspace that was replaced fails with `StaleWorkspace` and cannot
change another workspace.

## Alternatives

- **A queue for gestures only, commands still sent at once.** Rejected: a gesture and a
  command sent together race on the backend workers. A Step could run before the press
  it follows.
- **Ordering in the backend (a sequence number per request).** It would need a contract
  change and still could not apply a request it has not received. The order is known
  where the requests are made.
- **Releasing on `pointerleave`.** Rejected: an operator who slides off the button would
  release it unintentionally. Pointer capture keeps the gesture on the button.

## Consequences

- Toolbar commands are accepted while a gesture is in flight and are sent after it. Before
  this decision they were silently ignored.
- A gesture made during a long command (Build & Load, check) is sent when the command
  finishes, not dropped.
- PLC semantics, timing and process models are unchanged; the Rust core stays
  authoritative. The interface decides only the order in which it sends requests.
