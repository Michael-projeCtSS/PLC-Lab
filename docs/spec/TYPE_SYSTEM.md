# Alpha PLC type system

> **Prototype-era specification (milestones M1-M4B).** The semantic rules below remain implemented and tested in the current product; status lines and statements about missing features, milestones or the user interface are historical. Current overview: [PRODUCT_ARCHITECTURE.md](../PRODUCT_ARCHITECTURE.md) and [PLC_FEATURE_MATRIX.md](PLC_FEATURE_MATRIX.md).

Status: M1A value/type foundations accepted by technical-lead review. M1B implements the separate headless execution rules in [PLC_RUNTIME_SEMANTICS.md](PLC_RUNTIME_SEMANTICS.md), pending review. This is PLC Lab's minimal generic profile, not full IEC or vendor conformance. [ADR-0009](../adr/0009-semantic-foundation.md) records the accepted representations.

## Representations and construction

| PLC type | Rust representation                          | Valid states and construction                                                                                                                                                               |
| -------- | -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| BOOL     | `PlcValue::Bool(bool)`                       | Exactly false/true; no numeric truthiness.                                                                                                                                                  |
| INT      | `PlcValue::Int(i16)`                         | Signed 16-bit, -32768 through 32767 inclusive. `PlcValue::int(i64)` checks bounds; rejects overflow rather than wrapping/truncating.                                                        |
| REAL     | `PlcValue::Real(PlcReal)` with private `f32` | IEEE 754 binary32, finite values only, including subnormals. `new(f32)` rejects NaN and either infinity; normalizes -0 to +0. Largest magnitude is `f32::MAX` (approximately 3.4028235e38). |
| TIME     | `PlcValue::Time(PlcTime)` with private `u64` | Nonnegative duration in integer microseconds, 0 through 18446744073709551615. `from_micros(u64)` is inherently safe; `try_from_micros(i128)` rejects negatives and overflow. Zero is valid. |

`PlcType` has exactly these four variants. `PlcValue::plc_type` identifies its type. Values use same-variant exact equality; different variants are unequal even when their payloads look numerically equivalent. REAL equality is exact binary32 numeric equality, not epsilon comparison. Finite values and normalized zero make it reflexive; `Eq` is therefore valid. Grading tolerance, if needed, must later be a separate explicit grading policy, never a hidden PLC comparison rule.

No TIME value contains a host timestamp, date, time-of-day, or wall-clock reference. Its unit matches virtual ticks in ADR-0003 and can represent a future TON preset or elapsed duration. M1B uses it for a checked SimulationClock and exact comparisons; no TIME arithmetic or timer behavior exists.

## Assignment compatibility

Rows are target types; columns are source value types. I means identity assignment; U means unsupported.

| Target / source | BOOL | INT | REAL | TIME |
| --------------- | ---- | --- | ---- | ---- |
| BOOL            | I    | U   | U    | U    |
| INT             | U    | I   | U    | U    |
| REAL            | U    | U   | I    | U    |
| TIME            | U    | U   | U    | I    |

The reusable `compatibility::assignment` returns `AssignmentCompatibility`:

- Identity: same PLC type; currently the only permitted assignment.
- ImplicitConversion: reserved classification; no Alpha pair qualifies.
- ExplicitConversion: reserved classification; no Alpha conversion is currently approved or implemented.
- Unsupported: every cross-type pair, including INT to REAL and REAL to INT.

Host construction checks (i64 into bounded INT, i128 into TIME) validate representations. They are not PLC-language conversion operators. No BOOL/numeric conversion, TIME/numeric conversion, promotion, rounding cast, or explicit-conversion IR node exists.

## Operators and result types

| Operation                                      | Accepted operands                                         | Result                                             |
| ---------------------------------------------- | --------------------------------------------------------- | -------------------------------------------------- |
| NOT                                            | BOOL                                                      | BOOL                                               |
| Unary numeric negation                         | INT or REAL                                               | Same type                                          |
| AND, OR                                        | BOOL and BOOL                                             | BOOL                                               |
| Add, subtract, multiply, divide                | INT/INT or REAL/REAL                                      | Same numeric type                                  |
| Equal, not equal                               | Identical types: BOOL/BOOL, INT/INT, REAL/REAL, TIME/TIME | BOOL                                               |
| Less, less-or-equal, greater, greater-or-equal | INT/INT, REAL/REAL, TIME/TIME                             | BOOL                                               |
| Conditional                                    | BOOL condition                                            | No expression result; two ordered statement bodies |

All other pairs are unsupported: mixed INT/REAL comparisons or arithmetic, BOOL ordering, bitwise numeric operators, and every TIME arithmetic operation, including negation. `unary_result` and `binary_result` return an optional semantic result type; they never calculate values. Future frontends reuse these checks instead of duplicating rules in parsers. IR result-type claims must agree with these checks and symbol declarations.

## Serialization boundary

Values support serde with explicit tags, for example `{"type":"BOOL","value":true}` or `{"type":"TIME","value":"5000000"}`. The runtime TIME representation is always `u64`; only its serialized payload is a canonical unsigned decimal string, so JavaScript cannot lose integer precision. Reject signs, whitespace, leading zeros other than `"0"`, units, overflow, and numeric JSON TIME payloads. This is a machine encoding, not an ST TIME literal parser.

INT decoding preserves its range; REAL decoding constructs a binary32 value and passes the finite/zero validator. Decimal JSON REAL numbers are quantized to binary32 by serde, including finite underflow to zero; this does not grant a PLC REAL/INT conversion. Invalid type tags, payload types, nonfinite results, and unknown fields are rejected. REAL strings such as `"NaN"` are not accepted. JSON floating decimal text is a boundary encoding, not authoritative stored text.

No symbols or IR can be deserialized yet. No project persistence or public IPC value DTO has been introduced. These serde checks establish safe value fixtures, not a complete inbound project acceptance path.

## Static typing and separate execution

M1A specifies operand/result typing, value identity, and construction. M1B now implements checked INT arithmetic, finite/canonical REAL results, left-to-right/eager expression evaluation, exclusive IF selection, and transactional scan fault rollback in plc-runtime. See [PLC_RUNTIME_SEMANTICS.md](PLC_RUNTIME_SEMANTICS.md) for exact policies. plc-types compatibility functions remain pure type checks and do not execute expressions.

A well-typed IR expression such as INT maximum plus one, or division by a zero constant, passes static validation and faults when selected for M1B execution. Static validation promises structure/typing, not arithmetic safety. No constant folding or value-based arithmetic analysis is performed.

`PlcType::default_value` is the single authoritative default implementation: false, zero INT, canonical positive-zero REAL, and zero TIME. Runtime images use it without duplicating profile defaults.

## Evidence

`plc-types` tests cover all 16 assignment pairs, every unary/binary type pair, type identity, both BOOL values, signed INT limits and overflow, REAL finite/extreme/subnormal/zero policy, TIME zero/positive/max/negative/overflow, and checked JSON round trips and invalid payloads. `plc-ir` negative tests verify these rules at the common IR boundary.
