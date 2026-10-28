# Vendor platform profiles

A platform profile describes how PLC Lab **presents and approximates** a vendor platform.
Profiles are data (`simulation` field of family articles in the knowledge pack, plus the
built-in `generic.iec`). Generic runtime code never branches on vendor names; it branches
only on typed behavior selections:

| Field         | Values                                                                                                      |
| ------------- | ----------------------------------------------------------------------------------------------------------- |
| `addressing`  | `iecDirect` (%IX0.0), `byteBit` (%I0.0), `moduleDefinedTags`, `linkedTerminal`, `octalDevices`, `unitPorts` |
| `analog`      | `perTenThousand` (0..10000), `s7Nominal27648`, `fullScale32767`, `engineeringUnits` (REAL)                  |
| `startup`     | `reinitializeNonRetentive`                                                                                  |
| `products`    | product family names per generic module type (presentation only)                                            |
| `terminology` | generic term to platform term                                                                               |
| `fidelity`    | level plus modeled / approximated / omitted lists                                                           |

## Catalog

| Tier | Vendor                     | Families (knowledge)                            | Simulation profiles                              |
| ---- | -------------------------- | ----------------------------------------------- | ------------------------------------------------ |
| A    | Siemens SIMATIC            | S7-1200, S7-1500, ET 200SP, S7-300/400 (legacy) | `siemens.s7-1200`, `siemens.s7-1500`             |
| A    | Rockwell / Allen-Bradley   | CompactLogix, ControlLogix, POINT I/O           | `rockwell.compactlogix`, `rockwell.controllogix` |
| A    | Beckhoff                   | CX + TwinCAT 3, EtherCAT Terminals              | `beckhoff.twincat3`                              |
| A    | Schneider Electric Modicon | M241/M251/M262, M580                            | `schneider.m200`, `schneider.m580`               |
| B    | Mitsubishi Electric MELSEC | iQ-F (FX5), iQ-R                                | `mitsubishi.fx5` (conceptual)                    |
| B    | OMRON Sysmac               | NX/NJ                                           | `omron.nx-nj` (conceptual)                       |
| B    | Phoenix Contact PLCnext    | PLCnext Control                                 | none                                             |
| B    | WAGO                       | PFC200                                          | none                                             |

Tier A profiles are `APPROXIMATED_BEHAVIORAL`: addresses and analog raw values follow
the platform's documented conventions; diagnostics, timing and parameters are PLC Lab's
own. Tier B profiles are `CONCEPTUAL`: presentation only.

## Rules

- No vendor conformance is claimed. Product names in profiles are representative family
  names, not validated configurations, and carry a note saying so.
- Facts about vendor platforms cite source records; most vendor portal records are marked
  `unverified`. Documents checked during authoring (Siemens analog value processing
  manual, Beckhoff EL30xx presentation, NE 43 summary) are `verified`.
- Analog raw-value conventions used in profiles: S7 nominal 0..27648 (overrange to 32511,
  overflow 32767), Beckhoff EL305x 4 mA = 0 and 20 mA = 32767, Logix engineering units
  (REAL), PLC Lab generic 0..10000. Always check the module manual before relying on a
  threshold.

## Using profiles

A free lab chooses a profile when it is created. The Hardware tab then shows addresses in
the platform's style, product names and the analog raw values the program receives; the
Inspector shows the profile's fidelity. Exercises fix their profile (all v1 exercises use
`generic.iec`, so their numbers are platform independent).
