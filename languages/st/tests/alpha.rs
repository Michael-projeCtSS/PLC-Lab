use plc_types::{
    PlcTime, PlcType, PlcValue,
    compatibility::BinaryOperator,
    source::TextSpan,
    symbols::{Scope, VariableCategory},
};
use st_frontend::{
    CompiledProgram, CompilerConfig,
    ast::{ExpressionKind, Literal, StatementKind},
    codes, compile_st,
    lexer::{Keyword, TokenKind, lex},
    parse_st,
};

fn compile(source: &str) -> CompiledProgram {
    compile_st(source, CompilerConfig::default()).unwrap_or_else(|e| panic!("{e:#?}\n{source}"))
}
fn source(declarations: &str, body: &str) -> String {
    format!("PROGRAM Main\n{declarations}\n{body}\nEND_PROGRAM")
}
fn local(kind: &str, body: &str) -> String {
    source(&format!("VAR Out : {kind}; END_VAR"), body)
}
fn failure(source: &str, code: &str) {
    let errors = compile_st(source, CompilerConfig::default()).unwrap_err();
    assert!(errors.iter().any(|e| e.code() == code), "{errors:#?}");
    assert!(
        errors
            .iter()
            .all(|e| e.severity() == plc_types::diagnostics::Severity::Error)
    );
}
fn text_at(source: &str, span: TextSpan) -> String {
    assert_eq!(span.start().line(), span.end().line());
    source
        .lines()
        .nth(span.start().line() as usize - 1)
        .unwrap()
        .chars()
        .skip(span.start().column() as usize - 1)
        .take((span.end().column() - span.start().column()) as usize)
        .collect()
}
fn symbol(compiled: &CompiledProgram, name: &str) -> plc_types::identity::SymbolId {
    compiled
        .program()
        .symbols()
        .resolve(Scope::Pou(compiled.program().entry()), name)
        .unwrap()
        .id()
}
fn runtime(compiled: &CompiledProgram) -> plc_runtime::Runtime {
    plc_runtime::Runtime::new(
        plc_runtime::lower(&compiled.validated().unwrap()).unwrap(),
        plc_runtime::RuntimeConfig::new(PlcTime::from_micros(100_000), 10_000).unwrap(),
    )
}
fn assigned(kind: &str, expression: &str) -> PlcValue {
    let compiled = compile(&local(kind, &format!("Out := {expression};")));
    let mut runtime = runtime(&compiled);
    runtime.single_scan().unwrap();
    runtime.snapshot().committed()[&symbol(&compiled, "Out")]
}
macro_rules! bad {
    ($name:ident, $source:expr, $code:expr) => {
        #[test]
        fn $name() {
            failure(&$source, $code);
        }
    };
}
macro_rules! valid {
    ($name:ident, $source:expr) => {
        #[test]
        fn $name() {
            compile(&$source);
        }
    };
}

