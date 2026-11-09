//! Project library documents (ADR-0045): the main program document and library documents
//! with FUNCTIONs and FUNCTION_BLOCKs compile into one canonical program for the one PLC
//! runtime. Every source anchor and diagnostic names its document.
use plc_runtime::{Runtime, RuntimeConfig, lower};
use plc_types::{PlcTime, PlcValue, diagnostics::Diagnostic, symbols::Scope};
use st_frontend::{
    CompiledProgram, CompilerConfig, MAX_LIBRARY_DOCUMENTS, codes, compile_st, compile_st_documents,
};

const MAIN: &str = "PROGRAM Main
VAR_INPUT S1 : BOOL; S2 : BOOL; END_VAR
VAR_OUTPUT H1 : BOOL; H2 : BOOL; H3 : BOOL; END_VAR
VAR T1 : Toggle; T2 : Toggle; END_VAR
T1(Press := S1, On => H1);
T2(Press := S2, On => H2);
H3 := Both(H1, H2);
END_PROGRAM
";
const BLOCKS: &str = "(* Each press flips On. *)
FUNCTION_BLOCK Toggle
VAR_INPUT
    Press : BOOL; (* The pushbutton *)
END_VAR
VAR_OUTPUT On : BOOL; END_VAR
VAR Edge : R_TRIG; END_VAR
Edge(CLK := Press);
IF Edge.Q THEN On := NOT On; END_IF;
END_FUNCTION_BLOCK
";
const FUNCTIONS: &str = "(* TRUE when both are TRUE. *)
FUNCTION Both : BOOL
VAR_INPUT A : BOOL; B : BOOL; END_VAR
Both := A AND B;
END_FUNCTION
";

fn compile(main: &str, library: &[&str]) -> CompiledProgram {
    compile_st_documents(main, library, CompilerConfig::default())
        .unwrap_or_else(|e| panic!("{e:#?}"))
}
fn errors(main: &str, library: &[&str]) -> Vec<Diagnostic> {
    compile_st_documents(main, library, CompilerConfig::default()).unwrap_err()
}
/// (document, line, code, message) of every diagnostic.
fn located(errors: &[Diagnostic]) -> Vec<(u32, u32, String, String)> {
    errors
        .iter()
        .map(|e| {
            let source = e.source().expect("a source location");
            (
                source.reference().document_id().get(),
                source.text_span().expect("a text span").start().line(),
                e.code().to_owned(),
                e.message().to_owned(),
            )
        })
        .collect()
}

