//! End-to-end tests for the curriculum language extensions: ST -> IR -> one runtime.
use plc_runtime::{Runtime, RuntimeConfig, lower};
use plc_types::{PlcTime, PlcValue, identity::SymbolId, symbols::Scope};
use st_frontend::{CompiledProgram, CompilerConfig, codes, compile_st};

fn compile(source: &str) -> CompiledProgram {
    compile_st(source, CompilerConfig::default()).unwrap_or_else(|e| panic!("{e:#?}\n{source}"))
}
fn runtime(compiled: &CompiledProgram) -> Runtime {
    Runtime::new(
        lower(&compiled.validated().unwrap()).unwrap(),
        RuntimeConfig::new(PlcTime::from_micros(100_000), 10_000).unwrap(),
    )
}
fn id(c: &CompiledProgram, name: &str) -> SymbolId {
    c.program().symbols().resolve(Scope::Pou(c.program().entry()), name).unwrap().id()
}
fn fails(source: &str, code: &str) {
    let errors = compile_st(source, CompilerConfig::default()).unwrap_err();
    assert!(errors.iter().any(|e| e.code() == code), "{errors:#?}");
}
/// Scan once per input value and return the named output after each scan.
fn run(source: &str, input: &str, inputs: &[PlcValue], output: &str) -> Vec<PlcValue> {
    let c = compile(source);
    let mut r = runtime(&c);
    let (i, o) = (id(&c, input), id(&c, output));
    inputs
        .iter()
        .map(|v| {
            r.set_input(i, *v).unwrap();
            r.single_scan().unwrap().snapshot().committed()[&o]
        })
        .collect()
}
fn b(v: bool) -> PlcValue {
    PlcValue::Bool(v)
}
fn n(v: i16) -> PlcValue {
    PlcValue::Int(v)
}

#[test]
fn elsif_selects_the_first_true_branch() {
    let src = "PROGRAM Main VAR_INPUT X : INT; END_VAR VAR_OUTPUT Y : INT; END_VAR
        IF X < 0 THEN Y := -1; ELSIF X = 0 THEN Y := 0; ELSIF X < 10 THEN Y := 1; ELSE Y := 2; END_IF;
        END_PROGRAM";
    assert_eq!(run(src, "X", &[n(-5), n(0), n(3), n(99)], "Y"), [n(-1), n(0), n(1), n(2)]);
}

#[test]
fn case_values_ranges_lists_and_else() {
    let src = "PROGRAM Main VAR_INPUT S : INT; END_VAR VAR_OUTPUT Y : INT; END_VAR
        CASE S OF
          0: Y := 10;
          1, 3: Y := 20;
          4..6: Y := 30;
          -2: Y := 40;
        ELSE Y := 99;
        END_CASE;
        END_PROGRAM";
    assert_eq!(
        run(src, "S", &[n(0), n(1), n(2), n(3), n(5), n(-2), n(7)], "Y"),
        [n(10), n(20), n(99), n(20), n(30), n(40), n(99)]
    );
    fails(
        "PROGRAM Main VAR S : INT; END_VAR CASE S OF 1..3: S := 1; 2: S := 2; END_CASE; END_PROGRAM",
        codes::DUPLICATE,
    );
    fails("PROGRAM Main VAR B : BOOL; END_VAR CASE B OF 1: B := TRUE; END_CASE; END_PROGRAM", codes::TYPE);
}

#[test]
fn xor_mod_and_iec_precedence() {
    let x = "PROGRAM Main VAR_INPUT A : BOOL; END_VAR VAR_OUTPUT Y : BOOL; END_VAR Y := A XOR TRUE; END_PROGRAM";
    assert_eq!(run(x, "A", &[b(false), b(true)], "Y"), [b(true), b(false)]);
    let m = "PROGRAM Main VAR_INPUT A : INT; END_VAR VAR_OUTPUT Y : INT; END_VAR Y := A MOD 3; END_PROGRAM";
    assert_eq!(run(m, "A", &[n(7), n(-7), n(-32768)], "Y"), [n(1), n(-1), n(-2)]);
    // Relational binds tighter than equality: (1 < 2) = (3 < 4).
    let p = "PROGRAM Main VAR_INPUT A : BOOL; END_VAR VAR_OUTPUT Y : BOOL; END_VAR Y := 1 < 2 = 3 < 4; END_PROGRAM";
    assert_eq!(run(p, "A", &[b(false)], "Y"), [b(true)]);
    let c = compile("PROGRAM Main VAR A : INT; END_VAR A := 5 MOD 0; END_PROGRAM");
    let error = runtime(&c).single_scan().unwrap_err();
    assert_eq!(error.diagnostic().unwrap().code(), "PLC-RUNTIME-003");
}

#[test]
fn wide_boolean_chains_compile_with_constant_depth() {
    let names: Vec<String> = (0..200).map(|i| format!("I{i}")).collect();
    let src = format!(
        "PROGRAM Main VAR_INPUT {} END_VAR VAR_OUTPUT Y : BOOL; END_VAR Y := {}; END_PROGRAM",
        names.iter().map(|n| format!("{n} : BOOL;")).collect::<String>(),
        names.join(" OR ")
    );
    let c = compile(&src);
    let mut r = runtime(&c);
    assert_eq!(r.single_scan().unwrap().snapshot().committed()[&id(&c, "Y")], b(false));
    r.set_input(id(&c, "I199"), b(true)).unwrap();
    assert_eq!(r.single_scan().unwrap().snapshot().committed()[&id(&c, "Y")], b(true));
}

