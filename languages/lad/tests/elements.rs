//! Version 2 ladder elements (ADR-0051): SET/RESET coils, edge contacts, timer and counter
//! boxes and comparators, each executed by the one runtime and proven equal to the
//! Structured Text it stands for; the power observations; and their diagnostics.
use lad_frontend::{codes, compile_lad};
use plc_ir::Program;
use plc_types::{PlcTime, PlcValue, identity::SymbolId, symbols::Scope};
use serde_json::{Value, json};

fn var(name: &str, kind: &str, direction: &str) -> Value {
    json!({ "name": name, "type": kind, "direction": direction })
}
fn contact(id: u32, kind: &str, variable: &str) -> Value {
    json!({ "type": "contact", "id": id, "contact": kind, "variable": variable })
}
fn no(id: u32, variable: &str) -> Value {
    contact(id, "normallyOpen", variable)
}
fn series(id: u32, items: Vec<Value>) -> Value {
    json!({ "type": "series", "id": id, "items": items })
}
fn parallel(id: u32, branches: Vec<Value>) -> Value {
    json!({ "type": "parallel", "id": id, "branches": branches })
}
fn coil(id: u32, variable: &str, kind: &str) -> Value {
    if kind == "output" {
        json!({ "id": id, "variable": variable })
    } else {
        json!({ "id": id, "variable": variable, "kind": kind })
    }
}
fn rung(id: u32, network: Value, coils: Vec<Value>) -> Value {
    json!({ "id": id, "network": network, "coils": coils })
}
fn block(id: u32, kind: &str, instance: &str, extra: Value) -> Value {
    let mut b = json!({ "type": "block", "id": id, "block": kind, "instance": instance });
    for (k, v) in extra.as_object().unwrap() {
        b[k] = v.clone();
    }
    b
}
fn compare(id: u32, op: &str, left: Value, right: Value) -> Value {
    json!({ "type": "compare", "id": id, "op": op, "left": left, "right": right })
}
fn v(name: &str) -> Value {
    json!({ "variable": name })
}
fn k(text: &str) -> Value {
    json!({ "constant": text })
}
fn document(variables: Vec<Value>, rungs: Vec<Value>) -> String {
    json!({ "format": "plclab-lad", "version": 2, "name": "Main", "variables": variables, "rungs": rungs })
        .to_string()
}

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
    fn lad(source: &str) -> Self {
        Self::new(
            compile_lad(source)
                .unwrap_or_else(|e| panic!("{e:#?}"))
                .program,
        )
    }
    fn st(source: &str) -> Self {
        let compiled = st_frontend::compile_st(source, st_frontend::CompilerConfig::default())
            .unwrap_or_else(|e| panic!("{e:#?}\n{source}"));
        Self::new(compiled.program().clone())
    }
    fn symbol(&self, name: &str) -> SymbolId {
        self.program
            .symbols()
            .resolve(Scope::Pou(self.program.entry()), name)
            .unwrap_or_else(|_| panic!("no symbol {name}"))
            .id()
    }
    fn scan(&mut self, inputs: &[(&str, PlcValue)], read: &[&str]) -> Vec<PlcValue> {
        for (name, value) in inputs {
            let id = self.symbol(name);
            self.runtime.set_input(id, *value).unwrap();
        }
        self.runtime.single_scan().unwrap();
        let committed = self.runtime.snapshot().committed().clone();
        read.iter().map(|n| committed[&self.symbol(n)]).collect()
    }
}
const T: PlcValue = PlcValue::Bool(true);
const F: PlcValue = PlcValue::Bool(false);
fn b(value: bool) -> PlcValue {
    PlcValue::Bool(value)
}