struct Plc {
    compiled: CompiledProgram,
    runtime: Runtime,
}
impl Plc {
    fn new(compiled: CompiledProgram) -> Self {
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
    fn press(&mut self, name: &str) {
        for value in [true, false] {
            let id = self.id(name);
            self.runtime.set_input(id, PlcValue::Bool(value)).unwrap();
            self.runtime.single_scan().unwrap();
        }
    }
    fn get(&self, name: &str) -> PlcValue {
        self.runtime.snapshot().committed()[&self.id(name)]
    }
}

#[test]
fn library_documents_compile_with_the_program_into_one_runtime() {
    let mut plc = Plc::new(compile(MAIN, &[BLOCKS, FUNCTIONS]));
    let lamps = |plc: &Plc| ["H1", "H2", "H3"].map(|n| plc.get(n));
    let (on, off) = (PlcValue::Bool(true), PlcValue::Bool(false));
    plc.press("S1");
    assert_eq!(lamps(&plc), [on, off, off]);
    plc.press("S2");
    assert_eq!(lamps(&plc), [on, on, on]);
    plc.press("S1");
    assert_eq!(lamps(&plc), [off, on, off]);
    // The instance memory is program memory under the instance path.
    assert_eq!(plc.get("T2.On"), on);
}

#[test]
fn documents_compile_like_one_document_with_everything_in_it() {
    let together = format!("{BLOCKS}\n{FUNCTIONS}\n{MAIN}");
    let mut a = Plc::new(compile(MAIN, &[BLOCKS, FUNCTIONS]));
    let mut b = Plc::new(compile_st(&together, CompilerConfig::default()).unwrap());
    for input in ["S1", "S2", "S2", "S1", "S1", "S2"] {
        a.press(input);
        b.press(input);
        for lamp in ["H1", "H2", "H3", "T1.On", "T2.On"] {
            assert_eq!(a.get(lamp), b.get(lamp), "{lamp} after {input}");
        }
    }
    // The same canonical program, apart from where its code came from.
    assert_eq!(
        a.compiled.program().body().len(),
        b.compiled.program().body().len()
    );
}

#[test]
fn the_library_lists_each_pou_with_its_document_and_description() {
    let compiled = compile(MAIN, &[BLOCKS, FUNCTIONS]);
    let pous: Vec<_> = compiled
        .pous()
        .iter()
        .map(|p| {
            (
                p.name.as_str(),
                p.document,
                p.span.start().line(),
                p.description.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        pous,
        [
            ("Toggle", 2, 2, Some("Each press flips On.")),
            ("Both", 3, 2, Some("TRUE when both are TRUE.")),
        ]
    );
    assert_eq!(
        compiled.pous()[0].parameters[0].description.as_deref(),
        Some("The pushbutton")
    );
}

#[test]
fn a_runtime_fault_names_the_library_document_and_line() {
    let ratio = "FUNCTION Ratio : INT
VAR_INPUT A : INT; B : INT; END_VAR
Ratio := A / B;
END_FUNCTION";
    let main = "PROGRAM Main
VAR_INPUT X : INT; END_VAR
VAR_OUTPUT Y : INT; END_VAR
Y := Ratio(X, 0);
END_PROGRAM";
    let mut plc = Plc::new(compile(main, &["", ratio]));
    let fault = plc.runtime.single_scan().unwrap_err();
    let source = fault
        .diagnostic()
        .and_then(|d| d.source())
        .expect("a source location");
    assert_eq!(source.reference().document_id().get(), 3);
    assert_eq!(source.text_span().unwrap().start().line(), 3);
}

#[test]
fn errors_name_the_document_they_are_in() {
    // A type error inside a block of library document 2, reported once.
    let broken = BLOCKS.replace("On := NOT On;", "On := 5;");
    let typed: Vec<_> = located(&errors(MAIN, &[&broken, FUNCTIONS]))
        .into_iter()
        .map(|e| (e.0, e.1, e.2))
        .collect();
    assert_eq!(typed, [(2, 9, codes::TYPE.to_owned())]);
    // Syntax errors in several documents are all reported, each in its document.
    let both = errors(
        "PROGRAM Main\nVAR X : BOOL END_VAR\nEND_PROGRAM",
        &[BLOCKS, "FUNCTION Both : BOOL\nBoth := ;\nEND_FUNCTION"],
    );
    let places: Vec<_> = located(&both).into_iter().map(|e| (e.0, e.1)).collect();
    assert_eq!(places, [(1, 2), (3, 2)]);
    // A PROGRAM belongs in the main document.
    let program = located(&errors(
        MAIN,
        &[BLOCKS, FUNCTIONS, "\nPROGRAM Other\nEND_PROGRAM"],
    ));
    assert_eq!(program.len(), 1);
    assert_eq!((program[0].0, program[0].1), (4, 2));
    assert!(
        program[0].3.contains("library document holds FUNCTIONs"),
        "{program:?}"
    );
    // The second definition of a name is the duplicate, in its own document.
    let twice = located(&errors(MAIN, &[BLOCKS, FUNCTIONS, BLOCKS]));
    assert_eq!(
        twice
            .iter()
            .map(|e| (e.0, e.1, e.2.as_str()))
            .collect::<Vec<_>>(),
        [(4, 2, codes::DUPLICATE)]
    );
    // Recursion through two documents is reported where the cycle closes.
    let a = "FUNCTION A : BOOL\nVAR_INPUT X : BOOL; END_VAR\nA := B(X);\nEND_FUNCTION";
    let b = "FUNCTION B : BOOL\nVAR_INPUT X : BOOL; END_VAR\n\nB := A(X);\nEND_FUNCTION";
    let main = "PROGRAM Main\nVAR_OUTPUT Y : BOOL; END_VAR\nY := A(TRUE);\nEND_PROGRAM";
    let cycle = located(&errors(main, &[a, b]));
    assert!(
        cycle
            .iter()
            .any(|e| (e.0, e.1) == (3, 4) && e.3.contains("'A' uses itself through 'B'")),
        "{cycle:?}"
    );
    assert!(
        cycle
            .iter()
            .any(|e| (e.0, e.1) == (2, 3) && e.3.contains("'B' uses itself through 'A'")),
        "{cycle:?}"
    );
}

#[test]
fn empty_documents_are_allowed_and_the_number_of_documents_is_bounded() {
    let compiled = compile(MAIN, &["", "(* notes *)", BLOCKS, FUNCTIONS]);
    assert_eq!(compiled.pous().len(), 2);
    assert_eq!(compiled.pous()[0].document, 4);
    let many = vec![""; MAX_LIBRARY_DOCUMENTS + 1];
    let limit = errors(MAIN, &many);
    assert_eq!(limit.len(), 1);
    assert_eq!(limit[0].code(), codes::LIMIT);
}

#[test]
fn compiling_documents_twice_gives_the_same_program() {
    let a = compile(MAIN, &[BLOCKS, FUNCTIONS]);
    let b = compile(MAIN, &[BLOCKS, FUNCTIONS]);
    assert_eq!(a, b);
}