#[test]
fn conversions_round_ties_to_even_and_fault_on_overflow() {
    let src = "PROGRAM Main VAR_INPUT R : REAL; END_VAR VAR_OUTPUT Y : INT; T : INT; END_VAR
        Y := REAL_TO_INT(R); T := TRUNC(R); END_PROGRAM";
    let real = |v| PlcValue::real(v).unwrap();
    assert_eq!(run(src, "R", &[real(2.5), real(3.5), real(-2.5), real(2.4)], "Y"), [n(2), n(4), n(-2), n(2)]);
    assert_eq!(run(src, "R", &[real(2.9), real(-2.9)], "T"), [n(2), n(-2)]);
    let c = compile(src);
    let mut r = runtime(&c);
    r.set_input(id(&c, "R"), real(40000.0)).unwrap();
    assert_eq!(r.single_scan().unwrap_err().diagnostic().unwrap().code(), "PLC-RUNTIME-002");
    let scale = "PROGRAM Main VAR_INPUT Raw : INT; END_VAR VAR_OUTPUT Level : REAL; END_VAR
        Level := INT_TO_REAL(Raw) * 100.0 / 27648.0; END_PROGRAM";
    assert_eq!(run(scale, "Raw", &[n(13824)], "Level"), [real(50.0)]);
    fails("PROGRAM Main VAR A : REAL; END_VAR A := INT_TO_REAL(1.0); END_PROGRAM", codes::TYPE);
    fails("PROGRAM Main VAR A : REAL; END_VAR A := SQRT(1.0); END_PROGRAM", codes::UNSUPPORTED);
}

#[test]
fn timer_outputs_are_readable_as_members() {
    let src = "PROGRAM Main VAR_INPUT Start : BOOL; END_VAR VAR_OUTPUT Motor : BOOL; Elapsed : TIME; END_VAR
        VAR Delay : TON; END_VAR
        Delay(IN := Start, PT := T#300ms);
        Motor := Delay.Q;
        Elapsed := Delay.ET;
        END_PROGRAM";
    let out = run(src, "Start", &[b(true), b(true), b(true), b(true), b(false)], "Motor");
    assert_eq!(out, [b(false), b(false), b(false), b(true), b(false)]);
    fails(
        "PROGRAM Main VAR D : TON; X : BOOL; END_VAR X := D.IN; END_PROGRAM",
        codes::BLOCK,
    );
    fails("PROGRAM Main VAR X : BOOL; Y : BOOL; END_VAR X := Y.Q; END_PROGRAM", codes::UNKNOWN);
}

#[test]
fn edge_triggers_counters_and_off_delay_from_st() {
    let src = "PROGRAM Main VAR_INPUT Pulse : BOOL; END_VAR VAR_OUTPUT Count : INT; Done : BOOL; Fell : BOOL; Hold : BOOL; END_VAR
        VAR Edge : R_TRIG; Fall : F_TRIG; Counter : CTU; Off : TOF; END_VAR
        Edge(CLK := Pulse);
        Fall(CLK := Pulse);
        Counter(CU := Edge.Q, PV := 2, Q => Done, CV => Count);
        Off(IN := Pulse, PT := T#200ms, Q => Hold);
        Fell := Fall.Q;
        END_PROGRAM";
    let c = compile(src);
    let mut r = runtime(&c);
    let pulse = id(&c, "Pulse");
    let mut trace = Vec::new();
    for p in [true, true, false, true, false, false, false] {
        r.set_input(pulse, b(p)).unwrap();
        let s = r.single_scan().unwrap().snapshot().clone();
        trace.push((
            s.committed()[&id(&c, "Count")],
            s.committed()[&id(&c, "Done")],
            s.committed()[&id(&c, "Fell")],
            s.committed()[&id(&c, "Hold")],
        ));
    }
    assert_eq!(
        trace,
        [
            (n(1), b(false), b(false), b(true)),
            (n(1), b(false), b(false), b(true)),
            (n(1), b(false), b(true), b(true)),
            (n(2), b(true), b(false), b(true)),
            (n(2), b(true), b(true), b(true)),
            (n(2), b(true), b(false), b(true)),
            (n(2), b(true), b(false), b(false)),
        ]
    );
}

#[test]
fn fault_inside_a_counter_scan_rolls_back_block_state() {
    let src = "PROGRAM Main VAR_INPUT Pulse : BOOL; D : INT; END_VAR VAR_OUTPUT Count : INT; X : INT; END_VAR
        VAR C : CTU; END_VAR
        C(CU := Pulse, PV := 10, CV => Count);
        X := 10 / D;
        END_PROGRAM";
    let c = compile(src);
    let mut r = runtime(&c);
    r.set_input(id(&c, "D"), n(1)).unwrap();
    r.set_input(id(&c, "Pulse"), b(true)).unwrap();
    r.single_scan().unwrap();
    r.set_input(id(&c, "Pulse"), b(false)).unwrap();
    r.single_scan().unwrap();
    r.set_input(id(&c, "Pulse"), b(true)).unwrap();
    r.set_input(id(&c, "D"), n(0)).unwrap();
    assert!(r.single_scan().is_err());
    let snapshot = r.snapshot();
    assert_eq!(snapshot.committed()[&id(&c, "Count")], n(1), "the counted edge was rolled back");
}
