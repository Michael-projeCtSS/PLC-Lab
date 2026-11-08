//! User FUNCTIONs and FUNCTION_BLOCKs (ADR-0044): compiled by expansion into the canonical
//! IR and executed by the one PLC runtime.
use plc_runtime::{Runtime, RuntimeConfig, lower};
use plc_types::{PlcTime, PlcValue, symbols::Scope};
use st_frontend::{CompiledProgram, CompilerConfig, codes, compile_st};

fn compile(source: &str) -> CompiledProgram {
    compile_st(source, CompilerConfig::default()).unwrap_or_else(|e| panic!("{e:#?}\n{source}"))
}

struct Plc {
    compiled: CompiledProgram,
    runtime: Runtime,
}
impl Plc {
    fn new(source: &str) -> Self {
        let compiled = compile(source);
        let plan = lower(&compiled.validated().unwrap()).unwrap();
        let config = RuntimeConfig::new(PlcTime::from_micros(100_000), 100_000).unwrap();
        Self {
            runtime: Runtime::new(plan, config),
            compiled,
        }
    }
    fn id(&self, name: &str) -> plc_types::identity::SymbolId {
        let entry = Scope::Pou(self.compiled.program().entry());
        self.compiled
            .program()
            .symbols()
            .resolve(entry, name)
            .unwrap_or_else(|_| panic!("no symbol {name}"))
            .id()
    }
    fn set(&mut self, name: &str, value: PlcValue) {
        let id = self.id(name);
        self.runtime.set_input(id, value).unwrap();
    }
    fn scan(&mut self) {
        self.runtime.single_scan().unwrap();
    }
    fn get(&self, name: &str) -> PlcValue {
        self.runtime.snapshot().committed()[&self.id(name)]
    }
}

fn errors(source: &str) -> Vec<plc_types::diagnostics::Diagnostic> {
    compile_st(source, CompilerConfig::default()).unwrap_err()
}
fn fails(source: &str, code: &str, text: &str) {
    let errors = errors(source);
    assert!(
        errors
            .iter()
            .any(|e| e.code() == code && e.message().contains(text)),
        "expected {code} '{text}' in {errors:#?}"
    );
}

const MOTOR: &str = "
(* Start/stop with seal-in: Start starts, Stop (TRUE) stops. *)
FUNCTION_BLOCK MotorControl
VAR_INPUT
    Start : BOOL; (* Start command *)
    Stop : BOOL; (* Stop command, TRUE stops *)
END_VAR
VAR_OUTPUT
    Running : BOOL; (* Motor contactor *)
    Starts : INT;
END_VAR
VAR
    Edge : R_TRIG;
END_VAR
Edge(CLK := Start);
IF Edge.Q AND NOT Stop THEN
    Starts := Starts + 1;
END_IF;
Running := (Start OR Running) AND NOT Stop;
END_FUNCTION_BLOCK
";

#[test]
fn every_instance_keeps_its_own_memory_between_scans() {
    let mut plc = Plc::new(&format!(
        "{MOTOR}
PROGRAM Main
VAR_INPUT S1 : BOOL; S2 : BOOL; Halt : BOOL; END_VAR
VAR_OUTPUT M1 : BOOL; M2 : BOOL; Total : INT; END_VAR
VAR Pump1 : MotorControl; Pump2 : MotorControl; END_VAR
Pump1(Start := S1, Stop := Halt, Running => M1);
Pump2(Start := S2, Stop := Halt, Running => M2);
Total := Pump1.Starts + Pump2.Starts;
END_PROGRAM"
    ));
    plc.set("S1", PlcValue::Bool(true));
    plc.scan();
    plc.set("S1", PlcValue::Bool(false));
    plc.scan();
    // Pump1 sealed in, Pump2 never started: separate instance memory.
    assert_eq!(plc.get("M1"), PlcValue::Bool(true));
    assert_eq!(plc.get("M2"), PlcValue::Bool(false));
    assert_eq!(plc.get("Pump1.Running"), PlcValue::Bool(true));
    assert_eq!(plc.get("Pump1.Starts"), PlcValue::Int(1));
    assert_eq!(plc.get("Pump2.Starts"), PlcValue::Int(0));
    // Each instance has its own R_TRIG: one more start of Pump2 counts once.
    plc.set("S2", PlcValue::Bool(true));
    plc.scan();
    plc.scan();
    assert_eq!(plc.get("Total"), PlcValue::Int(2));
    plc.set("Halt", PlcValue::Bool(true));
    plc.scan();
    assert_eq!(
        (plc.get("M1"), plc.get("M2")),
        (PlcValue::Bool(false), PlcValue::Bool(false))
    );
}