#[test]
fn lexer_case_insensitive_keywords() {
    let tokens = lex("pRoGrAm mAiN eNd_PrOgRaM", CompilerConfig::default()).unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Keyword(Keyword::Program));
    assert_eq!(tokens[1].kind, TokenKind::Identifier("mAiN".into()));
}
#[test]
fn lexer_ascii_identifiers() {
    let tokens = lex("_A9 Z_0", CompilerConfig::default()).unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Identifier("_A9".into()));
}
valid!(
    lexer_block_comments,
    source(
        "(* declarations *) VAR A : BOOL; END_VAR",
        "(* body *) A := TRUE;"
    )
);
valid!(
    lexer_line_comments,
    source(
        "// ą ż – 😀\nVAR A : BOOL; END_VAR",
        "A := FALSE; // comment\n"
    )
);
#[test]
fn lexer_unicode_comments_preserve_scalar_columns() {
    let text = "(* ą ż – 😀 *) @";
    let errors = lex(text, CompilerConfig::default()).unwrap_err();
    let span = errors[0].source().unwrap().text_span().unwrap();
    assert_eq!(span.start().column(), text.chars().count() as u32);
    assert_eq!(text_at(text, span), "@");
}
bad!(
    lexer_invalid_characters,
    "PROGRAM Main @ END_PROGRAM",
    codes::LEXICAL
);
bad!(
    lexer_unterminated_comment,
    "PROGRAM Main (* ą 😀",
    codes::LEXICAL
);
bad!(
    lexer_nested_comments_are_explicitly_rejected,
    "PROGRAM Main (* outer (* inner *) *) END_PROGRAM",
    codes::LEXICAL
);
#[test]
fn lexer_bool_literals() {
    let tokens = lex("tRuE false", CompilerConfig::default()).unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Literal(Literal::Bool(true)));
    assert_eq!(tokens[1].kind, TokenKind::Literal(Literal::Bool(false)));
}
#[test]
fn lexer_int_is_unsigned_intermediate_text() {
    let tokens = lex("-32768 32768", CompilerConfig::default()).unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Minus);
    assert_eq!(
        tokens[1].kind,
        TokenKind::Literal(Literal::Integer("32768".into()))
    );
}
#[test]
fn lexer_real_syntax() {
    for value in ["0.0", "1.5", "1.0E3", "1.0e-3"] {
        assert_eq!(
            lex(value, CompilerConfig::default()).unwrap()[0].kind,
            TokenKind::Literal(Literal::Real(value.into()))
        );
    }
}
#[test]
fn lexer_time_prefix_case_insensitive() {
    for value in ["T#3s", "time#3S", "t#1h2m3s"] {
        assert!(matches!(
            lex(value, CompilerConfig::default()).unwrap()[0].kind,
            TokenKind::Literal(Literal::Time(_))
        ));
    }
}
bad!(
    lexer_malformed_time,
    local("TIME", "Out := T#1.5s;"),
    codes::LITERAL
);
#[test]
fn lexer_source_and_token_limits() {
    let config = CompilerConfig::new(3, 100, 32, 32).unwrap();
    assert_eq!(lex("abcd", config).unwrap_err()[0].code(), codes::LIMIT);
    let config = CompilerConfig::new(100, 3, 32, 32).unwrap();
    assert_eq!(
        lex("PROGRAM Main END_PROGRAM", config).unwrap_err()[0].code(),
        codes::LIMIT
    );
    assert!(lex("", CompilerConfig::new(1, 1, 1, 1).unwrap()).is_ok());
}

valid!(parser_minimal_program, "PROGRAM OriginalName END_PROGRAM");
#[test]
fn parser_input_section() {
    let ast = parse_st(
        &source("VAR_INPUT Start : BOOL; END_VAR", ""),
        CompilerConfig::default(),
    )
    .unwrap();
    assert_eq!(ast.declarations[0].role, VariableCategory::Input);
}
#[test]
fn parser_output_section() {
    let ast = parse_st(
        &source("VAR_OUTPUT Motor : BOOL; END_VAR", ""),
        CompilerConfig::default(),
    )
    .unwrap();
    assert_eq!(ast.declarations[0].role, VariableCategory::Output);
}
#[test]
fn parser_local_section() {
    let ast = parse_st(&local("REAL", ""), CompilerConfig::default()).unwrap();
    assert_eq!(ast.declarations[0].role, VariableCategory::Local);
}
valid!(
    parser_nested_if,
    local(
        "BOOL",
        "IF TRUE THEN IF FALSE THEN Out := FALSE; ELSE Out := TRUE; END_IF; END_IF;"
    )
);
#[test]
fn parser_boolean_precedence() {
    let ast = parse_st(
        &local("BOOL", "Out := TRUE OR FALSE AND FALSE;"),
        CompilerConfig::default(),
    )
    .unwrap();
    let StatementKind::Assignment(_, expr) = &ast.body[0].kind else {
        panic!()
    };
    let ExpressionKind::Binary(BinaryOperator::Or, _, right) = &expr.kind else {
        panic!()
    };
    assert!(matches!(
        right.kind,
        ExpressionKind::Binary(BinaryOperator::And, _, _)
    ));
}
#[test]
fn parser_arithmetic_precedence() {
    assert_eq!(assigned("INT", "1 + 2 * 3"), PlcValue::Int(7));
}
#[test]
fn parser_parentheses() {
    assert_eq!(assigned("INT", "(1 + 2) * 3"), PlcValue::Int(9));
}
valid!(
    parser_ton_declaration,
    source("VAR Delay : TON; END_VAR", "")
);
valid!(
    parser_ton_call,
    source(
        "VAR Delay : TON; END_VAR",
        "Delay(IN := TRUE, PT := T#300ms);"
    )
);
valid!(
    parser_arbitrary_named_ton_order,
    source(
        "VAR Delay : TON; QOut : BOOL; Elapsed : TIME; END_VAR",
        "Delay(ET => Elapsed, PT := T#300ms, Q => QOut, IN := TRUE);"
    )
);
bad!(
    parser_missing_semicolon,
    local("BOOL", "Out := TRUE"),
    codes::SYNTAX
);
bad!(
    parser_missing_end_var,
    source("VAR Out : BOOL;", "Out := TRUE;"),
    codes::SYNTAX
);
bad!(
    parser_missing_end_if,
    local("BOOL", "IF TRUE THEN Out := TRUE;"),
    codes::SYNTAX
);
bad!(
    parser_missing_end_program,
    "PROGRAM Main VAR A : BOOL; END_VAR A := TRUE;",
    codes::SYNTAX
);
#[test]
fn parser_nesting_and_flat_expression_limits() {
    for expr in [
        format!("{}TRUE{}", "(".repeat(200), ")".repeat(200)),
        vec!["1"; 200].join("+"),
    ] {
        let kind = if expr.contains("TRUE") { "BOOL" } else { "INT" };
        failure(&local(kind, &format!("Out := {expr};")), codes::LIMIT);
    }
}
#[test]
fn parser_recovers_multiple_obvious_errors_and_caps_diagnostics() {
    let src = local("BOOL", "Out := ; Out := ;");
    let errors = compile_st(&src, CompilerConfig::default()).unwrap_err();
    assert!(errors.len() >= 2);
    let errors = compile_st(&src, CompilerConfig::new(65_536, 16_384, 32, 1).unwrap()).unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code(), codes::LIMIT);
}
bad!(
    multiple_programs_never_select_one,
    "PROGRAM A END_PROGRAM PROGRAM B END_PROGRAM",
    codes::UNSUPPORTED
);
#[test]
fn config_is_checked_nonzero_and_bounded() {
    for values in [
        (0, 1, 1, 1),
        (1, 0, 1, 1),
        (1, 1, 0, 1),
        (1, 1, 1, 0),
        (2_000_000, 1, 1, 1),
        (1, 1, 100, 1),
    ] {
        assert!(CompilerConfig::new(values.0, values.1, values.2, values.3).is_err());
    }
}