/// Drive both programs with the same inputs, scan by scan, and compare `read`.
fn equivalent(lad: &str, st: &str, script: &[Vec<(&str, PlcValue)>], read: &[&str]) {
    let (mut l, mut s) = (Plc::lad(lad), Plc::st(st));
    for (scan, inputs) in script.iter().enumerate() {
        let expected = s.scan(inputs, read);
        assert_eq!(l.scan(inputs, read), expected, "scan {scan}: {inputs:?}");
    }
}

#[test]
fn set_and_reset_coils_latch_and_the_later_instruction_wins() {
    let lad = document(
        vec![
            var("Start", "BOOL", "input"),
            var("Stop", "BOOL", "input"),
            var("Run", "BOOL", "output"),
            var("Latch", "BOOL", "output"),
        ],
        vec![
            rung(
                1,
                no(2, "Start"),
                vec![coil(3, "Run", "set"), coil(4, "Latch", "reset")],
            ),
            rung(
                5,
                no(6, "Stop"),
                vec![coil(7, "Run", "reset"), coil(8, "Latch", "set")],
            ),
        ],
    );
    let st = "PROGRAM Main
VAR_INPUT Start : BOOL; Stop : BOOL; END_VAR
VAR_OUTPUT Run : BOOL; Latch : BOOL; END_VAR
IF Start THEN Run := TRUE; Latch := FALSE; END_IF;
IF Stop THEN Run := FALSE; Latch := TRUE; END_IF;
END_PROGRAM";
    let step = |start, stop| vec![("Start", b(start)), ("Stop", b(stop))];
    let script = [
        step(false, false),
        step(true, false),
        step(false, false),
        step(false, true),
        step(false, false),
        step(true, true),
        step(false, false),
    ];
    equivalent(&lad, st, &script, &["Run", "Latch"]);
    // The documented rule, scan by scan: SET holds when unpowered; when both coils of
    // one variable are powered in the same scan, the later one (rung 2) wins.
    let mut plc = Plc::lad(&lad);
    let run = |plc: &mut Plc, start, stop| plc.scan(&step(start, stop), &["Run", "Latch"]);
    assert_eq!(run(&mut plc, true, false), [T, F]);
    assert_eq!(run(&mut plc, false, false), [T, F], "SET holds");
    assert_eq!(run(&mut plc, true, true), [F, T], "RESET in rung 2 wins");
}

#[test]
fn a_rung_powers_all_its_coils_with_one_value_computed_before_them() {
    // The first coil changes the variable the contact reads; the second coil still gets
    // the power of the scan's evaluation, not a re-evaluation.
    let lad = document(
        vec![
            var("Toggle", "BOOL", "output"),
            var("Copy", "BOOL", "output"),
        ],
        vec![rung(
            1,
            contact(2, "normallyClosed", "Toggle"),
            vec![coil(3, "Toggle", "output"), coil(4, "Copy", "output")],
        )],
    );
    let mut plc = Plc::lad(&lad);
    assert_eq!(plc.scan(&[], &["Toggle", "Copy"]), [T, T]);
    assert_eq!(plc.scan(&[], &["Toggle", "Copy"]), [F, F]);
}

#[test]
fn edge_contacts_pass_power_for_one_scan_like_r_trig_and_f_trig() {
    let lad = document(
        vec![
            var("Button", "BOOL", "input"),
            var("Enable", "BOOL", "input"),
            var("Rise", "BOOL", "output"),
            var("Fall", "BOOL", "output"),
        ],
        vec![
            rung(
                1,
                series(
                    2,
                    vec![no(3, "Enable"), contact(4, "positiveEdge", "Button")],
                ),
                vec![coil(5, "Rise", "output")],
            ),
            rung(
                6,
                contact(7, "negativeEdge", "Button"),
                vec![coil(8, "Fall", "output")],
            ),
        ],
    );
    let st = "PROGRAM Main
VAR_INPUT Button : BOOL; Enable : BOOL; END_VAR
VAR_OUTPUT Rise : BOOL; Fall : BOOL; END_VAR
VAR Up : R_TRIG; Down : F_TRIG; END_VAR
Up(CLK := Button);
Rise := Enable AND Up.Q;
Down(CLK := Button);
Fall := Down.Q;
END_PROGRAM";
    let step = |button, enable| vec![("Button", b(button)), ("Enable", b(enable))];
    // Edges are detected whether or not power reaches the contact: the memory follows
    // the variable every scan.
    let script = [
        step(true, true),
        step(true, true),
        step(false, true),
        step(true, false),
        step(false, false),
        step(true, true),
        step(false, true),
        step(false, true),
    ];
    equivalent(&lad, st, &script, &["Rise", "Fall"]);
    let mut plc = Plc::lad(&lad);
    let rise: Vec<PlcValue> = script.iter().map(|s| plc.scan(s, &["Rise"])[0]).collect();
    assert_eq!(rise, [T, F, F, F, F, T, F, F]);
}

