//! Deterministic behavioral grading over causal evidence.
//!
//! grading-core never executes a PLC, a process or a network and never inspects user
//! interface state. It consumes [`GradingEvidence`] produced by the engine: one
//! [`CycleFacts`] per PLC cycle holding the scan input image, the committed values, the
//! physical result after the process interval and the inputs staged for the next scan.
//! Rules read exactly the layer they name, so causal claims ("the PLC saw LevelHigh and
//! closed the valve") never depend on looking one row backwards.

use lab_model::exercise::{Comparison, Condition, Criterion, EvidenceLayer, Literal, ProcessEventKind, Rule};
use std::collections::BTreeMap;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EvidenceValue {
    Bool(bool),
    Number(f64),
}
impl EvidenceValue {
    fn number(self) -> f64 {
        match self {
            Self::Bool(b) => f64::from(u8::from(b)),
            Self::Number(n) => n,
        }
    }
}
impl std::fmt::Display for EvidenceValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bool(b) => f.write_str(if *b { "TRUE" } else { "FALSE" }),
            Self::Number(n) => write!(f, "{n}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleOutcome {
    Scanned,
    Faulted,
    NotScanned,
}

/// Facts about one PLC cycle, keyed by exercise evidence names.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CycleFacts {
    pub cycle: u64,
    pub start_us: u64,
    pub end_us: u64,
    pub outcome: Option<CycleOutcome>,
    pub inputs: BTreeMap<String, EvidenceValue>,
    pub outputs: BTreeMap<String, EvidenceValue>,
    pub process: BTreeMap<String, EvidenceValue>,
    pub next: BTreeMap<String, EvidenceValue>,
}
impl CycleFacts {
    pub fn layer(&self, layer: EvidenceLayer) -> &BTreeMap<String, EvidenceValue> {
        match layer {
            EvidenceLayer::Inputs => &self.inputs,
            EvidenceLayer::Outputs => &self.outputs,
            EvidenceLayer::Process => &self.process,
            EvidenceLayer::Next => &self.next,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceEvent {
    pub time_us: u64,
    pub component: String,
    pub kind: ProcessEventKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlcFaultEvidence {
    pub cycle: u64,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GradingEvidence {
    pub scenario: String,
    pub duration_us: u64,
    pub cycles: Vec<CycleFacts>,
    pub events: Vec<EvidenceEvent>,
    pub plc_fault: Option<PlcFaultEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CriterionResult {
    pub id: String,
    pub title: String,
    pub points: u32,
    pub earned: u32,
    pub passed: bool,
    /// Author feedback on failure, or a short confirmation.
    pub feedback: String,
    /// Concrete evidence: where and why the rule failed or passed.
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GradeReport {
    pub score: u32,
    pub max_score: u32,
    pub passed: bool,
    pub criteria: Vec<CriterionResult>,
}

fn seconds(us: u64) -> String {
    format!("{}.{:03} s", us / 1_000_000, (us % 1_000_000) / 1_000)
}

/// Result of evaluating a condition in one cycle; missing evidence is reported, not guessed.
enum Truth {
    Value(bool),
    Missing(String),
}

fn compare(op: Comparison, actual: EvidenceValue, expected: Literal) -> bool {
    match (actual, expected) {
        (EvidenceValue::Bool(a), Literal::Bool(b)) => match op {
            Comparison::Eq => a == b,
            Comparison::Ne => a != b,
            _ => false,
        },
        (actual, Literal::Number(b)) => {
            let a = actual.number();
            match op {
                Comparison::Eq => a == b,
                Comparison::Ne => a != b,
                Comparison::Gt => a > b,
                Comparison::Ge => a >= b,
                Comparison::Lt => a < b,
                Comparison::Le => a <= b,
            }
        }
        (EvidenceValue::Number(_), Literal::Bool(_)) => false,
    }
}

fn eval(condition: &Condition, facts: &CycleFacts) -> Truth {
    match condition {
        Condition::All { of } => {
            for c in of {
                match eval(c, facts) {
                    Truth::Value(true) => {}
                    other => return other,
                }
            }
            Truth::Value(true)
        }
        Condition::Any { of } => {
            let mut missing = None;
            for c in of {
                match eval(c, facts) {
                    Truth::Value(true) => return Truth::Value(true),
                    Truth::Value(false) => {}
                    Truth::Missing(m) => missing = Some(m),
                }
            }
            missing.map_or(Truth::Value(false), Truth::Missing)
        }
        Condition::Not { of } => match eval(of, facts) {
            Truth::Value(v) => Truth::Value(!v),
            missing => missing,
        },
        Condition::Compare { signal, layer, op, value } => match facts.layer(*layer).get(signal.as_str()) {
            Some(actual) => Truth::Value(compare(*op, *actual, *value)),
            None => Truth::Missing(format!("evidence '{signal}' is not available in layer {layer:?}")),
        },
    }
}

fn describe(condition: &Condition, facts: &CycleFacts) -> String {
    let mut parts = Vec::new();
    collect(condition, facts, &mut parts);
    parts.join(", ")
}
fn collect(condition: &Condition, facts: &CycleFacts, out: &mut Vec<String>) {
    match condition {
        Condition::All { of } | Condition::Any { of } => of.iter().for_each(|c| collect(c, facts, out)),
        Condition::Not { of } => collect(of, facts, out),
        Condition::Compare { signal, layer, .. } => {
            if let Some(v) = facts.layer(*layer).get(signal.as_str()) {
                let entry = format!("{signal}={v}");
                if !out.contains(&entry) {
                    out.push(entry);
                }
            }
        }
    }
}

/// Evaluate one rule; returns (passed, detail).
pub fn evaluate_rule(rule: &Rule, evidence: &GradingEvidence) -> (bool, String) {
    let cycles = &evidence.cycles;
    match rule {
        Rule::Always { condition, from_ms } => {
            let from = u64::from(*from_ms) * 1_000;
            for f in cycles.iter().filter(|f| f.start_us >= from) {
                match eval(condition, f) {
                    Truth::Value(true) => {}
                    Truth::Value(false) => {
                        return (false, format!("At {} (cycle {}): {}.", seconds(f.start_us), f.cycle, describe(condition, f)));
                    }
                    Truth::Missing(m) => return (false, m),
                }
            }
            (true, "Held in every observed cycle.".into())
        }
        Rule::Never { condition } => {
            for f in cycles {
                match eval(condition, f) {
                    Truth::Value(false) => {}
                    Truth::Value(true) => {
                        return (false, format!("Occurred at {} (cycle {}): {}.", seconds(f.start_us), f.cycle, describe(condition, f)));
                    }
                    Truth::Missing(m) => return (false, m),
                }
            }
            (true, "Never occurred.".into())
        }
        Rule::Eventually { condition, by_ms } => {
            let by = u64::from(*by_ms) * 1_000;
            for f in cycles.iter().filter(|f| f.start_us <= by) {
                match eval(condition, f) {
                    Truth::Value(true) => return (true, format!("Reached at {}.", seconds(f.start_us))),
                    Truth::Value(false) => {}
                    Truth::Missing(m) => return (false, m),
                }
            }
            (false, format!("Not reached by {}.", seconds(by)))
        }
        Rule::RespondsWithin { trigger, response, within_ms } => {
            let within = u64::from(*within_ms) * 1_000;
            let mut previous = false;
            let mut triggers = 0;
            for (k, f) in cycles.iter().enumerate() {
                let now = match eval(trigger, f) {
                    Truth::Value(v) => v,
                    Truth::Missing(m) => return (false, m),
                };
                if now && !previous {
                    triggers += 1;
                    let deadline = f.start_us + within;
                    let mut answered = false;
                    for g in cycles[k..].iter().take_while(|g| g.start_us <= deadline) {
                        match eval(response, g) {
                            Truth::Value(true) => {
                                answered = true;
                                break;
                            }
                            Truth::Value(false) => {}
                            Truth::Missing(m) => return (false, m),
                        }
                    }
                    if !answered {
                        return (
                            false,
                            format!(
                                "Triggered at {} (cycle {}: {}) but no response within {} ms.",
                                seconds(f.start_us),
                                f.cycle,
                                describe(trigger, f),
                                within_ms
                            ),
                        );
                    }
                }
                previous = now;
            }
            if triggers == 0 {
                (false, "The scenario never produced the trigger condition; check the required interface.".into())
            } else {
                (true, format!("Responded correctly to {triggers} trigger(s)."))
            }
        }
        Rule::StateAt { at_ms, condition } => {
            let at = u64::from(*at_ms) * 1_000;
            let Some(f) = cycles.iter().rev().find(|f| f.start_us <= at) else {
                return (false, format!("No cycle was observed at {}.", seconds(at)));
            };
            match eval(condition, f) {
                Truth::Value(true) => (true, format!("Correct at {}.", seconds(at))),
                Truth::Value(false) => (false, format!("At {}: {}.", seconds(at), describe(condition, f))),
                Truth::Missing(m) => (false, m),
            }
        }
        Rule::NoPlcFault => match &evidence.plc_fault {
            None => (true, "No runtime fault.".into()),
            Some(fault) => (false, format!("Runtime fault {} in cycle {}: {}", fault.code, fault.cycle, fault.message)),
        },
        Rule::EventAbsent { event } => match evidence.events.iter().find(|e| e.kind == *event) {
            None => (true, format!("No {event:?} event.")),
            Some(e) => (false, format!("{:?} at {} on '{}'.", e.kind, seconds(e.time_us), e.component)),
        },
    }
}

/// Grade criteria against evidence keyed by scenario ID. Every criterion must pass.
pub fn grade(criteria: &[Criterion], evidence: &BTreeMap<String, GradingEvidence>) -> GradeReport {
    let mut results = Vec::with_capacity(criteria.len());
    for c in criteria {
        let (passed, detail) = match evidence.get(c.scenario.as_str()) {
            Some(e) => evaluate_rule(&c.rule, e),
            None => (false, format!("Scenario '{}' was not executed.", c.scenario)),
        };
        results.push(CriterionResult {
            id: c.id.to_string(),
            title: c.title.clone(),
            points: c.points,
            earned: if passed { c.points } else { 0 },
            passed,
            feedback: if passed { "Passed.".into() } else { c.on_fail.clone() },
            detail,
        });
    }
    let score = results.iter().map(|r| r.earned).sum();
    let max_score = results.iter().map(|r| r.points).sum();
    GradeReport {
        score,
        max_score,
        passed: results.iter().all(|r| r.passed),
        criteria: results,
    }
}

/// Troubleshooting assessment inputs, all recorded by the engine.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InvestigationSummary {
    /// Diagnosis option IDs submitted, in order.
    pub diagnoses: Vec<String>,
    /// Evidence kinds the student inspected (for feedback only, not points).
    pub evidence_viewed: Vec<String>,
    /// Physical repairs applied.
    pub repairs: Vec<String>,
    /// Hidden faults still active when verification ran.
    pub remaining_faults: Vec<String>,
}

/// Combine diagnosis correctness, repair state and the behavioral verification report.
pub fn grade_troubleshooting(
    root_causes: &[String],
    investigation: &InvestigationSummary,
    verification: GradeReport,
) -> GradeReport {
    let correct = investigation.diagnoses.iter().any(|d| root_causes.contains(d));
    let first = investigation.diagnoses.first().is_some_and(|d| root_causes.contains(d));
    let wrong = investigation.diagnoses.iter().filter(|d| !root_causes.contains(d)).count();
    let mut criteria = vec![CriterionResult {
        id: "diagnosis".into(),
        title: "Root cause identified from evidence".into(),
        points: 30,
        earned: if first { 30 } else if correct { 15 } else { 0 },
        passed: correct,
        feedback: if first {
            "Correct diagnosis on the first attempt.".into()
        } else if correct {
            format!("Correct diagnosis after {wrong} incorrect assumption(s). Gather more evidence before concluding.")
        } else {
            "The root cause has not been identified yet.".into()
        },
        detail: format!(
            "{} diagnosis submission(s); evidence inspected: {}.",
            investigation.diagnoses.len(),
            if investigation.evidence_viewed.is_empty() { "none".to_string() } else { investigation.evidence_viewed.join(", ") }
        ),
    }];
    criteria.push(CriterionResult {
        id: "repair".into(),
        title: "Fault repaired".into(),
        points: 20,
        earned: if investigation.remaining_faults.is_empty() { 20 } else { 0 },
        passed: investigation.remaining_faults.is_empty(),
        feedback: if investigation.remaining_faults.is_empty() {
            "No physical fault remains.".into()
        } else {
            "A physical fault is still present in the installation.".into()
        },
        detail: format!("Repairs applied: {}.", if investigation.repairs.is_empty() { "none".to_string() } else { investigation.repairs.join(", ") }),
    });
    let scale = |earned: u32, max: u32| if max == 0 { 0 } else { earned * 50 / max };
    criteria.push(CriterionResult {
        id: "verification".into(),
        title: "System verified after repair".into(),
        points: 50,
        earned: scale(verification.score, verification.max_score),
        passed: verification.passed,
        feedback: if verification.passed { "Post-repair verification passed.".into() } else { "Post-repair verification failed.".into() },
        detail: verification
            .criteria
            .iter()
            .map(|c| format!("{}: {}", c.title, if c.passed { "pass" } else { c.detail.as_str() }))
            .collect::<Vec<_>>()
            .join(" | "),
    });
    let score = criteria.iter().map(|c| c.earned).sum();
    GradeReport {
        score,
        max_score: 100,
        passed: criteria.iter().all(|c| c.passed),
        criteria,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lab_model::Identifier;

    fn cond(json: &str) -> Condition {
        serde_json::from_str(json).unwrap()
    }
    fn facts(cycle: u64, start_ms: u64, input: bool, output: bool) -> CycleFacts {
        let mut f = CycleFacts { cycle, start_us: start_ms * 1000, end_us: (start_ms + 100) * 1000, ..Default::default() };
        f.inputs.insert("start".into(), EvidenceValue::Bool(input));
        f.outputs.insert("motor".into(), EvidenceValue::Bool(output));
        f.process.insert("level".into(), EvidenceValue::Number(f64::from(u32::try_from(cycle).unwrap()) * 10.0));
        f
    }
    fn evidence(pattern: &[(bool, bool)]) -> GradingEvidence {
        GradingEvidence {
            scenario: "s".into(),
            duration_us: pattern.len() as u64 * 100_000,
            cycles: pattern.iter().enumerate().map(|(i, (a, b))| facts(i as u64, i as u64 * 100, *a, *b)).collect(),
            ..Default::default()
        }
    }
    const START: &str = r#"{"type": "compare", "signal": "start", "layer": "inputs", "op": "eq", "value": true}"#;
    const MOTOR: &str = r#"{"type": "compare", "signal": "motor", "layer": "outputs", "op": "eq", "value": true}"#;

    #[test]
    fn responds_within_uses_the_scan_input_layer_directly() {
        let rule = Rule::RespondsWithin { trigger: cond(START), response: cond(MOTOR), within_ms: 100 };
        // Same-cycle response: the scan that saw Start commanded the motor.
        assert!(evaluate_rule(&rule, &evidence(&[(false, false), (true, true), (true, true)])).0);
        // One cycle late is within 100 ms.
        assert!(evaluate_rule(&rule, &evidence(&[(false, false), (true, false), (true, true)])).0);
        let (ok, detail) = evaluate_rule(&rule, &evidence(&[(false, false), (true, false), (true, false), (true, true)]));
        assert!(!ok);
        assert!(detail.contains("0.100 s"), "{detail}");
        let (ok, detail) = evaluate_rule(&rule, &evidence(&[(false, false)]));
        assert!(!ok && detail.contains("never produced"));
    }

    #[test]
    fn always_never_eventually_state_at_and_missing_evidence() {
        let e = evidence(&[(false, false), (true, true), (false, true)]);
        assert!(evaluate_rule(&Rule::Eventually { condition: cond(MOTOR), by_ms: 100 }, &e).0);
        assert!(!evaluate_rule(&Rule::Eventually { condition: cond(MOTOR), by_ms: 50 }, &e).0);
        assert!(evaluate_rule(&Rule::Always { condition: cond(MOTOR), from_ms: 100 }, &e).0);
        assert!(!evaluate_rule(&Rule::Always { condition: cond(MOTOR), from_ms: 0 }, &e).0);
        assert!(!evaluate_rule(&Rule::Never { condition: cond(MOTOR) }, &e).0);
        assert!(evaluate_rule(&Rule::StateAt { at_ms: 250, condition: cond(MOTOR) }, &e).0);
        let level = cond(r#"{"type": "compare", "signal": "level", "layer": "process", "op": "ge", "value": 20}"#);
        assert!(evaluate_rule(&Rule::Eventually { condition: level, by_ms: 300 }, &e).0);
        let missing = cond(r#"{"type": "compare", "signal": "ghost", "layer": "outputs", "op": "eq", "value": true}"#);
        let (ok, detail) = evaluate_rule(&Rule::Never { condition: missing }, &e);
        assert!(!ok && detail.contains("ghost"));
    }

    #[test]
    fn grade_requires_every_criterion_and_reports_scores() {
        let criteria: Vec<Criterion> = serde_json::from_str(&format!(
            r#"[{{"id": "a", "title": "A", "points": 60, "scenario": "s", "rule": {{"type": "noPlcFault"}}, "onFail": "fault"}},
               {{"id": "b", "title": "B", "points": 40, "scenario": "s", "rule": {{"type": "never", "condition": {MOTOR}}}, "onFail": "motor ran"}}]"#
        ))
        .unwrap();
        let mut map = BTreeMap::new();
        map.insert("s".to_string(), evidence(&[(false, false), (true, true)]));
        let report = grade(&criteria, &map);
        assert_eq!((report.score, report.max_score, report.passed), (60, 100, false));
        assert_eq!(report.criteria[1].feedback, "motor ran");
        let _ = Identifier::new("x").unwrap();
    }

    #[test]
    fn troubleshooting_scoring_is_deterministic() {
        let verification = GradeReport { score: 10, max_score: 10, passed: true, criteria: Vec::new() };
        let roots = vec!["field-power".to_string()];
        let mut inv = InvestigationSummary { diagnoses: vec!["field-power".into()], ..Default::default() };
        let r = grade_troubleshooting(&roots, &inv, verification.clone());
        assert_eq!((r.score, r.passed), (100, true));
        inv.diagnoses.insert(0, "wiring".into());
        let r = grade_troubleshooting(&roots, &inv, verification.clone());
        assert_eq!(r.score, 85);
        inv.remaining_faults.push("f1".into());
        let r = grade_troubleshooting(&roots, &inv, verification);
        assert!(!r.passed);
    }
}
