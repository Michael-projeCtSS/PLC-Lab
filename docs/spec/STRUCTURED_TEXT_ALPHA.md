# PLC Lab Alpha Structured Text

> **Historical document.** It describes the pre-rebuild prototype (milestones M1-M4B). The current product is described in [PLC_FEATURE_MATRIX.md](PLC_FEATURE_MATRIX.md). Kept for engineering history.

M2A implements a deliberately small language. This is not a claim of full IEC 61131-3 or vendor compatibility. [ADR-0012](../adr/0012-alpha-st-compiler.md) records the compiler boundary; accepted M1 types, IR, runtime and TON semantics remain authoritative.

## IMPLEMENTED

Exactly one named PROGRAM, zero or more VAR_INPUT/VAR_OUTPUT/VAR sections before statements, BOOL/INT/REAL/TIME scalars, local TON instances, assignments, nested IF/THEN/optional ELSE/END_IF and named TON calls. Keywords, types, identifiers and TON parameter names are ASCII case-insensitive. Preserve original spelling for display and diagnostics. Names match `[A-Za-z_][A-Za-z0-9_]*`; keywords cannot be identifiers. There are no declaration initializers or comma-separated declarations.

```text
document    = PROGRAM name { section } { statement } END_PROGRAM EOF
section     = (VAR_INPUT | VAR_OUTPUT | VAR) { declaration } END_VAR
declaration = name ':' (BOOL | INT | REAL | TIME | TON) ';'
statement   = name ':=' expression ';'
            | IF expression THEN { statement } [ELSE { statement }] END_IF ';'
            | name '(' [association { ',' association }] ')' ';'
association = name ':=' expression | name '=>' name
primary     = literal | name | '(' expression ')'
unary       = primary | NOT unary | '-' unary
```

TON declarations are permitted only in VAR. Input associations use :=; output associations use =>. IN/BOOL and PT/TIME are required on every call; Q/BOOL and ET/TIME are optional writable targets. Source order is arbitrary; the existing runtime lowering normalizes IN, PT, Q, ET. There is one semantic TON definition per compiled document and independent declared instances. No Delay.Q/Delay.ET member access exists.

Operator precedence, highest first: parentheses/literals/names; unary NOT/minus; multiply/divide; add/subtract; comparisons (=, <>, <, <=, >, >=); AND; OR. Binary operators at the same level associate left. Type rules come from plc-types compatibility and final IR validation; no implicit conversions, TIME arithmetic or BOOL ordering are added. Ordinary expression evaluation is performed only by the existing runtime.

Whitespace is discarded. Non-nested `(* block comments *)` and `// line comments` are supported; // is an Alpha convenience/vendor-common extension. Nested or unterminated block comments are diagnosed. Unicode is allowed in comments, including non-BMP characters. LF, CRLF and lone CR are line breaks. Backend columns count one-based Unicode scalars, never UTF-8 bytes; spans are start-inclusive/end-exclusive and allow EOF points. Monaco conversion is explicit and counts UTF-16 code units.

## Literals

BOOL is TRUE/FALSE, case-insensitive. INT syntax is decimal digits with a separately parsed numeric minus. Lexing retains magnitude text before range checks. Semantic interpretation of signed numeric literals admits -32768 and rejects positive 32768, -32769 and oversized magnitudes. No wrapping/truncation/saturation occurs. Negation of a variable or non-literal expression remains canonical Unary Negate.

REAL is `digits '.' digits [E ['+'|'-'] digits]`, with case-insensitive E. Examples: 0.0, 1.5, -3.25, 1.0E3, 1.0e-3. Integer exponent-only syntax such as 1e3 and incomplete 1./1.0e are rejected. Existing PlcReal constructs finite binary32 values and canonical positive zero; non-finite overflow and NaN/INF/Infinity text are rejected. Literal signs are interpreted before constructing checked values; no general constant folding or runtime arithmetic transformation occurs.

TIME prefixes T#/TIME# are case-insensitive. A payload consists of one or more nonnegative integer/unit components in strictly descending order d, h, m, s, ms, us, with no repetition. Exact factors in microseconds are 86,400,000,000; 3,600,000,000; 60,000,000; 1,000,000; 1,000; 1. Each parse/multiply/add is checked u64 arithmetic. Examples: T#0s, T#500ms, T#1m30s, T#1h2m3s, T#2s500ms, T#100us. No fractional component, sign, nanosecond, whitespace between components or floating-point conversion exists. Malformed/order/repetition/overflow errors are normal compiler diagnostics.

## Compiler API and limits

`compile_st(source, CompilerConfig) -> Result<CompiledProgram, Vec<Diagnostic>>` is headless. `parse_st` exposes the separate ST AST for inspection; it cannot execute. CompiledProgram has private construction and owns an immutable IR candidate already accepted by plc-ir::validate. Its validated() method obtains the existing borrowed ValidatedProgram at runtime lowering. Errors never yield a partially executable wrapper.

| Limit                                                   | Default | Absolute ceiling |
| ------------------------------------------------------- | ------- | ---------------- |
| Source bytes, before allocating character/token storage | 65,536  | 1,048,576        |
| Tokens, including EOF                                   | 16,384  | 65,536           |
| Parser recursion and expression-tree depth              | 32      | 48               |
| Diagnostics                                             | 32      | 128              |

CompilerConfig::new rejects zero/out-of-ceiling values. Tree depth checks include long flat operator chains before constructing deep recursive objects. Error recovery synchronizes at semicolons/section or branch/program ends and recognizable next statements; every loop makes progress or stops. On reaching the diagnostic cap, its final slot contains PLC-ST-010 and traversal stops. The unchanged canonical validator also enforces its independent declaration/node/depth limits. A limit returns data, not a panic or process termination. Source-driven filesystem, sockets, processes, imports, native/JavaScript generation and eval are absent.

PouId and DocumentId start at 1 per compilation. Scalar SymbolIds and FunctionBlockInstanceIds increment independently in declaration order. A TON definition ID is allocated once upon the first TON declaration; explicit standard kind, not its numeric value, selects semantics. SourceElementIds increment in deterministic syntax preorder (program, declarations, statements/expressions and diagnostic anchors). Symbols resolve through the accepted M1 SymbolTable. IDs are reproducible for identical source/configuration and are not promised stable across arbitrary edits.

## Desktop compilation

Monaco edits source and colors tokens. Typed IPC v3 compile_st sends source/version to a bounded Rust compiler worker and returns a success/failure union containing diagnostics and optional program/declaration summaries. It returns no IR, ExecutionPlan, memory values or persistent runtime. Compile is logically pure. Frontend states are NOT COMPILED, COMPILING, COMPILE SUCCESS and COMPILE FAILED; local monotonic revision/request counters reject stale asynchronous results. Editing clears obsolete markers/declarations. Backend scalar spans are converted to UTF-16 for markers/navigation. Runtime controls remain disabled and Variables values show —/Not running.

## PLANNED

Separately authorized runtime UI/trace/process milestones, project persistence and later language increments. These are not implemented by M2A and cannot be inferred from successful compilation.

## UNSUPPORTED

Multiple PROGRAMs; CASE/ELSIF/FOR/WHILE/REPEAT/RETURN; ARRAY/STRUCT/enums/strings; user functions/FBs, methods/classes/namespaces; VAR_IN_OUT/VAR_TEMP/RETAIN/AT and direct I/Q/M addressing; pointers/references/vendor instructions; imports/INCLUDE; other standard blocks; FB member access; LAD/FBD/STL. Unsupported constructs produce diagnostics rather than silently selecting a program or extending semantics.
