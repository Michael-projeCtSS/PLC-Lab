//! Retentive declarations, constant initializers and the FIRST_SCAN system variable
//! (ADR-0038): from the ST source into canonical symbol metadata.
use plc_runtime::{Runtime, RuntimeConfig, StartKind, lower};
use plc_types::{
    PlcTime, PlcValue,
    symbols::{Retention, Scope, Symbol, SystemVariable},
};
use st_frontend::{CompiledProgram, CompilerConfig, codes, compile_st};

fn compile(source: &str) -> CompiledProgram {
    compile_st(source, CompilerConfig::default()).unwrap_or_else(|e| panic!("{e:#?}\n{source}"))
}
fn symbol<'a>(c: &'a CompiledProgram, name: &str) -> &'a Symbol {
    let entry = Scope::Pou(c.program().entry());
    c.program().symbols().resolve(entry, name).unwrap()
}
fn fails(source: &str, code: &str, text: &str) {
    let errors = compile_st(source, CompilerConfig::default()).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.code() == code && e.message().contains(text)),
        "{errors:#?}"
    );
}
fn program(declarations: &str, body: &str) -> String {
    format!("PROGRAM Main\n{declarations}\n{body}\nEND_PROGRAM\n")
}

#[test]
fn retained_variables_and_initial_values_reach_the_canonical_symbols() {
    let c = compile(&program(
        "VAR_INPUT Sensor : BOOL; END_VAR
         VAR_OUTPUT Shown : INT := 7; END_VAR
         VAR RETAIN Count : INT := -3; Total : REAL := 2.5; Window : TIME := T#2s; Seen : BOOL := TRUE; Plain : INT; END_VAR
         VAR Scratch : INT := 4; Edge : R_TRIG; END_VAR",
        "Edge(CLK := Sensor); IF Edge.Q THEN Count := Count + 1; END_IF; Shown := Count;",
    ));
    let count = symbol(&c, "Count");
    assert_eq!(count.retention(), Retention::Retained);
    assert_eq!(count.initial_value(), PlcValue::Int(-3));
    assert_eq!(
        symbol(&c, "Total").initial_value(),
        PlcValue::real(2.5).unwrap()
    );
    let window = symbol(&c, "Window").initial_value();
    assert_eq!(window, PlcValue::Time(PlcTime::from_micros(2_000_000)));
    assert_eq!(symbol(&c, "Seen").initial_value(), PlcValue::Bool(true));
    // Without an initializer the initial value is the type's default.
    let plain = symbol(&c, "Plain");
    assert_eq!(
        (plain.retention(), plain.declared_initial()),
        (Retention::Retained, None)
    );
    assert_eq!(plain.initial_value(), PlcValue::Int(0));
    let scratch = symbol(&c, "Scratch");
    assert_eq!(scratch.retention(), Retention::NonRetained);
    assert_eq!(scratch.initial_value(), PlcValue::Int(4));
    assert_eq!(symbol(&c, "Shown").initial_value(), PlcValue::Int(7));
    let retained: Vec<_> = c
        .declarations()
        .iter()
        .filter(|d| d.retained)
        .map(|d| d.name.as_str())
        .collect();
    assert_eq!(retained, ["Count", "Total", "Window", "Seen", "Plain"]);
}

#[test]
fn programs_without_retention_compile_as_before() {
    let c = compile(&program(
        "VAR_INPUT A : BOOL; END_VAR VAR_OUTPUT Q : BOOL; END_VAR VAR T : TON; END_VAR",
        "T(IN := A, PT := T#1s); Q := T.Q;",
    ));
    assert!(c.program().symbols().symbols().all(|s| {
        s.retention() == Retention::NonRetained
            && s.declared_initial().is_none()
            && s.system().is_none()
    }));
}

