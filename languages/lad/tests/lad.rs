//! The LAD frontend (ADR-0050): document validation, lowering into the canonical IR and
//! behavioural equivalence with Structured Text in the one PLC runtime. These documents
//! are version 1; version 1 stays readable (ADR-0051).
use lad_frontend::{codes, compile_lad, is_lad, is_synthetic};
use plc_ir::Program;
use plc_types::{
    PlcTime, PlcValue,
    identity::SymbolId,
    source::SourceLanguage,
    symbols::{Scope, VariableCategory},
};
use serde_json::{Value, json};

fn variable(name: &str, direction: &str) -> Value {
    json!({ "name": name, "type": "BOOL", "direction": direction })
}
fn no(id: u32, variable: &str) -> Value {
    json!({ "type": "contact", "id": id, "contact": "normallyOpen", "variable": variable })
}
fn nc(id: u32, variable: &str) -> Value {
    json!({ "type": "contact", "id": id, "contact": "normallyClosed", "variable": variable })
}
fn series(id: u32, items: Vec<Value>) -> Value {
    json!({ "type": "series", "id": id, "items": items })
}
fn parallel(id: u32, branches: Vec<Value>) -> Value {
    json!({ "type": "parallel", "id": id, "branches": branches })
}
fn rung(id: u32, network: Value, coils: &[(u32, &str)]) -> Value {
    json!({
        "id": id,
        "network": network,
        "coils": coils.iter().map(|(id, v)| json!({ "id": id, "variable": v })).collect::<Vec<_>>(),
    })
}
fn document(variables: Vec<Value>, rungs: Vec<Value>) -> String {
    json!({
        "format": "plclab-lad",
        "version": 1,
        "name": "Main",
        "variables": variables,
        "rungs": rungs,
    })
    .to_string()
}

/// Rung 1 nests series and parallel blocks with a normally closed contact; rung 2 reads
/// the coil rung 1 wrote in the same scan; rung 3 holds itself over scans.
fn network_lad() -> String {
    document(
        vec![
            variable("A", "input"),
            variable("B", "input"),
            variable("C", "input"),
            variable("D", "input"),
            variable("E", "input"),
            variable("F", "input"),
            variable("G", "input"),
            variable("Start", "input"),
            variable("Stop", "input"),
            variable("Q1", "output"),
            variable("Q2", "output"),
            variable("Motor", "output"),
        ],
        vec![
            rung(
                1,
                series(
                    2,
                    vec![
                        parallel(
                            3,
                            vec![
                                series(4, vec![no(5, "A"), nc(6, "B")]),
                                series(
                                    7,
                                    vec![no(8, "C"), parallel(9, vec![no(10, "D"), nc(11, "E")])],
                                ),
                            ],
                        ),
                        no(12, "F"),
                    ],
                ),
                &[(13, "Q1")],
            ),
            rung(
                14,
                parallel(15, vec![no(16, "Q1"), no(17, "G")]),
                &[(18, "Q2")],
            ),
            rung(
                19,
                series(
                    20,
                    vec![
                        parallel(21, vec![no(22, "Start"), no(23, "Motor")]),
                        nc(24, "Stop"),
                    ],
                ),
                &[(25, "Motor")],
            ),
        ],
    )
}
const NETWORK_ST: &str = "PROGRAM Main
VAR_INPUT A : BOOL; B : BOOL; C : BOOL; D : BOOL; E : BOOL; F : BOOL; G : BOOL;
    Start : BOOL; Stop : BOOL; END_VAR
VAR_OUTPUT Q1 : BOOL; Q2 : BOOL; Motor : BOOL; END_VAR
Q1 := ((A AND NOT B) OR (C AND (D OR NOT E))) AND F;
Q2 := Q1 OR G;
Motor := (Start OR Motor) AND NOT Stop;
END_PROGRAM";

struct Plc {
    program: Program,
    runtime: plc_runtime::Runtime,
}
impl Plc {
    fn new(program: Program) -> Self {
        let plan = plc_runtime::lower(&plc_ir::validate(&program).unwrap()).unwrap();
        let runtime = plc_runtime::Runtime::new(
            plan,
            plc_runtime::RuntimeConfig::new(PlcTime::from_micros(100_000), 10_000).unwrap(),
        );
        Self { program, runtime }
    }
    fn symbol(&self, name: &str) -> SymbolId {
        self.program
            .symbols()
            .resolve(Scope::Pou(self.program.entry()), name)
            .unwrap()
            .id()
    }
    fn scan(&mut self, inputs: &[(&str, bool)], outputs: &[&str]) -> Vec<PlcValue> {
        for (name, value) in inputs {
            let id = self.symbol(name);
            self.runtime.set_input(id, PlcValue::Bool(*value)).unwrap();
        }
        self.runtime.single_scan().unwrap();
        let snapshot = self.runtime.snapshot();
        outputs
            .iter()
            .map(|name| snapshot.outputs()[&self.symbol(name)])
            .collect()
    }
}
fn ladder() -> Plc {
    Plc::new(compile_lad(&network_lad()).unwrap().program)
}
fn text() -> Plc {
    let compiled =
        st_frontend::compile_st(NETWORK_ST, st_frontend::CompilerConfig::default()).unwrap();
    Plc::new(compiled.program().clone())
}

