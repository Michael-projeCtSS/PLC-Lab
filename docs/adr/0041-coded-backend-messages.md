# ADR-0041: Coded backend messages for the interface language

- Status: Accepted within the requested localization scope
- Date: 2026-11-06

## Context

The commissioning checklist, the hardware comparison, the event log, the signal map and
the CPU diagnostics produced English sentences in Rust. The interface could only show
them as they were, so a Polish student read English in exactly the engineering views
where the wording matters most. Matching English sentences in the interface would break
whenever a sentence changed and would guess at values inside them.

## Decision

- `lab-model::message` defines `Message { code, args, text }`. `code` is stable, `args`
  are named values, `text` is the canonical English text. A value is data (names, tags,
  addresses, numbers), a vocabulary term with its own code (a quality, an LED state, a
  module type) or a nested message (a slot, a list of reasons).
- `message!("code", "English {name}", name = value)` builds both from the same values,
  so the English text is exactly what the backend produced before. Placeholders are plain
  `{name}`; numbers are formatted before they are passed.
- Commissioning findings and hints, hardware findings (message, expected, detected), the
  signal map's device state and every event carry messages. Checklist titles stay keyed
  by their stable item and category ids; CPU diagnostic entries keep their kind and typed
  details, which the interface already words.
- IPC contract 8, additive: `MessageDto` (code, text, args) and `MessageArgDto` (name,
  text, term, nested message) travel next to the existing English fields
  (`findingMessages`, `howMessage`, `detail`, `expectedMessage`, `detectedMessage`,
  `deviceStateMessage`, event `message`). The validators accept at most four levels of
  nesting and 32 values per message.
- The interface shows the backend text in English. In Polish it words the code from
  `packages/ui/src/locale/backend-pl.ts` and each value by its kind; a code without a
  Polish entry shows the English original, never a guess.
- `scripts/backend-messages.mjs` inventories every coded message and term in the Rust
  sources. Tests require Polish wording with the same placeholders for each of them, a
  stated reason for every Polish text equal to the English one, and dictionary module
  names equal to the backend's generic names.

## Alternatives

- Translate English sentences in the interface by pattern: rejected, it silently breaks
  when a sentence or a value format changes.
- Localize in Rust: rejected, the backend would need the viewer's language and every
  language switch would round-trip through the engine.
- Replace the English fields by messages: rejected for this release, the additive form
  keeps every consumer and test of the English text unchanged.

## Consequences

A new backend sentence needs a code and a Polish template; the inventory test names any
that is missing. Scene and device labels, project names and the runtime's own error
details remain data and are shown as sent.