#[test]
fn timer_boxes_are_the_runtimes_standard_timers() {
    for kind in ["TON", "TOF", "TP"] {
        let lad = document(
            vec![
                var("Start", "BOOL", "input"),
                var("Lamp", "BOOL", "output"),
                var("Elapsed", "TIME", "output"),
            ],
            vec![rung(
                1,
                series(
                    2,
                    vec![
                        no(3, "Start"),
                        block(
                            4,
                            kind,
                            "T1",
                            json!({ "preset": k("T#300ms"), "output": "Elapsed" }),
                        ),
                    ],
                ),
                vec![coil(5, "Lamp", "output")],
            )],
        );
        let st = format!(
            "PROGRAM Main
VAR_INPUT Start : BOOL; END_VAR
VAR_OUTPUT Lamp : BOOL; Elapsed : TIME; END_VAR
VAR T1 : {kind}; END_VAR
T1(IN := Start, PT := T#300ms, ET => Elapsed);
Lamp := T1.Q;
END_PROGRAM"
        );
        let script: Vec<Vec<(&str, PlcValue)>> = [
            true, true, true, true, true, false, false, false, false, true, false,
        ]
        .iter()
        .map(|on| vec![("Start", b(*on))])
        .collect();
        equivalent(&lad, &st, &script, &["Lamp", "Elapsed"]);
    }
    // TON from a TIME variable preset: Lamp follows after 3 scans of 100 ms.
    let lad = document(
        vec![
            var("Start", "BOOL", "input"),
            var("Delay", "TIME", "input"),
            var("Lamp", "BOOL", "output"),
        ],
        vec![rung(
            1,
            series(
                2,
                vec![
                    no(3, "Start"),
                    block(4, "TON", "Delay1", json!({ "preset": v("Delay") })),
                ],
            ),
            vec![coil(5, "Lamp", "output")],
        )],
    );
    let mut plc = Plc::lad(&lad);
    let delay = PlcValue::Time(PlcTime::from_micros(300_000));
    let lamp: Vec<PlcValue> = (0..5)
        .map(|_| plc.scan(&[("Start", T), ("Delay", delay)], &["Lamp"])[0])
        .collect();
    assert_eq!(lamp, [F, F, F, T, T]);
}

#[test]
fn counter_boxes_count_edges_and_reset_like_ctu_and_ctd() {
    let lad = document(
        vec![
            var("Part", "BOOL", "input"),
            var("Clear", "BOOL", "input"),
            var("Load", "BOOL", "input"),
            var("Full", "BOOL", "output"),
            var("Empty", "BOOL", "output"),
            var("Count", "INT", "output"),
            var("Left", "INT", "output"),
        ],
        vec![
            rung(
                1,
                series(
                    2,
                    vec![
                        no(3, "Part"),
                        block(
                            4,
                            "CTU",
                            "Up",
                            json!({ "preset": k("3"), "reset": v("Clear"), "output": "Count" }),
                        ),
                    ],
                ),
                vec![coil(5, "Full", "output")],
            ),
            rung(
                6,
                series(
                    7,
                    vec![
                        no(8, "Part"),
                        block(
                            9,
                            "CTD",
                            "Down",
                            json!({ "preset": k("2"), "reset": v("Load"), "output": "Left" }),
                        ),
                    ],
                ),
                vec![coil(10, "Empty", "output")],
            ),
        ],
    );
    let st = "PROGRAM Main
VAR_INPUT Part : BOOL; Clear : BOOL; Load : BOOL; END_VAR
VAR_OUTPUT Full : BOOL; Empty : BOOL; Count : INT; Left : INT; END_VAR
VAR Up : CTU; Down : CTD; END_VAR
Up(CU := Part, PV := 3, R := Clear, CV => Count);
Full := Up.Q;
Down(CD := Part, PV := 2, LD := Load, CV => Left);
Empty := Down.Q;
END_PROGRAM";
    let step = |part, clear, load| vec![("Part", b(part)), ("Clear", b(clear)), ("Load", b(load))];
    let script = [
        step(false, false, true),
        step(false, false, false),
        step(true, false, false),
        step(false, false, false),
        step(true, false, false),
        step(false, false, false),
        step(true, false, false),
        step(true, false, false),
        step(false, true, true),
        step(true, false, false),
    ];
    equivalent(&lad, st, &script, &["Full", "Empty", "Count", "Left"]);
    let mut plc = Plc::lad(&lad);
    let last = script
        .iter()
        .map(|s| plc.scan(s, &["Count", "Full"]))
        .nth(7)
        .unwrap();
    assert_eq!(last, [PlcValue::Int(3), T], "three rising edges reach PV");
}

#[test]
fn comparators_pass_power_while_they_hold() {
    let lad = document(
        vec![
            var("Level", "INT", "input"),
            var("Temp", "REAL", "input"),
            var("Waited", "TIME", "input"),
            var("Ok", "BOOL", "input"),
            var("High", "BOOL", "output"),
            var("Exact", "BOOL", "output"),
            var("Warm", "BOOL", "output"),
            var("Late", "BOOL", "output"),
            var("Same", "BOOL", "output"),
        ],
        vec![
            rung(
                1,
                compare(2, "gt", v("Level"), k("80")),
                vec![coil(3, "High", "output")],
            ),
            rung(
                4,
                compare(5, "eq", v("Level"), k("50")),
                vec![coil(6, "Exact", "output")],
            ),
            rung(
                7,
                parallel(
                    8,
                    vec![
                        compare(9, "ge", v("Temp"), k("60.5")),
                        compare(10, "lt", k("-5.0"), v("Temp")),
                    ],
                ),
                vec![coil(11, "Warm", "output")],
            ),
            rung(
                12,
                compare(13, "le", k("T#2s"), v("Waited")),
                vec![coil(14, "Late", "output")],
            ),
            rung(
                15,
                compare(16, "ne", v("Ok"), k("FALSE")),
                vec![coil(17, "Same", "output")],
            ),
        ],
    );
    let st = "PROGRAM Main
VAR_INPUT Level : INT; Temp : REAL; Waited : TIME; Ok : BOOL; END_VAR
VAR_OUTPUT High : BOOL; Exact : BOOL; Warm : BOOL; Late : BOOL; Same : BOOL; END_VAR
High := Level > 80;
Exact := Level = 50;
Warm := Temp >= 60.5 OR -5.0 < Temp;
Late := T#2s <= Waited;
Same := Ok <> FALSE;
END_PROGRAM";
    let real = |x: f32| PlcValue::real(x).unwrap();
    let time = |ms: u64| PlcValue::Time(PlcTime::from_micros(ms * 1000));
    let mut script = Vec::new();
    for (level, temp, waited, ok) in [
        (0, -10.0, 0, false),
        (50, 60.5, 2000, true),
        (81, 60.4, 1999, false),
        (80, -5.0, 2001, true),
        (-32768, 0.0, 0, true),
    ] {
        script.push(vec![
            ("Level", PlcValue::Int(level)),
            ("Temp", real(temp)),
            ("Waited", time(waited)),
            ("Ok", b(ok)),
        ]);
    }
    equivalent(
        &lad,
        st,
        &script,
        &["High", "Exact", "Warm", "Late", "Same"],
    );
}

#[test]
fn every_element_reports_the_power_leaving_it() {
    // Start in parallel with the held motor, then not Stop; plus a TON on the rung.
    let lad = document(
        vec![
            var("Start", "BOOL", "input"),
            var("Stop", "BOOL", "input"),
            var("Motor", "BOOL", "output"),
        ],
        vec![rung(
            1,
            series(
                2,
                vec![
                    parallel(3, vec![no(4, "Start"), no(5, "Motor")]),
                    contact(6, "normallyClosed", "Stop"),
                    block(7, "TON", "Delay", json!({ "preset": k("T#200ms") })),
                ],
            ),
            vec![coil(8, "Motor", "output")],
        )],
    );
    let mut plc = Plc::lad(&lad);
    let power = ["LAD#4", "LAD#5", "LAD#3", "LAD#6", "LAD#7", "LAD#2"];
    // Start pressed: power reaches the timer, which is still timing.
    assert_eq!(
        plc.scan(&[("Start", T), ("Stop", F)], &power),
        [T, F, T, T, F, F]
    );
    assert_eq!(
        plc.scan(&[("Start", T), ("Stop", F)], &power),
        [T, F, T, T, F, F]
    );
    // After 200 ms the timer passes power: the rung and the coil are energized.
    assert_eq!(
        plc.scan(&[("Start", T), ("Stop", F)], &power),
        [T, F, T, T, T, T]
    );
    // Released, the motor holds itself through its own contact.
    assert_eq!(
        plc.scan(&[("Start", F), ("Stop", F)], &power),
        [F, T, T, T, T, T]
    );
    // Stop opens the path at its contact: power still reaches it through the parallel
    // block, but not the timer or the coil.
    assert_eq!(
        plc.scan(&[("Start", F), ("Stop", T)], &power),
        [F, T, T, F, F, F]
    );
    assert_eq!(plc.scan(&[("Start", F), ("Stop", F)], &power), [F; 6]);
}

fn errors(source: &str) -> Vec<(String, String, Option<u32>)> {
    compile_lad(source)
        .unwrap_err()
        .into_iter()
        .map(|d| {
            (
                d.code().to_owned(),
                d.message().to_owned(),
                d.source().map(|s| s.reference().element_id().get()),
            )
        })
        .collect()
}

#[test]
fn type_errors_are_reported_at_the_element_in_rust() {
    let vars = || {
        vec![
            var("Level", "INT", "input"),
            var("Temp", "REAL", "input"),
            var("Flag", "BOOL", "input"),
            var("Out", "BOOL", "output"),
            var("Wait", "TIME", "local"),
        ]
    };
    let one = |network: Value| {
        document(
            vars(),
            vec![rung(1, network, vec![coil(9, "Out", "output")])],
        )
    };
    // A contact on an INT.
    let e = errors(&one(no(2, "Level")));
    assert_eq!(
        e[0],
        (
            codes::TYPE.into(),
            "Rung 1: 'Level' is INT; contacts and coils use BOOL variables.".into(),
            Some(2)
        )
    );
    // Comparing INT with REAL is not silently converted.
    let e = errors(&one(compare(2, "gt", v("Level"), v("Temp"))));
    assert_eq!(e[0].0, codes::TYPE);
    assert!(e[0].1.contains("not INT and REAL"), "{e:?}");
    // Ordering BOOL values is not defined; EQ and NE are.
    assert!(
        errors(&one(compare(2, "gt", v("Flag"), k("TRUE"))))[0]
            .1
            .contains("use EQ or NE")
    );
    // Two literals have no type; a literal of the wrong type is named.
    assert!(
        errors(&one(compare(2, "eq", k("1"), k("1"))))[0]
            .1
            .contains("at least one variable")
    );
    let e = errors(&one(compare(2, "eq", v("Level"), k("2.5"))));
    assert!(e[0].1.contains("'2.5' is not an INT literal"), "{e:?}");
    assert!(
        errors(&one(compare(2, "lt", v("Wait"), k("5s"))))[0]
            .1
            .contains("T#5s")
    );
    // An unknown operand.
    assert_eq!(
        errors(&one(compare(2, "eq", v("Nope"), k("1"))))[0].0,
        codes::UNKNOWN
    );
}

#[test]
fn incomplete_or_conflicting_boxes_are_rejected() {
    let vars = || {
        vec![
            var("Start", "BOOL", "input"),
            var("Out", "BOOL", "output"),
            var("Count", "INT", "output"),
            var("Wait", "TIME", "output"),
            var("Raw", "INT", "input"),
        ]
    };
    let with = |nodes: Vec<Value>| {
        document(
            vars(),
            vec![rung(1, series(2, nodes), vec![coil(9, "Out", "output")])],
        )
    };
    let first = |nodes: Vec<Value>| errors(&with(nodes)).remove(0);
    let e = first(vec![block(3, "TON", "T1", json!({}))]);
    assert_eq!(
        (e.0.as_str(), e.1.as_str(), e.2),
        (codes::BLOCK, "Rung 1: TON 'T1' needs PT.", Some(3))
    );
    let e = first(vec![block(3, "TON", "T1", json!({ "preset": k("5") }))]);
    assert!(
        e.1.contains("TON.PT") && e.1.contains("not a TIME literal"),
        "{e:?}"
    );
    let e = first(vec![block(3, "CTU", "C1", json!({ "preset": v("Wait") }))]);
    assert!(
        e.1.contains("CTU.PV needs INT, but 'Wait' is TIME"),
        "{e:?}"
    );
    let e = first(vec![block(
        3,
        "TP",
        "P1",
        json!({ "preset": k("T#1s"), "reset": v("Start") }),
    )]);
    assert_eq!(e.1, "Rung 1: TP has no reset input.");
    let e = first(vec![block(
        3,
        "TON",
        "Start",
        json!({ "preset": k("T#1s") }),
    )]);
    assert_eq!(e.1, "Rung 1: instance 'Start' has the name of a variable.");
    let e = first(vec![
        block(3, "TON", "T1", json!({ "preset": k("T#1s") })),
        block(4, "TOF", "t1", json!({ "preset": k("T#1s") })),
    ]);
    assert_eq!(
        e.1,
        "Rung 1: instance 't1' is already used by another box (element 3)."
    );
    let e = first(vec![block(
        3,
        "CTU",
        "C1",
        json!({ "preset": k("3"), "output": "Raw" }),
    )]);
    assert_eq!(
        (e.0.as_str(), e.1.as_str()),
        (codes::TARGET, "Rung 1: CTU.CV cannot write input 'Raw'.")
    );
    let e = first(vec![block(
        3,
        "TON",
        "T1",
        json!({ "preset": k("T#1s"), "output": "Count" }),
    )]);
    assert!(e.1.contains("TON.ET is TIME, but 'Count' is INT"), "{e:?}");
    let e = first(vec![block(
        3,
        "TON",
        "My Timer",
        json!({ "preset": k("T#1s") }),
    )]);
    assert_eq!(e.1, "Rung 1: 'My Timer' is not a valid instance name.");
    // A coil on a non-BOOL variable.
    let source = document(
        vars(),
        vec![rung(1, no(2, "Start"), vec![coil(3, "Count", "set")])],
    );
    assert_eq!(errors(&source)[0].0, codes::TYPE);
}