bad!(
    semantic_duplicate_variable,
    source("VAR Start : BOOL; start : BOOL; END_VAR", ""),
    codes::DUPLICATE
);
bad!(
    semantic_duplicate_instance,
    source("VAR Delay : TON; DELAY : TON; END_VAR", ""),
    codes::DUPLICATE
);
bad!(
    semantic_variable_instance_name_collision,
    source("VAR Delay : BOOL; delay : TON; END_VAR", ""),
    codes::DUPLICATE
);
bad!(
    semantic_unknown_symbol,
    local("BOOL", "Out := Missing;"),
    codes::UNKNOWN
);
#[test]
fn semantic_case_resolution_preserves_source_names() {
    let compiled = compile("pRoGrAm mAiN var OuT : bool; end_var oUt := true; end_program");
    assert_eq!(compiled.program_name(), "mAiN");
    assert_eq!(compiled.declarations()[0].name, "OuT");
    assert_eq!(assigned("BOOL", "TRUE"), PlcValue::Bool(true));
}
bad!(
    semantic_input_write_is_rejected,
    source("VAR_INPUT Start : BOOL; END_VAR", "Start := TRUE;"),
    codes::TARGET
);
#[test]
fn semantic_bool_assignment() {
    assert_eq!(
        assigned("BOOL", "NOT FALSE AND TRUE OR FALSE"),
        PlcValue::Bool(true)
    );
}
#[test]
fn semantic_int_assignment() {
    assert_eq!(assigned("INT", "32767"), PlcValue::Int(32767));
}
#[test]
fn semantic_real_assignment_and_canonical_zero() {
    assert_eq!(assigned("REAL", "-3.25"), PlcValue::real(-3.25).unwrap());
    let PlcValue::Real(value) = assigned("REAL", "-0.0") else {
        panic!()
    };
    assert_eq!(value.get().to_bits(), 0);
}
#[test]
fn semantic_time_exact_components() {
    for (literal, expected) in [
        ("T#0s", 0),
        ("T#500ms", 500_000),
        ("T#3s", 3_000_000),
        ("T#1m30s", 90_000_000),
        ("T#1h2m3s", 3_723_000_000),
        ("T#2s500ms", 2_500_000),
        ("T#100us", 100),
        ("TIME#1D1H1M1S1MS1US", 90_061_001_001),
    ] {
        assert_eq!(
            assigned("TIME", literal),
            PlcValue::Time(PlcTime::from_micros(expected))
        );
    }
}
bad!(
    semantic_incompatible_assignment,
    local("BOOL", "Out := 1;"),
    codes::TYPE
);
bad!(
    semantic_if_requires_bool,
    local("BOOL", "IF 1 THEN Out := TRUE; END_IF;"),
    codes::TYPE
);
bad!(
    semantic_invalid_boolean_operand,
    local("BOOL", "Out := TRUE AND 1;"),
    codes::TYPE
);
bad!(
    semantic_arithmetic_mismatch,
    local("REAL", "Out := 1 + 2.0;"),
    codes::TYPE
);
#[test]
fn semantic_min_int_is_valid() {
    assert_eq!(assigned("INT", "-32768"), PlcValue::Int(i16::MIN));
}
bad!(
    semantic_positive_int_overflow,
    local("INT", "Out := 32768;"),
    codes::LITERAL
);
bad!(
    semantic_negative_int_overflow,
    local("INT", "Out := -32769;"),
    codes::LITERAL
);
bad!(
    semantic_nonfinite_real_is_rejected,
    local("REAL", "Out := 1.0E999;"),
    codes::LITERAL
);
bad!(
    semantic_unknown_ton_instance,
    local("BOOL", "Missing(IN := TRUE, PT := T#1s);"),
    codes::BLOCK
);
bad!(
    semantic_ton_duplicate_binding,
    source(
        "VAR D : TON; END_VAR",
        "D(IN := TRUE, in := FALSE, PT := T#1s);"
    ),
    codes::BLOCK
);
bad!(
    semantic_ton_missing_in,
    source("VAR D : TON; END_VAR", "D(PT := T#1s);"),
    codes::BLOCK
);
bad!(
    semantic_ton_missing_pt,
    source("VAR D : TON; END_VAR", "D(IN := TRUE);"),
    codes::BLOCK
);
bad!(
    semantic_ton_wrong_in,
    source("VAR D : TON; END_VAR", "D(IN := 1, PT := T#1s);"),
    codes::BLOCK
);
bad!(
    semantic_ton_wrong_pt,
    source("VAR D : TON; END_VAR", "D(IN := TRUE, PT := 1);"),
    codes::BLOCK
);
bad!(
    semantic_ton_wrong_q_target,
    source(
        "VAR D : TON; A : INT; END_VAR",
        "D(IN := TRUE, PT := T#1s, Q => A);"
    ),
    codes::BLOCK
);
bad!(
    semantic_ton_wrong_et_target,
    source(
        "VAR D : TON; A : BOOL; END_VAR",
        "D(IN := TRUE, PT := T#1s, ET => A);"
    ),
    codes::BLOCK
);
bad!(
    semantic_ton_output_cannot_write_input,
    source(
        "VAR D : TON; END_VAR VAR_INPUT A : BOOL; END_VAR",
        "D(IN := TRUE, PT := T#1s, Q => A);"
    ),
    codes::BLOCK
);
bad!(
    semantic_ton_wrong_direction,
    source("VAR D : TON; A : BOOL; END_VAR", "D(IN => A, PT := T#1s);"),
    codes::BLOCK
);
bad!(
    semantic_ton_unknown_parameter,
    source(
        "VAR D : TON; END_VAR",
        "D(IN := TRUE, PT := T#1s, Bad := TRUE);"
    ),
    codes::BLOCK
);
bad!(
    semantic_ton_only_local,
    source("VAR_INPUT D : TON; END_VAR", ""),
    codes::BLOCK
);
bad!(
    semantic_unsupported_type,
    local("DINT", ""),
    codes::UNSUPPORTED
);
#[test]
fn time_invalid_order_repetition_sign_fraction_unit_and_overflow() {
    for literal in [
        "T#",
        "T#-1s",
        "T#1.5s",
        "T#1ns",
        "T#1s1m",
        "T#1s2s",
        "T#999999999999999999999h",
        "T#18446744073709551615us1us",
    ] {
        failure(
            &local("TIME", &format!("Out := {literal};")),
            codes::LITERAL,
        );
    }
}
#[test]
fn real_malformed_and_nonfinite_words_rejected() {
    for literal in ["1.", "1.0e", "1e3", "NaN", "INF", "Infinity"] {
        assert!(
            compile_st(
                &local("REAL", &format!("Out := {literal};")),
                CompilerConfig::default()
            )
            .is_err()
        );
    }
}

