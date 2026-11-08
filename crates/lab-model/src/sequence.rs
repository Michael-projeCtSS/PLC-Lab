//! Sequence definitions (ADR-0043): the documented steps, transitions, permissives and
//! interlocks of a step sequence that a PLC program implements. The definition is data;
//! the PLC program stays the only controller and `sequence-core` observes it.
//!
//! Conditions reuse the exercise condition model (typed comparisons of evidence signals),
//! so nothing in content is ever evaluated as an expression.

use crate::{exercise::Condition, ids::Identifier};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SequenceDefinition {
    pub id: Identifier,
    pub name: String,
    /// Evidence name of the INT program variable holding the active step number.
    pub step_signal: Identifier,
    /// Evidence name of a BOOL program variable that is TRUE while the program holds.
    #[serde(default)]
    pub held_signal: Option<Identifier>,
    pub steps: Vec<SequenceStep>,
    pub transitions: Vec<SequenceTransition>,
    #[serde(default)]
    pub interlocks: Vec<SequenceInterlock>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StepRole {
    Initial,
    #[default]
    Normal,
    Final,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SequenceStep {
    pub id: Identifier,
    /// The step variable's value while this step is active.
    pub number: i16,
    pub name: String,
    #[serde(default)]
    pub role: StepRole,
    /// World time the step may stay active (not held) before it times out.
    #[serde(default)]
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SequenceTransition {
    pub id: Identifier,
    pub from: Identifier,
    pub to: Identifier,
    pub name: String,
    pub condition: Condition,
    #[serde(default)]
    pub permissives: Vec<Permissive>,
}

/// A condition that must hold before a transition may proceed. Never a fault.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Permissive {
    pub id: Identifier,
    pub name: String,
    pub condition: Condition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InterlockReaction {
    /// While active, the sequence must not leave its initial step.
    BlockStart,
    /// While active in one of its steps, the sequence must not advance.
    HoldProgress,
    /// While active in one of its steps, the program must return to the initial step
    /// within the reaction time.
    AbortSequence,
}

/// A protective condition monitored continuously where it applies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SequenceInterlock {
    pub id: Identifier,
    pub name: String,
    /// TRUE while the protective condition is active.
    pub condition: Condition,
    pub reaction: InterlockReaction,
    /// Steps where it applies (`blockStart`: the initial step only).
    pub steps: Vec<Identifier>,
    /// Reaction time for `abortSequence`.
    #[serde(default)]
    pub reaction_ms: Option<u32>,
}

impl SequenceDefinition {
    pub fn initial(&self) -> Option<&SequenceStep> {
        self.steps.iter().find(|s| s.role == StepRole::Initial)
    }
    pub fn step(&self, id: &Identifier) -> Option<&SequenceStep> {
        self.steps.iter().find(|s| &s.id == id)
    }
}

/// Every evidence name a condition compares.
pub fn condition_signals<'a>(condition: &'a Condition, out: &mut Vec<&'a Identifier>) {
    match condition {
        Condition::All { of } | Condition::Any { of } => {
            of.iter().for_each(|c| condition_signals(c, out))
        }
        Condition::Not { of } => condition_signals(of, out),
        Condition::Compare { signal, .. } => out.push(signal),
    }
}

/// Structural problems of a sequence definition; `evidence` lists the exercise's evidence
/// names and `program_tags` those bound to PLC program tags.
pub fn sequence_problems(
    s: &SequenceDefinition,
    evidence: &BTreeSet<&Identifier>,
    program_tags: &BTreeSet<&Identifier>,
) -> Vec<String> {
    let mut problems = Vec::new();
    let mut check = |ok: bool, message: String| {
        if !ok {
            problems.push(format!("sequence '{}': {message}", s.id));
        }
    };
    check(
        program_tags.contains(&s.step_signal),
        format!(
            "step signal '{}' must be evidence bound to a program tag",
            s.step_signal
        ),
    );
    if let Some(held) = &s.held_signal {
        check(
            program_tags.contains(held),
            format!("held signal '{held}' must be evidence bound to a program tag"),
        );
    }
    check(!s.steps.is_empty(), "has no steps".into());
    check(
        s.steps.len() <= 64 && s.transitions.len() <= 128 && s.interlocks.len() <= 32,
        "is larger than 64 steps, 128 transitions or 32 interlocks".into(),
    );
    let mut ids = BTreeSet::new();
    let mut numbers = BTreeSet::new();
    for step in &s.steps {
        check(
            ids.insert(&step.id),
            format!("step '{}' is duplicated", step.id),
        );
        check(
            numbers.insert(step.number),
            format!("step number {} is used twice", step.number),
        );
        if let Some(t) = step.timeout_ms {
            check(
                step.role == StepRole::Normal && (100..=3_600_000).contains(&t),
                format!(
                    "step '{}' timeout must be 100 ms to 1 hour on a normal step",
                    step.id
                ),
            );
        }
    }
    let count = |role| s.steps.iter().filter(|x| x.role == role).count();
    check(
        count(StepRole::Initial) == 1,
        "needs exactly one initial step".into(),
    );
    check(
        count(StepRole::Final) <= 1,
        "has more than one final step".into(),
    );
    let mut transition_ids = BTreeSet::new();
    let mut pairs = BTreeSet::new();
    let mut referenced = Vec::new();
    for t in &s.transitions {
        check(
            transition_ids.insert(&t.id),
            format!("transition '{}' is duplicated", t.id),
        );
        check(
            ids.contains(&t.from) && ids.contains(&t.to),
            format!("transition '{}' references a missing step", t.id),
        );
        check(
            t.from != t.to,
            format!("transition '{}' does not change the step", t.id),
        );
        check(
            pairs.insert((&t.from, &t.to)),
            format!("transition '{}' repeats a pair of steps", t.id),
        );
        condition_signals(&t.condition, &mut referenced);
        let mut permissive_ids = BTreeSet::new();
        for p in &t.permissives {
            check(
                permissive_ids.insert(&p.id),
                format!("permissive '{}' is duplicated in '{}'", p.id, t.id),
            );
            condition_signals(&p.condition, &mut referenced);
        }
    }
    let mut interlock_ids = BTreeSet::new();
    for i in &s.interlocks {
        check(
            interlock_ids.insert(&i.id),
            format!("interlock '{}' is duplicated", i.id),
        );
        check(
            !i.steps.is_empty() && i.steps.iter().all(|x| ids.contains(x)),
            format!("interlock '{}' names no step or a missing step", i.id),
        );
        let initial = s.initial().map(|x| &x.id);
        match i.reaction {
            InterlockReaction::BlockStart => check(
                i.steps.iter().all(|x| Some(x) == initial) && i.reaction_ms.is_none(),
                format!(
                    "interlock '{}' blocks the start: it applies to the initial step only and has no reaction time",
                    i.id
                ),
            ),
            InterlockReaction::HoldProgress => check(
                i.steps.iter().all(|x| Some(x) != initial) && i.reaction_ms.is_none(),
                format!(
                    "interlock '{}' holds progress: it applies to normal or final steps and has no reaction time",
                    i.id
                ),
            ),
            InterlockReaction::AbortSequence => check(
                i.steps.iter().all(|x| Some(x) != initial)
                    && i.reaction_ms.is_some_and(|ms| ms <= 60_000),
                format!(
                    "interlock '{}' aborts: it applies to steps after the start and needs a reaction time of at most 60 s",
                    i.id
                ),
            ),
        }
        condition_signals(&i.condition, &mut referenced);
    }
    for signal in referenced {
        check(
            evidence.contains(signal),
            format!("condition references undefined evidence '{signal}'"),
        );
    }
    problems
}