#[test]
fn equivalent_ladder_and_structured_text_agree_on_every_input_combination() {
    let (mut lad, mut st) = (ladder(), text());
    let names = ["A", "B", "C", "D", "E", "F", "G"];
    let mut q1_true = 0;
    for bits in 0u32..(1 << names.len()) {
        let inputs: Vec<(&str, bool)> = names
            .iter()
            .enumerate()
            .map(|(i, n)| (*n, bits & (1 << i) != 0))
            .collect();
        let expected = st.scan(&inputs, &["Q1", "Q2"]);
        assert_eq!(lad.scan(&inputs, &["Q1", "Q2"]), expected, "{inputs:?}");
        if expected[0] == PlcValue::Bool(true) {
            q1_true += 1;
        }
    }
    // The network is neither constant nor trivial.
    assert!(q1_true > 10 && q1_true < 100, "{q1_true}");
}

#[test]
fn a_self_holding_rung_keeps_its_state_over_scans_like_structured_text() {
    let (mut lad, mut st) = (ladder(), text());
    let script = [
        (false, false, false),
        (true, false, true),
        (false, false, true),
        (false, false, true),
        (false, true, false),
        (false, false, false),
        (true, true, false),
        (true, false, true),
        (false, false, true),
    ];
    for (step, (start, stop, motor)) in script.into_iter().enumerate() {
        let inputs = [("Start", start), ("Stop", stop)];
        let expected = st.scan(&inputs, &["Motor"]);
        assert_eq!(expected, [PlcValue::Bool(motor)], "ST step {step}");
        assert_eq!(lad.scan(&inputs, &["Motor"]), expected, "LAD step {step}");
    }
}

#[test]
fn variables_and_elements_become_symbols_and_ladder_sources() {
    let compiled = compile_lad(&network_lad()).unwrap();
    assert_eq!(compiled.name, "Main");
    let program = &compiled.program;
    let scope = Scope::Pou(program.entry());
    let category = |name: &str| program.symbols().resolve(scope, name).unwrap().category();
    assert_eq!(category("a"), VariableCategory::Input, "case-insensitive");
    assert_eq!(category("Q1"), VariableCategory::Output);
    // Each rung ends with one assignment per coil, at the coil's element; before it,
    // every network element assigned the power leaving it, left to right.
    let elements: Vec<u32> = program
        .body()
        .iter()
        .map(|s| s.source().element_id().get())
        .collect();
    assert_eq!(
        elements,
        [
            5, 6, 4, 8, 10, 11, 9, 7, 3, 12, 2, 13, 16, 17, 15, 18, 22, 23, 21, 24, 20, 25
        ]
    );
    assert!(
        program
            .sources()
            .elements()
            .all(|e| e.language() == SourceLanguage::Lad)
    );
    assert_eq!(program.sources().elements().count(), 25);
    // The power symbols are the compiler's: hidden, and not a name a student can write.
    let hidden: Vec<String> = program
        .symbols()
        .symbols()
        .filter(|s| is_synthetic(s.name()))
        .map(|s| s.name().to_owned())
        .collect();
    assert_eq!(hidden.len(), 19, "{hidden:?}");
    assert!(hidden.contains(&"LAD#2".to_owned()));
}

#[test]
fn an_empty_network_connects_the_left_rail_to_the_coils() {
    let source = document(
        vec![variable("Ready", "output"), variable("Lamp", "output")],
        vec![rung(1, series(2, vec![]), &[(3, "Ready"), (4, "Lamp")])],
    );
    let mut plc = Plc::new(compile_lad(&source).unwrap().program);
    assert_eq!(
        plc.scan(&[], &["Ready", "Lamp"]),
        [PlcValue::Bool(true), PlcValue::Bool(true)]
    );
}

fn errors(source: &str) -> Vec<(String, String)> {
    compile_lad(source)
        .unwrap_err()
        .into_iter()
        .map(|d| (d.code().to_owned(), d.message().to_owned()))
        .collect()
}
fn element_of(source: &str) -> Option<u32> {
    compile_lad(source).unwrap_err()[0]
        .source()
        .map(|s| s.reference().element_id().get())
}

#[test]
fn malformed_documents_are_rejected_with_a_clear_reason() {
    let e = errors("{\"format\": \"plclab-lad\", ");
    assert_eq!(e[0].0, codes::DOCUMENT);
    assert!(
        e[0].1.starts_with("The ladder document is not valid"),
        "{e:?}"
    );
    let e = errors(
        &json!({ "format": "plclab-lad", "version": 3, "name": "Main", "variables": [], "rungs": [] })
            .to_string(),
    );
    assert_eq!(e[0].0, codes::DOCUMENT);
    assert!(e[0].1.contains("version 3"), "{e:?}");
    let e = errors(
        &json!({ "format": "plclab-lad", "version": 1, "name": "Main", "variables": [], "rungs": [], "extra": 1 })
            .to_string(),
    );
    assert_eq!(e[0].0, codes::DOCUMENT, "unknown fields");
    let too_big = format!("{}{}", network_lad(), " ".repeat(lad_frontend::MAX_BYTES));
    assert_eq!(errors(&too_big)[0].0, codes::LIMIT);
}

