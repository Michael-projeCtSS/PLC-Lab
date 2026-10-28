# Roadmap

The product rebuild (see [rebuild report](PRODUCT_REBUILD_REPORT.md)) delivered the
architecture and the first complete product: Learn, Practice, Troubleshoot, Free Lab,
Knowledge and Progress over one deterministic engine. Next increments, each requiring its
own scoped task and, where fundamental, an ADR:

1. **Signed Windows distribution**: code signing in a release pipeline, optional installer.
2. **More components**: variable frequency drive with local/remote control (enables the
   documented VFD troubleshooting case), conveyor with moving parts, pneumatic cylinder
   with end-position sensors, second pump for alternating-pump exercises.
3. **Structured Text growth**: DINT/UDINT/WORD types, arrays, user functions and FBs, loops,
   CTUD, retentive memory.
4. **Ladder Diagram editor** on the dedicated rung/grid model compiling to the same IR, with
   ST/LAD semantic-equivalence tests; then FBD; then the educational STL profile.
5. **More exercises and labs** per new component; analog output exercises.
6. **Replay and timeline scrubbing** over the compact trace.
7. **Additional protocol adapters** on the virtual network kernel (Modbus RTU first), still
   educational, never claiming conformance.
8. **Localization** of the user interface and knowledge content.