#[test]
fn initial_values_must_be_constants_of_the_declared_type_and_range() {
    let decl = |d: &str| program(&format!("VAR {d} END_VAR"), "");
    fails(
        &decl("X : INT := 1.5;"),
        codes::TYPE,
        "must be INT, not REAL",
    );
    fails(
        &decl("X : REAL := 1;"),
        codes::TYPE,
        "must be REAL, not INT",
    );
    fails(
        &decl("X : BOOL := 0;"),
        codes::TYPE,
        "must be BOOL, not INT",
    );
    fails(&decl("X : INT := 40000;"), codes::LITERAL, "");
    fails(
        &decl("Y : INT; X : INT := Y + 1;"),
        codes::UNSUPPORTED,
        "constant INT literal",
    );
    fails(
        &decl("X : BOOL := NOT TRUE;"),
        codes::UNSUPPORTED,
        "constant BOOL literal",
    );
    fails(
        &program("VAR_INPUT X : BOOL := TRUE; END_VAR", ""),
        codes::UNSUPPORTED,
        "an initial value is not allowed",
    );
    fails(
        &decl("X : TON := 1;"),
        codes::UNSUPPORTED,
        "no initial value",
    );
    fails(&decl("X : INT := 1; x : BOOL;"), codes::DUPLICATE, "");
}

#[test]
fn retain_is_refused_outside_program_variables_and_for_block_instances() {
    fails(
        &program("VAR_INPUT RETAIN X : BOOL; END_VAR", ""),
        codes::UNSUPPORTED,
        "VAR RETAIN",
    );
    fails(
        &program("VAR_OUTPUT RETAIN X : BOOL; END_VAR", ""),
        codes::UNSUPPORTED,
        "VAR RETAIN",
    );
    for block in ["TON", "TOF", "TP", "R_TRIG", "F_TRIG", "CTU", "CTD"] {
        fails(
            &program(&format!("VAR RETAIN B : {block}; END_VAR"), ""),
            codes::UNSUPPORTED,
            "not retentive in PLC Lab",
        );
    }
}

#[test]
fn first_scan_is_a_read_only_system_variable_that_cannot_be_shadowed() {
    let c = compile(&program(
        "VAR_OUTPUT Started : BOOL; END_VAR",
        "IF first_scan THEN Started := TRUE; END_IF;",
    ));
    let first = symbol(&c, "FIRST_SCAN");
    assert_eq!(first.system(), Some(SystemVariable::FirstScan));
    assert_eq!(first.retention(), Retention::NonRetained);
    // A program that never reads it has no such symbol.
    let plain = compile(&program("VAR X : BOOL; END_VAR", "X := TRUE;"));
    assert!(
        plain
            .program()
            .symbols()
            .symbols()
            .all(|s| s.system().is_none())
    );
    fails(
        &program("VAR FIRST_SCAN : BOOL; END_VAR", ""),
        codes::DUPLICATE,
        "reserved",
    );
    fails(
        &program("VAR RETAIN First_Scan : BOOL; END_VAR", ""),
        codes::DUPLICATE,
        "reserved",
    );
    fails(
        &program("", "FIRST_SCAN := FALSE;"),
        codes::TARGET,
        "read-only",
    );
    fails(
        &program(
            "VAR E : R_TRIG; END_VAR",
            "E(CLK := TRUE, Q => FIRST_SCAN);",
        ),
        codes::BLOCK,
        "read-only",
    );
}

// ---- runtime: one initialization per start (ADR-0038) ---------------------------------

const COUNTER: &str = "PROGRAM Main
VAR_INPUT Part : BOOL; END_VAR
VAR_OUTPUT Shown : INT; Started : BOOL; END_VAR
VAR RETAIN Count : INT := 10; END_VAR
VAR Seen : INT := 5; Edge : R_TRIG; Scans : INT; END_VAR
Edge(CLK := Part);
IF Edge.Q THEN Count := Count + 1; Seen := Seen + 1; END_IF;
IF FIRST_SCAN THEN Scans := 0; END_IF;
Scans := Scans + 1;
Started := FIRST_SCAN;
Shown := Count;
END_PROGRAM";

struct Plc {
    compiled: CompiledProgram,
    runtime: Runtime,
}
impl Plc {
    fn new(source: &str) -> Self {
        let compiled = compile(source);
        let plan = lower(&compiled.validated().unwrap()).unwrap();
        let config = RuntimeConfig::new(PlcTime::from_micros(100_000), 10_000).unwrap();
        let runtime = Runtime::new(plan, config);
        Self { compiled, runtime }
    }
    fn value(&self, name: &str) -> PlcValue {
        let id = symbol(&self.compiled, name).id();
        self.runtime.snapshot().committed()[&id]
    }
    fn scan(&mut self, part: bool) {
        let id = symbol(&self.compiled, "Part").id();
        self.runtime.set_input(id, PlcValue::Bool(part)).unwrap();
        self.runtime.single_scan().unwrap();
    }
    fn pulse(&mut self) {
        self.scan(true);
        self.scan(false);
    }
}

