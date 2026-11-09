//! The PID standard block (ADR-0046) called from Structured Text and executed by the one
//! PLC runtime: a closed loop, its parts online, compile errors and runtime faults.
use plc_runtime::{Runtime, RuntimeConfig, lower};
use plc_types::{PlcTime, PlcValue, symbols::Scope};
use st_frontend::{CompiledProgram, CompilerConfig, compile_st};

struct Plc {
    compiled: CompiledProgram,
    runtime: Runtime,
}
impl Plc {
    fn new(source: &str) -> Self {
        let compiled = compile_st(source, CompilerConfig::default())
            .unwrap_or_else(|e| panic!("{e:#?}\n{source}"));
        let plan = lower(&compiled.validated().unwrap()).unwrap();
        // A 100 ms task.
        let config = RuntimeConfig::new(PlcTime::from_micros(100_000), 100_000).unwrap();
        Self {
            runtime: Runtime::new(plan, config),
            compiled,
        }
    }
    fn real(&self, name: &str) -> f32 {
        let entry = Scope::Pou(self.compiled.program().entry());
        let id = self
            .compiled
            .program()
            .symbols()
            .resolve(entry, name)
            .unwrap_or_else(|_| panic!("no symbol {name}"))
            .id();
        match self.runtime.snapshot().committed()[&id] {
            PlcValue::Real(r) => r.get(),
            other => panic!("{name} is {other:?}"),
        }
    }
}

/// A heater on a first-order thermal plant written in ST: 0.1 s per scan, gain 0.5 K/%,
/// time constant 10 s, ambient 20 °C.
const LOOP: &str = "PROGRAM Main
VAR_OUTPUT Heater : REAL; END_VAR
VAR
    Loop : PID;
    Temp : REAL := 20.0;
    Integral : REAL;
    Saturated : BOOL;
END_VAR
Loop(SP := 60.0, PV := Temp, KP := 4.0, TI := T#8s, MV => Heater);
Integral := Loop.I;
Saturated := Loop.SAT;
Temp := Temp + (Heater * 0.5 - (Temp - 20.0)) * 0.01;
END_PROGRAM";

#[test]
fn a_pi_loop_brings_the_plant_to_its_setpoint() {
    let mut plc = Plc::new(LOOP);
    let mut peak = 0.0f32;
    for _ in 0..1200 {
        plc.runtime.single_scan().unwrap();
        peak = peak.max(plc.real("Temp"));
    }
    // 120 s later the integral has removed the offset a P controller would leave.
    let temp = plc.real("Temp");
    assert!((temp - 60.0).abs() < 0.2, "temperature {temp}");
    assert!(peak < 66.0, "overshoot to {peak}");
    // At 60 °C the plant needs 80 % heater power: all of it from the integral.
    assert!((plc.real("Heater") - 80.0).abs() < 1.0);
    assert!((plc.real("Integral") - plc.real("Heater")).abs() < 1.0);
}

#[test]
fn the_same_program_gives_bit_identical_runs() {
    let run = || {
        let mut plc = Plc::new(LOOP);
        (0..600)
            .map(|_| {
                plc.runtime.single_scan().unwrap();
                plc.real("Heater").to_bits()
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(run(), run());
}

#[test]
fn proportional_only_leaves_an_offset() {
    let mut plc = Plc::new(&LOOP.replace(", TI := T#8s", ""));
    for _ in 0..1200 {
        plc.runtime.single_scan().unwrap();
    }
    // Steady state of P control: Temp = 20 + 0.5 * 4 * (60 - Temp) → 46.7 °C.
    let temp = plc.real("Temp");
    assert!((temp - 46.67).abs() < 0.2, "temperature {temp}");
}

#[test]
fn calls_are_checked_like_every_standard_block() {
    let errors = |body: &str| {
        let source = LOOP.replace(
            "Loop(SP := 60.0, PV := Temp, KP := 4.0, TI := T#8s, MV => Heater);",
            body,
        );
        compile_st(&source, CompilerConfig::default())
            .unwrap_err()
            .iter()
            .map(|e| e.message().to_owned())
            .collect::<Vec<_>>()
            .join(" | ")
    };
    let missing = errors("Loop(PV := Temp, MV => Heater);");
    assert!(missing.contains("SP"), "{missing}");
    let mistyped = errors("Loop(SP := 60.0, PV := Temp, KP := 4, MV => Heater);");
    assert!(mistyped.contains("KP"), "{mistyped}");
    let wrong_output = errors("Loop(SP := 60.0, PV := Temp, MV => Saturated);");
    assert!(wrong_output.contains("MV"), "{wrong_output}");
}

#[test]
fn inverted_limits_fault_the_scan_at_the_call() {
    let source = LOOP.replace("KP := 4.0,", "KP := 4.0, MV_MIN := 50.0, MV_MAX := 10.0,");
    let mut plc = Plc::new(&source);
    let fault = plc.runtime.single_scan().unwrap_err();
    let diagnostic = fault.diagnostic().expect("a runtime fault");
    assert_eq!(diagnostic.code(), "PLC-RUNTIME-007");
    let line = diagnostic
        .source()
        .and_then(|s| s.text_span())
        .expect("a source location")
        .start()
        .line();
    assert_eq!(line, 9, "the PID call");
    // The faulted scan changed nothing.
    assert_eq!(plc.real("Heater"), 0.0);
}