#[test]
fn an_unbound_input_keeps_its_last_value() {
    let mut plc = Plc::new(&format!(
        "{MOTOR}
PROGRAM Main
VAR_INPUT S1 : BOOL; END_VAR
VAR_OUTPUT M1 : BOOL; END_VAR
VAR Pump1 : MotorControl; First : BOOL := TRUE; END_VAR
IF First THEN
    Pump1(Start := S1, Stop := FALSE);
    First := FALSE;
ELSE
    Pump1();
END_IF;
M1 := Pump1.Running;
END_PROGRAM"
    ));
    plc.set("S1", PlcValue::Bool(true));
    plc.scan();
    plc.set("S1", PlcValue::Bool(false));
    plc.scan();
    // The second call binds nothing: Start is still TRUE in the instance.
    assert_eq!(plc.get("Pump1.Start"), PlcValue::Bool(true));
    assert_eq!(plc.get("M1"), PlcValue::Bool(true));
}

#[test]
fn functions_are_pure_and_nest() {
    let mut plc = Plc::new(
        "
FUNCTION Clamp : INT
VAR_INPUT Value : INT; Low : INT; High : INT; END_VAR
VAR Calls : INT; END_VAR
Calls := Calls + 1;
Clamp := Value;
IF Value < Low THEN Clamp := Low; END_IF;
IF Value > High THEN Clamp := High; END_IF;
END_FUNCTION

FUNCTION Percent : INT
VAR_INPUT Raw : INT; END_VAR
Percent := Clamp(Raw / 10, 0, 100) + Clamp(0, 1, 1) - 1;
END_FUNCTION

PROGRAM Main
VAR_INPUT Raw : INT; END_VAR
VAR_OUTPUT Level : INT; High : BOOL; END_VAR
Level := Percent(Raw);
IF Clamp(Raw, 0, 500) = 500 THEN High := TRUE; ELSE High := FALSE; END_IF;
END_PROGRAM",
    );
    for (raw, level, high) in [(-30, 0, false), (455, 45, false), (2000, 100, true)] {
        plc.set("Raw", PlcValue::Int(raw));
        plc.scan();
        assert_eq!(plc.get("Level"), PlcValue::Int(level), "raw {raw}");
        assert_eq!(plc.get("High"), PlcValue::Bool(high), "raw {raw}");
    }
    // A local starts from its initial value on every call: no memory.
    let calls: Vec<PlcValue> = plc
        .compiled
        .program()
        .symbols()
        .symbols()
        .filter(|s| s.name().starts_with("Clamp#") && s.name().ends_with(".Calls"))
        .map(|s| plc.runtime.snapshot().committed()[&s.id()])
        .collect();
    assert_eq!(calls.len(), 3, "one temporary set per call site");
    assert!(calls.iter().all(|c| *c == PlcValue::Int(1)));
}

#[test]
fn var_in_out_writes_the_callers_variable() {
    let mut plc = Plc::new(
        "
FUNCTION_BLOCK Counter
VAR_INPUT Pulse : BOOL; END_VAR
VAR_IN_OUT Count : INT; END_VAR
VAR Edge : R_TRIG; END_VAR
Edge(CLK := Pulse);
IF Edge.Q THEN Count := Count + 1; END_IF;
END_FUNCTION_BLOCK

PROGRAM Main
VAR_INPUT Part : BOOL; END_VAR
VAR_OUTPUT Parts : INT; END_VAR
VAR Counting : Counter; END_VAR
Counting(Pulse := Part, Count := Parts);
END_PROGRAM",
    );
    for part in [true, false, true, false, true] {
        plc.set("Part", PlcValue::Bool(part));
        plc.scan();
    }
    assert_eq!(plc.get("Parts"), PlcValue::Int(3));
}

