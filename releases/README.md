# Releases

- `knowledge/PLC_Lab_Knowledge_Pack_v1.zip` (+ `.sha256`): the knowledge pack release
  archive, committed and verified by `npm run knowledge:verify`.

This folder holds archives and CI artifacts only; it is not a runtime location. The
application is `PLC Lab.exe` in the repository root, with its data in `resources/`, both
created by `BUILD_PLC_LAB_RELEASE.bat` (ADR-0030). Earlier builds staged a separate
`current/` folder here; staging removes it.

See [build and run](../docs/BUILD_AND_RUN.md).
