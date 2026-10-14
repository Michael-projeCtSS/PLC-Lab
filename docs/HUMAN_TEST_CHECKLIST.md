# Human test checklist

Run automated validation first. Perform this checklist in DEV and then in RELEASE when testing a newly built release. Human visual testing complements automated checks and does not replace them. Future milestones must extend this checklist while retaining these foundation checks.

- Test date:
- Tester:
- Context shown in Diagnostics: DEVELOPMENT / RELEASE
- Application version:
- Core version:
- IPC contract version:
- Automated validation result:

## Startup

- [ ] The application opens through the selected root launcher.
- [ ] There is no immediate crash or unexpected disappearance.
- [ ] The window renders correctly at its default size.
- [ ] There is no blank page or indefinitely uninitialized application state.
- [ ] The terminal identifies DEV or RELEASE and displays any failure clearly.

## Main shell

- [ ] Project is visible.
- [ ] Program is visible.
- [ ] Variables is visible.
- [ ] Process is visible.
- [ ] Network is visible.
- [ ] Timeline is visible.
- [ ] Diagnostics is visible.

## Execution controls

- [ ] Run exists.
- [ ] Pause exists.
- [ ] Stop exists.
- [ ] Single Scan exists.
- [ ] All four controls remain disabled because the PLC Runtime is not implemented yet.
- [ ] No fake program execution, variable values, process behavior, or network activity is presented.

## Backend and build identity

- [ ] Diagnostics reaches CORE CONNECTED rather than staying CONNECTING or showing an IPC error.
- [ ] Application/core information is displayed after the real Rust IPC response arrives.
- [ ] Application/core versions and IPC contract version are visible.
- [ ] The context matches the launcher: DEVELOPMENT for DEV, RELEASE for the locally built release.
- [ ] The release can start without the Vite development server running.
- [ ] If no release exists, the release launcher asks me to run BUILD_PLC_LAB_RELEASE.bat and does not build automatically.

CORE CONNECTED and its version/context information are rendered from getCoreInfo supplied by Rust, not from handwritten UI version constants. Automated native acceptance verifies that real IPC boundary; this checklist verifies its visible outcome.

## Visual integrity

- [ ] Major panels do not overlap.
- [ ] Primary text is not obviously clipped.
- [ ] The layout is intact at the default application window size.
- [ ] Diagnostics and Timeline are readable.
- [ ] Resizing within the supported window limits keeps the workspace usable.

## Findings

- [ ] Any failure includes the mode, displayed versions, checklist item, and useful terminal error text.
- [ ] Issues discovered during human testing are resolved and relevant automated checks rerun before acceptance.

Notes:

```text
Record observations here. Empty Process/Network/Timeline states and disabled
execution controls are expected at M0; they are not missing runtime behavior.
```