#[test]
fn declarations_must_be_valid_variables_with_distinct_names() {
    let e = errors(&document(
        vec![json!({ "name": "Level", "type": "WORD", "direction": "input" })],
        vec![],
    ));
    assert_eq!(e[0].0, codes::DECLARATION);
    assert!(e[0].1.contains("BOOL, INT, REAL or TIME"), "{e:?}");
    // Compiler-owned names cannot be declared.
    let e = errors(&document(vec![variable("LAD#1", "local")], vec![]));
    assert_eq!(e[0].0, codes::DECLARATION, "{e:?}");
    let e = errors(&document(
        vec![variable("Pump", "output"), variable("PUMP", "local")],
        vec![],
    ));
    assert_eq!(e[0].0, codes::DECLARATION, "duplicate ignoring case");
    let e = errors(&document(vec![variable("2nd", "local")], vec![]));
    assert_eq!(e[0].0, codes::DECLARATION, "invalid name");
    let bad_name = json!({ "format": "plclab-lad", "version": 1, "name": "My Program", "variables": [], "rungs": [] });
    assert_eq!(errors(&bad_name.to_string())[0].0, codes::DECLARATION);
}

#[test]
fn rung_errors_name_the_rung_and_point_at_the_element() {
    let vars = || vec![variable("In", "input"), variable("Out", "output")];
    // An undeclared variable.
    let source = document(vars(), vec![rung(1, no(2, "Missing"), &[(3, "Out")])]);
    assert_eq!(
        errors(&source),
        [(
            codes::UNKNOWN.to_owned(),
            "Rung 1: variable 'Missing' is not declared.".to_owned()
        )]
    );
    assert_eq!(element_of(&source), Some(2));
    // A contact without a variable.
    let source = document(vars(), vec![rung(1, no(2, ""), &[(3, "Out")])]);
    assert_eq!(errors(&source)[0].1, "Rung 1: an element has no variable.");
    // A coil on an input.
    let source = document(vars(), vec![rung(1, no(2, "In"), &[(3, "In")])]);
    assert_eq!(errors(&source)[0].0, codes::TARGET);
    assert_eq!(element_of(&source), Some(3));
    // A rung without a coil.
    let source = document(vars(), vec![rung(1, no(2, "In"), &[])]);
    assert_eq!(errors(&source)[0].0, codes::RUNG);
    assert_eq!(element_of(&source), Some(1));
    // A parallel block with one branch, and an empty series inside a network.
    let source = document(
        vars(),
        vec![rung(1, parallel(2, vec![no(3, "In")]), &[(4, "Out")])],
    );
    assert!(errors(&source)[0].1.contains("at least two branches"));
    let source = document(
        vars(),
        vec![rung(
            1,
            series(2, vec![no(3, "In"), series(4, vec![])]),
            &[(5, "Out")],
        )],
    );
    assert!(errors(&source)[0].1.contains("empty series"));
    // Errors in the second rung say so.
    let source = document(
        vars(),
        vec![
            rung(1, no(2, "In"), &[(3, "Out")]),
            rung(4, no(5, "Nope"), &[(6, "Out")]),
        ],
    );
    assert_eq!(
        errors(&source)[0].1,
        "Rung 2: variable 'Nope' is not declared."
    );
}

#[test]
fn element_ids_are_persistent_unique_and_bounded() {
    let vars = || vec![variable("In", "input"), variable("Out", "output")];
    let source = document(vars(), vec![rung(1, no(1, "In"), &[(3, "Out")])]);
    assert_eq!(
        errors(&source)[0],
        (
            codes::RUNG.to_owned(),
            "Element id 1 is used twice.".to_owned()
        )
    );
    let source = document(vars(), vec![rung(1, no(0, "In"), &[(3, "Out")])]);
    assert_eq!(errors(&source)[0].0, codes::RUNG);
    // Nesting deeper than the limit.
    let mut node = no(1000, "In");
    for depth in 0..lad_frontend::MAX_DEPTH as u32 {
        node = series(2000 + depth, vec![node, no(3000 + depth, "In")]);
    }
    let source = document(vars(), vec![rung(1, node, &[(2, "Out")])]);
    assert_eq!(errors(&source)[0].0, codes::LIMIT);
}

#[test]
fn ladder_documents_are_told_apart_from_structured_text() {
    assert!(is_lad(&network_lad()));
    assert!(is_lad(&format!("\n  {}", network_lad())));
    assert!(!is_lad(NETWORK_ST));
    assert!(!is_lad("{ not a ladder }"));
}