#[test]
fn pipeline_deterministic_compilation_and_id_order() {
    let text = source(
        "VAR_INPUT Start : BOOL; END_VAR VAR D : TON; A : INT; E : TON; END_VAR VAR_OUTPUT Q : BOOL; END_VAR",
        "D(IN := Start, PT := T#300ms, Q => Q); E(PT := T#0s, IN := Start);",
    );
    let a = compile(&text);
    assert_eq!(a, compile(&text));
    assert_eq!(a.program().entry().get(), 1);
    assert_eq!(
        a.program()
            .symbols()
            .symbols()
            .map(|s| s.name())
            .collect::<Vec<_>>(),
        ["Start", "A", "Q"]
    );
    assert_eq!(
        a.program()
            .function_blocks()
            .instances()
            .map(|i| (i.id().get(), i.name()))
            .collect::<Vec<_>>(),
        [(1, "D"), (2, "E")]
    );
    assert_eq!(a.program().function_blocks().definitions().count(), 1);
    assert!(
        a.program()
            .sources()
            .elements()
            .all(|s| s.reference().document_id().get() == 1)
    );
}
#[test]
fn pipeline_motor_tracks_start() {
    let compiled = compile(&source(
        "VAR_INPUT Start : BOOL; END_VAR VAR_OUTPUT Motor : BOOL; END_VAR",
        "Motor := Start;",
    ));
    let mut runtime = runtime(&compiled);
    for (input, expected) in [(false, false), (true, true), (false, false)] {
        runtime
            .set_input(symbol(&compiled, "Start"), PlcValue::Bool(input))
            .unwrap();
        let scan = runtime.single_scan().unwrap();
        assert_eq!(
            scan.snapshot().outputs()[&symbol(&compiled, "Motor")],
            PlcValue::Bool(expected)
        );
    }
}
#[test]
fn pipeline_start_and_not_stop_truth_table() {
    let compiled = compile(&source(
        "VAR_INPUT Start : BOOL; Stop : BOOL; END_VAR VAR_OUTPUT Motor : BOOL; END_VAR",
        "Motor := Start AND NOT Stop;",
    ));
    let mut runtime = runtime(&compiled);
    for (start, stop, expected) in [
        (false, false, false),
        (false, true, false),
        (true, false, true),
        (true, true, false),
    ] {
        runtime
            .set_input(symbol(&compiled, "Start"), PlcValue::Bool(start))
            .unwrap();
        runtime
            .set_input(symbol(&compiled, "Stop"), PlcValue::Bool(stop))
            .unwrap();
        runtime.single_scan().unwrap();
        assert_eq!(
            runtime.snapshot().outputs()[&symbol(&compiled, "Motor")],
            PlcValue::Bool(expected)
        );
    }
}
#[test]
fn pipeline_if_selects_existing_runtime_branches() {
    let compiled = compile(&source(
        "VAR_INPUT LevelHigh : BOOL; END_VAR VAR_OUTPUT Pump : BOOL; END_VAR",
        "IF LevelHigh THEN Pump := FALSE; ELSE Pump := TRUE; END_IF;",
    ));
    let mut runtime = runtime(&compiled);
    for (input, expected) in [(false, true), (true, false)] {
        runtime
            .set_input(symbol(&compiled, "LevelHigh"), PlcValue::Bool(input))
            .unwrap();
        runtime.single_scan().unwrap();
        assert_eq!(
            runtime.snapshot().outputs()[&symbol(&compiled, "Pump")],
            PlcValue::Bool(expected)
        );
    }
}
#[test]
fn pipeline_exact_ton_timeline_and_binding_order_equivalence() {
    let declarations = "VAR_INPUT Start : BOOL; END_VAR VAR_OUTPUT Motor : BOOL; Elapsed : TIME; END_VAR VAR Delay : TON; END_VAR";
    let a = compile(&source(
        declarations,
        "Delay(IN := Start, PT := T#300ms, Q => Motor, ET => Elapsed);",
    ));
    let b = compile(&source(
        declarations,
        "Delay(ET => Elapsed, PT := T#300ms, IN := Start, Q => Motor);",
    ));
    let mut a_runtime = runtime(&a);
    let mut b_runtime = runtime(&b);
    a_runtime
        .set_input(symbol(&a, "Start"), PlcValue::Bool(true))
        .unwrap();
    b_runtime
        .set_input(symbol(&b, "Start"), PlcValue::Bool(true))
        .unwrap();
    for (index, (elapsed, q)) in [
        (0, false),
        (100_000, false),
        (200_000, false),
        (300_000, true),
        (300_000, true),
    ]
    .into_iter()
    .enumerate()
    {
        let left = a_runtime.single_scan().unwrap();
        let right = b_runtime.single_scan().unwrap();
        assert_eq!(left, right);
        assert_eq!(
            left.snapshot().outputs()[&symbol(&a, "Motor")],
            PlcValue::Bool(q)
        );
        assert_eq!(
            left.snapshot().outputs()[&symbol(&a, "Elapsed")],
            PlcValue::Time(PlcTime::from_micros(elapsed))
        );
        assert_eq!(
            left.snapshot().time().as_micros(),
            (index as u64 + 1) * 100_000
        );
    }
}
#[test]
fn pipeline_invalid_source_has_no_executable_wrapper() {
    assert!(compile_st(&local("BOOL", "Out := 1;"), CompilerConfig::default()).is_err());
}
#[test]
fn pipeline_runtime_fault_resolves_original_st_expression() {
    let text = local("INT", "Out := 1 / 0;");
    let compiled = compile(&text);
    let mut runtime = runtime(&compiled);
    let error = runtime.single_scan().unwrap_err();
    let diagnostic = error.diagnostic().unwrap();
    let location = diagnostic.source().unwrap();
    assert_eq!(
        compiled.program().sources().get(location.reference()),
        Some(location)
    );
    assert_eq!(text_at(&text, location.text_span().unwrap()), "1 / 0");
    assert_eq!(diagnostic.code(), "PLC-RUNTIME-003");
}
#[test]
fn semantic_error_after_unicode_comment_has_correct_range() {
    let text = local("BOOL", "(* ą ż – 😀 *) Out := Missing;");
    let errors = compile_st(&text, CompilerConfig::default()).unwrap_err();
    let diagnostic = errors.iter().find(|e| e.code() == codes::UNKNOWN).unwrap();
    assert_eq!(
        text_at(&text, diagnostic.source().unwrap().text_span().unwrap()),
        "Missing"
    );
}
#[test]
fn crlf_and_lone_cr_comments_preserve_lines() {
    for newline in ["\n", "\r\n", "\r"] {
        let text = format!(
            "PROGRAM Main{newline}// 😀{newline}VAR A : BOOL; END_VAR{newline}A := Missing;{newline}END_PROGRAM"
        );
        let errors = compile_st(&text, CompilerConfig::default()).unwrap_err();
        assert_eq!(
            errors[0]
                .source()
                .unwrap()
                .text_span()
                .unwrap()
                .start()
                .line(),
            4
        );
    }
}
#[test]
fn comparisons_and_numeric_unary_use_accepted_types() {
    for expr in [
        "1 = 1",
        "1 <> 2",
        "1 < 2",
        "1 <= 1",
        "2 > 1",
        "2 >= 2",
        "T#1s < T#2s",
        "1.0 < 2.0",
    ] {
        assert_eq!(assigned("BOOL", expr), PlcValue::Bool(true));
    }
    let compiled = compile(&local("INT", "Out := 3; Out := -Out;"));
    let mut runtime = runtime(&compiled);
    runtime.single_scan().unwrap();
    assert_eq!(
        runtime.snapshot().committed()[&symbol(&compiled, "Out")],
        PlcValue::Int(-3)
    );
    assert_eq!(
        compiled
            .program()
            .symbols()
            .get(symbol(&compiled, "Out"))
            .unwrap()
            .plc_type(),
        PlcType::Int
    );
}
#[test]
fn bounded_malformed_inputs_never_panic_or_hang() {
    let mut seed = 123u64;
    let alphabet = [
        'P', 'R', 'O', 'G', 'A', 'M', ' ', ':', ';', '(', ')', '-', '1', '\n', '😀', '*', '=', 'I',
        'F',
    ];
    for _ in 0..500 {
        let text: String = (0..96)
            .map(|_| {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                alphabet[(seed as usize) % alphabet.len()]
            })
            .collect();
        let _ = compile_st(&text, CompilerConfig::default());
    }
}