#[test]
fn standard_and_user_blocks_nest_inside_function_blocks() {
    let mut plc = Plc::new(&format!(
        "{MOTOR}
FUNCTION_BLOCK DelayedMotor
VAR_INPUT Request : BOOL; END_VAR
VAR_OUTPUT On : BOOL; END_VAR
VAR Wait : TON; Drive : MotorControl; END_VAR
Wait(IN := Request, PT := T#300ms);
Drive(Start := Wait.Q, Stop := NOT Request);
On := Drive.Running;
END_FUNCTION_BLOCK

PROGRAM Main
VAR_INPUT A : BOOL; B : BOOL; END_VAR
VAR_OUTPUT OnA : BOOL; OnB : BOOL; END_VAR
VAR First : DelayedMotor; Second : DelayedMotor; END_VAR
First(Request := A, On => OnA);
Second(Request := B, On => OnB);
END_PROGRAM"
    ));
    plc.set("A", PlcValue::Bool(true));
    for _ in 0..3 {
        plc.scan();
        assert_eq!(plc.get("OnA"), PlcValue::Bool(false));
    }
    plc.scan();
    plc.scan();
    // 100 ms cycles: the 300 ms timer of First only; Second never ran its timer.
    assert_eq!(plc.get("OnA"), PlcValue::Bool(true));
    assert_eq!(plc.get("OnB"), PlcValue::Bool(false));
    assert_eq!(plc.get("First.Drive.Running"), PlcValue::Bool(true));
}

#[test]
fn an_expanded_block_behaves_like_the_same_code_written_out() {
    let with_block = format!(
        "{MOTOR}
PROGRAM Main
VAR_INPUT S : BOOL; H : BOOL; END_VAR
VAR_OUTPUT M : BOOL; N : INT; END_VAR
VAR Pump : MotorControl; END_VAR
Pump(Start := S, Stop := H, Running => M, Starts => N);
END_PROGRAM"
    );
    let written_out = "
PROGRAM Main
VAR_INPUT S : BOOL; H : BOOL; END_VAR
VAR_OUTPUT M : BOOL; N : INT; END_VAR
VAR Running : BOOL; Starts : INT; Edge : R_TRIG; END_VAR
Edge(CLK := S);
IF Edge.Q AND NOT H THEN Starts := Starts + 1; END_IF;
Running := (S OR Running) AND NOT H;
M := Running;
N := Starts;
END_PROGRAM";
    let mut a = Plc::new(&with_block);
    let mut b = Plc::new(written_out);
    let stimulus = [
        (true, false),
        (false, false),
        (false, true),
        (true, true),
        (true, false),
        (false, false),
        (true, false),
    ];
    for (s, h) in stimulus {
        for plc in [&mut a, &mut b] {
            plc.set("S", PlcValue::Bool(s));
            plc.set("H", PlcValue::Bool(h));
            plc.scan();
        }
        assert_eq!(a.get("M"), b.get("M"));
        assert_eq!(a.get("N"), b.get("N"));
    }
}

#[test]
fn a_runtime_fault_points_into_the_function_body() {
    let source = "
FUNCTION Ratio : INT
VAR_INPUT A : INT; B : INT; END_VAR
Ratio := A / B;
END_FUNCTION

PROGRAM Main
VAR_INPUT X : INT; END_VAR
VAR_OUTPUT Y : INT; END_VAR
Y := Ratio(X, 0);
END_PROGRAM";
    let mut plc = Plc::new(source);
    plc.set("X", PlcValue::Int(5));
    let fault = plc.runtime.single_scan().unwrap_err();
    let span = fault
        .diagnostic()
        .and_then(|d| d.source())
        .and_then(|s| s.text_span())
        .expect("a source location");
    assert_eq!(span.start().line(), 4, "the division inside Ratio");
}

#[test]
fn the_library_lists_signatures_and_descriptions() {
    let compiled = compile(&format!(
        "{MOTOR}
(* Limits a value to a range. *)
FUNCTION Clamp : INT
VAR_INPUT Value : INT; (* Value to limit *) Low : INT; High : INT; END_VAR
Clamp := Value;
END_FUNCTION
PROGRAM Main
END_PROGRAM"
    ));
    let pous = compiled.pous();
    assert_eq!(pous.len(), 2);
    let motor = &pous[0];
    assert_eq!(
        (motor.name.as_str(), motor.kind.as_str()),
        ("MotorControl", "FUNCTION_BLOCK")
    );
    assert_eq!(
        motor.description.as_deref(),
        Some("Start/stop with seal-in: Start starts, Stop (TRUE) stops.")
    );
    let params: Vec<(&str, &str, &str, Option<&str>)> = motor
        .parameters
        .iter()
        .map(|p| {
            (
                p.name.as_str(),
                p.type_name.as_str(),
                p.direction.as_str(),
                p.description.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        params,
        [
            ("Start", "BOOL", "Input", Some("Start command")),
            ("Stop", "BOOL", "Input", Some("Stop command, TRUE stops")),
            ("Running", "BOOL", "Output", Some("Motor contactor")),
            ("Starts", "INT", "Output", None),
        ]
    );
    let clamp = &pous[1];
    assert_eq!(clamp.return_type.as_deref(), Some("INT"));
    assert_eq!(
        clamp.description.as_deref(),
        Some("Limits a value to a range.")
    );
    assert_eq!(
        clamp.parameters[0].description.as_deref(),
        Some("Value to limit")
    );
}

#[test]
fn errors_name_the_problem_once_at_its_place() {
    // An error inside a block is reported once, at the definition, however often it is
    // used, and also when nothing uses it.
    let broken = "
FUNCTION_BLOCK Broken
VAR_OUTPUT Q : BOOL; END_VAR
Q := Missing;
END_FUNCTION_BLOCK
PROGRAM Main
VAR A : Broken; B : Broken; END_VAR
A(); B();
END_PROGRAM";
    let found = errors(broken);
    assert_eq!(
        found
            .iter()
            .filter(|e| e.message().contains("Unknown variable 'Missing'"))
            .count(),
        1,
        "{found:#?}"
    );
    fails(
        "FUNCTION F : INT VAR_INPUT X : INT; END_VAR F := F(X); END_FUNCTION PROGRAM Main END_PROGRAM",
        codes::UNSUPPORTED,
        "'F' uses itself",
    );
    fails(
        "FUNCTION_BLOCK A VAR Inner : B; END_VAR END_FUNCTION_BLOCK FUNCTION_BLOCK B VAR Back : A; END_VAR END_FUNCTION_BLOCK PROGRAM Main END_PROGRAM",
        codes::UNSUPPORTED,
        "uses itself through",
    );
    fails(
        "PROGRAM Main VAR P : Pumpe; END_VAR END_PROGRAM",
        codes::UNSUPPORTED,
        "Type 'Pumpe' is unknown",
    );
    let clamp = "FUNCTION Clamp : INT VAR_INPUT V : INT; L : INT; END_VAR Clamp := V; END_FUNCTION";
    fails(
        &format!("{clamp} PROGRAM Main VAR X : INT; END_VAR X := Clamp(1); END_PROGRAM"),
        codes::BLOCK,
        "Clamp takes 2 argument(s) in this order: V, L",
    );
    fails(
        &format!("{clamp} PROGRAM Main VAR X : INT; END_VAR X := Clamp(1, TRUE); END_PROGRAM"),
        codes::TYPE,
        "Clamp argument L requires INT, received BOOL",
    );
    fails(
        "FUNCTION F : INT VAR_OUTPUT Y : INT; END_VAR F := 1; END_FUNCTION PROGRAM Main END_PROGRAM",
        codes::UNSUPPORTED,
        "returns its value through its own name",
    );
    fails(
        "FUNCTION F : BOOL VAR T : TON; END_VAR F := TRUE; END_FUNCTION PROGRAM Main END_PROGRAM",
        codes::UNSUPPORTED,
        "A function has no memory",
    );
    fails(
        "FUNCTION F : INT VAR_INPUT X : INT; END_VAR X := 2; F := X; END_FUNCTION PROGRAM Main END_PROGRAM",
        codes::TARGET,
        "Cannot write VAR_INPUT 'X'",
    );
    let block =
        format!("{MOTOR} PROGRAM Main VAR_OUTPUT M : BOOL; END_VAR VAR P : MotorControl; END_VAR");
    fails(
        &format!("{block} P(Running := M); END_PROGRAM"),
        codes::BLOCK,
        "Use := for MotorControl inputs",
    );
    fails(
        &format!("{block} M := P.Start; END_PROGRAM"),
        codes::BLOCK,
        "P.Start is not an output of MotorControl",
    );
    fails(
        &format!("{block} P(Speed := TRUE); END_PROGRAM"),
        codes::BLOCK,
        "Unknown MotorControl parameter 'Speed'",
    );
    let counter = "FUNCTION_BLOCK C VAR_IN_OUT N : INT; END_VAR N := N + 1; END_FUNCTION_BLOCK";
    fails(
        &format!("{counter} PROGRAM Main VAR K : C; X : INT; END_VAR K(); END_PROGRAM"),
        codes::BLOCK,
        "VAR_IN_OUT N of C must be bound to a variable",
    );
    fails(
        &format!("{counter} PROGRAM Main VAR K : C; X : INT; END_VAR K(N := X + 1); END_PROGRAM"),
        codes::BLOCK,
        "needs a variable, not an expression",
    );
    fails(
        "PROGRAM Main VAR_IN_OUT X : INT; END_VAR END_PROGRAM",
        codes::UNSUPPORTED,
        "VAR_IN_OUT belongs to function blocks",
    );
    fails(
        "FUNCTION Main : INT Main := 1; END_FUNCTION PROGRAM Main END_PROGRAM",
        codes::DUPLICATE,
        "already the name",
    );
    fails(
        "PROGRAM A END_PROGRAM PROGRAM B END_PROGRAM",
        codes::UNSUPPORTED,
        "Only one PROGRAM",
    );
    fails(
        &format!("{MOTOR} PROGRAM Main VAR X : BOOL; END_VAR X := MotorControl(TRUE); END_PROGRAM"),
        codes::BLOCK,
        "is a FUNCTION_BLOCK",
    );
}

#[test]
fn compiling_twice_gives_the_same_program() {
    let source = format!(
        "{MOTOR}
FUNCTION Twice : INT VAR_INPUT X : INT; END_VAR Twice := X * 2; END_FUNCTION
PROGRAM Main
VAR_INPUT S : BOOL; END_VAR
VAR_OUTPUT Y : INT; END_VAR
VAR P : MotorControl; END_VAR
P(Start := S, Stop := FALSE);
Y := Twice(P.Starts) + Twice(1);
END_PROGRAM"
    );
    assert_eq!(compile(&source), compile(&source));
}

#[test]
fn the_library_browsers_standard_examples_compile() {
    let examples: std::collections::BTreeMap<String, String> = serde_json::from_str(include_str!(
        "../../../packages/ui/src/workspace/standard-examples.json"
    ))
    .unwrap();
    assert_eq!(examples.len(), 7);
    for (block, example) in examples {
        let (declaration, body) = example.split_once('\n').unwrap();
        let program = format!(
            "PROGRAM Main
VAR_INPUT StartPB : BOOL; Running : BOOL; PartPresent : BOOL; ResetPB : BOOL; LoadPB : BOOL; END_VAR
VAR_OUTPUT Lamp : BOOL; Fan : BOOL; Horn : BOOL; BatchDone : BOOL; Empty : BOOL; END_VAR
VAR Count : INT; Stops : INT; END_VAR
{declaration}
{body}
END_PROGRAM"
        );
        assert!(
            compile_st(&program, CompilerConfig::default()).is_ok(),
            "{block}: {example}"
        );
    }
}