#[test]
fn memory_starts_at_the_declared_initial_values_and_first_scan_is_true_once() {
    let mut plc = Plc::new(COUNTER);
    assert_eq!(plc.value("Count"), PlcValue::Int(10));
    assert_eq!(plc.value("Seen"), PlcValue::Int(5));
    assert!(plc.runtime.first_scan_pending());
    // Observing the runtime does not consume the first scan.
    let _ = plc.runtime.snapshot();
    assert!(plc.runtime.first_scan_pending());
    plc.scan(false);
    assert_eq!(plc.value("Started"), PlcValue::Bool(true));
    assert!(!plc.runtime.first_scan_pending());
    plc.scan(false);
    assert_eq!(plc.value("Started"), PlcValue::Bool(false));
    assert_eq!(plc.value("Scans"), PlcValue::Int(2));
    assert_eq!(plc.runtime.retained_variables(), 1);
}

#[test]
fn a_warm_start_keeps_retained_values_and_a_cold_start_initializes_everything() {
    let mut plc = Plc::new(COUNTER);
    plc.pulse();
    plc.pulse();
    assert_eq!(
        (plc.value("Count"), plc.value("Seen")),
        (PlcValue::Int(12), PlcValue::Int(7))
    );
    plc.runtime.start(StartKind::Warm);
    assert_eq!(plc.value("Count"), PlcValue::Int(12), "retained");
    assert_eq!(
        plc.value("Seen"),
        PlcValue::Int(5),
        "back to its initial value"
    );
    assert_eq!(
        plc.value("Shown"),
        PlcValue::Int(0),
        "outputs are not retained"
    );
    assert_eq!(plc.runtime.snapshot().completed_scans(), 0);
    assert!(plc.runtime.first_scan_pending());
    plc.scan(false);
    assert_eq!(plc.value("Started"), PlcValue::Bool(true));
    assert_eq!(plc.value("Scans"), PlcValue::Int(1));
    // The edge memory restarted FALSE: a part present at the first scan counts once.
    plc.scan(true);
    assert_eq!(plc.value("Count"), PlcValue::Int(13));
    plc.scan(true);
    assert_eq!(plc.value("Count"), PlcValue::Int(13));
    plc.runtime.start(StartKind::Cold);
    assert_eq!(
        (plc.value("Count"), plc.value("Seen")),
        (PlcValue::Int(10), PlcValue::Int(5))
    );
    assert!(plc.runtime.first_scan_pending());
    // A memory reset of the runtime is a cold start.
    plc.pulse();
    plc.runtime.reset();
    assert_eq!(plc.value("Count"), PlcValue::Int(10));
}

#[test]
fn block_instances_restart_on_every_start() {
    let source = "PROGRAM Main
VAR_INPUT Part : BOOL; END_VAR
VAR_OUTPUT Shown : INT; END_VAR
VAR RETAIN Kept : INT; END_VAR
VAR Counter : CTU; END_VAR
Counter(CU := Part, R := FALSE, PV := 100);
Shown := Counter.CV;
Kept := Counter.CV;
END_PROGRAM";
    let mut plc = Plc::new(source);
    plc.pulse();
    plc.pulse();
    plc.pulse();
    assert_eq!(plc.value("Shown"), PlcValue::Int(3));
    plc.runtime.start(StartKind::Warm);
    assert_eq!(
        plc.value("Kept"),
        PlcValue::Int(3),
        "the copied value is retained"
    );
    plc.scan(false);
    assert_eq!(
        plc.value("Shown"),
        PlcValue::Int(0),
        "the CTU instance itself is not retentive"
    );
}

#[test]
fn the_same_program_and_inputs_give_the_same_values_over_restarts() {
    let run = || {
        let mut plc = Plc::new(COUNTER);
        let mut values = Vec::new();
        for step in 0..12 {
            plc.scan(step % 3 == 0);
            if step == 5 {
                plc.runtime.start(StartKind::Warm);
            }
            if step == 9 {
                plc.runtime.start(StartKind::Cold);
            }
            values.push((plc.value("Count"), plc.value("Seen"), plc.value("Started")));
        }
        values
    };
    assert_eq!(run(), run());
}
