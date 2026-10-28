//! Knowledge, curriculum and exercise catalog projections. Read-only use cases over the
//! loaded knowledge library and the learner's progress.

use crate::{
    convert,
    error::{AppError, AppResult},
};
use app_contract::*;
use knowledge_core::{
    Article, ArticleKind, Category, KnowledgeLibrary, PackStatus, SearchQuery, SupportLevel,
    Verification,
};
use lab_model::{
    Identifier,
    exercise::{Difficulty, ExerciseDefinition, ExerciseKind, TroubleshootingSetup},
    scene::{SignalDirection, TagType},
};
use project_model::Progress;

pub fn category_name(c: Category) -> &'static str {
    match c {
        Category::Fundamental => "FUNDAMENTAL",
        Category::VendorSpecific => "VENDOR_SPECIFIC",
        Category::PlcLabSimulation => "PLC_LAB_SIMULATION",
        Category::Troubleshooting => "TROUBLESHOOTING",
    }
}

fn parse_category(text: &str) -> AppResult<Category> {
    Ok(match text {
        "FUNDAMENTAL" => Category::Fundamental,
        "VENDOR_SPECIFIC" => Category::VendorSpecific,
        "PLC_LAB_SIMULATION" => Category::PlcLabSimulation,
        "TROUBLESHOOTING" => Category::Troubleshooting,
        other => {
            return Err(AppError::InvalidRequest(format!(
                "Unknown knowledge category '{other}'."
            )));
        }
    })
}

pub fn status(library: &KnowledgeLibrary, search_paths: &[String]) -> KnowledgeStatusDto {
    KnowledgeStatusDto {
        packs: library
            .packs()
            .iter()
            .map(|p| PackDto {
                id: p.manifest.as_ref().map(|m| m.id.to_string()),
                title: p.manifest.as_ref().map(|m| m.title.clone()),
                version: p.manifest.as_ref().map(|m| m.version.clone()),
                path: p.path.clone(),
                status: match p.status {
                    PackStatus::Loaded => "LOADED",
                    PackStatus::Partial => "PARTIAL",
                    PackStatus::Invalid => "INVALID",
                }
                .into(),
                errors: p.errors.clone(),
                warnings: p.warnings.clone(),
                articles: count(p.counts.articles),
                lessons: count(p.counts.lessons),
                exercises: count(p.counts.exercises),
                scenes: count(p.counts.scenes),
                sources: count(p.counts.sources),
                profiles: count(p.counts.profiles),
            })
            .collect(),
        indexed_items: count(library.index_size()),
        search_paths: search_paths.to_vec(),
    }
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

pub fn search(
    library: &KnowledgeLibrary,
    request: &KnowledgeSearchRequest,
) -> AppResult<Vec<SearchHitDto>> {
    if request.query.len() > 200 {
        return Err(AppError::InvalidRequest(
            "The search text is too long (at most 200 characters).".into(),
        ));
    }
    let query = SearchQuery {
        text: request.query.clone(),
        category: request
            .category
            .as_deref()
            .map(parse_category)
            .transpose()?,
        vendor: request.vendor.clone(),
        doc_type: request.doc_type.clone(),
        limit: usize::from(request.limit.clamp(1, 100)),
    };
    Ok(library
        .search(&query)
        .into_iter()
        .map(|h| SearchHitDto {
            id: h.id,
            doc_type: h.doc_type,
            category: category_name(h.category).into(),
            title: h.title,
            summary: h.summary,
            vendor: h.vendor,
            snippet: h.snippet,
        })
        .collect())
}

fn related(library: &KnowledgeLibrary, ids: &[Identifier]) -> Vec<RelatedDto> {
    ids.iter()
        .filter_map(|id| {
            library.title_of(id).map(|title| RelatedDto {
                id: id.to_string(),
                title,
            })
        })
        .collect()
}

fn support(level: SupportLevel) -> &'static str {
    match level {
        SupportLevel::ExactBehavioral => "EXACT_BEHAVIORAL",
        SupportLevel::ApproximatedBehavioral => "APPROXIMATED_BEHAVIORAL",
        SupportLevel::Conceptual => "CONCEPTUAL",
        SupportLevel::NotImplemented => "NOT_IMPLEMENTED",
    }
}

fn kind_name(kind: ArticleKind) -> &'static str {
    match kind {
        ArticleKind::Vendor => "vendor",
        ArticleKind::Family => "family",
        ArticleKind::Hardware => "hardware",
        ArticleKind::Instruction => "instruction",
        ArticleKind::Protocol => "protocol",
        ArticleKind::Topic => "topic",
        ArticleKind::Troubleshooting => "troubleshooting",
        ArticleKind::Diagnostic => "diagnostic",
    }
}

pub fn article(library: &KnowledgeLibrary, id: &str) -> AppResult<ArticleDto> {
    let a = library.article(id).ok_or_else(|| AppError::NotFound {
        what: "article",
        id: id.to_owned(),
    })?;
    Ok(ArticleDto {
        id: a.id.to_string(),
        kind: kind_name(a.kind).into(),
        category: category_name(a.category()).into(),
        title: a.title.clone(),
        summary: a.summary.clone(),
        vendor: a.vendor.as_ref().map(ToString::to_string),
        family: a.family.as_ref().map(ToString::to_string),
        protocol: a.protocol.as_ref().map(ToString::to_string),
        tier: a.tier.clone(),
        tags: a.tags.clone(),
        sections: a
            .sections
            .iter()
            .map(|s| SectionDto {
                heading: s.heading.clone(),
                body: s.body.clone(),
            })
            .collect(),
        facts: a
            .facts
            .iter()
            .map(|f| FactDto {
                label: f.label.clone(),
                value: f.value.clone(),
                sources: f.sources.iter().map(ToString::to_string).collect(),
            })
            .collect(),
        fidelity: a
            .fidelity
            .as_ref()
            .or(a.simulation.as_ref().map(|p| &p.fidelity))
            .map(convert::fidelity),
        instruction: a.instruction.as_ref().map(|i| InstructionDto {
            category: i.category.clone(),
            inputs: i
                .inputs
                .iter()
                .map(|p| ParameterDto {
                    name: p.name.clone(),
                    data_type: p.data_type.clone(),
                    description: p.description.clone(),
                })
                .collect(),
            outputs: i
                .outputs
                .iter()
                .map(|p| ParameterDto {
                    name: p.name.clone(),
                    data_type: p.data_type.clone(),
                    description: p.description.clone(),
                })
                .collect(),
            state: i.state.clone(),
            retentivity: i.retentivity.clone(),
            timing: i.timing.clone(),
            simulation_support: support(i.simulation_support).into(),
            equivalents: i
                .equivalents
                .iter()
                .map(|e| EquivalentDto {
                    platform: e.platform.clone(),
                    name: e.name.clone(),
                    notes: e.notes.clone(),
                })
                .collect(),
        }),
        troubleshooting: a.troubleshooting.as_ref().map(|t| TroubleshootingDto {
            fault_id: t.fault_id.to_string(),
            area: t.area.clone(),
            target: t.target.clone(),
            effect: t.effect.clone(),
            symptoms: t.symptoms.clone(),
            evidence: t.evidence.clone(),
            diagnostic_method: t.diagnostic_method.clone(),
            repair: t.repair.clone(),
            explanation: t.explanation.clone(),
            simulated_in: related(library, &t.simulated_in),
        }),
        simulation_profile: a.simulation.as_ref().map(|p| p.id.to_string()),
        sources: a
            .sources
            .iter()
            .filter_map(|id| library.source(id))
            .map(|s| SourceDto {
                id: s.id.to_string(),
                vendor: s.vendor.as_ref().map(ToString::to_string),
                title: s.title.clone(),
                document_type: s.document_type.clone(),
                document_number: s.document_number.clone(),
                url: s.url.clone(),
                retrieved: s.retrieved.clone(),
                applies_to: s.applies_to.clone(),
                notes: s.notes.clone(),
                confidence: format!("{:?}", s.confidence).to_uppercase(),
                verification: match s.verification {
                    Verification::Verified => "VERIFIED",
                    Verification::Unverified => "UNVERIFIED",
                }
                .into(),
            })
            .collect(),
        related: related(library, &a.related),
    })
}

fn entry(a: &Article, children: Vec<CatalogEntryDto>) -> CatalogEntryDto {
    CatalogEntryDto {
        id: a.id.to_string(),
        title: a.title.clone(),
        summary: a.summary.clone(),
        tier: a.tier.clone(),
        profile: a.simulation.as_ref().map(|p| p.id.to_string()),
        area: a.troubleshooting.as_ref().map(|t| t.area.clone()),
        children,
    }
}

pub fn catalog(library: &KnowledgeLibrary) -> CatalogDto {
    let of_kind = |kind: ArticleKind| library.articles().filter(move |a| a.kind == kind);
    let vendors = of_kind(ArticleKind::Vendor)
        .map(|v| {
            let families = of_kind(ArticleKind::Family)
                .filter(|f| f.vendor.as_ref() == Some(&v.id))
                .map(|f| {
                    let hardware = library
                        .articles()
                        .filter(|h| {
                            matches!(h.kind, ArticleKind::Hardware | ArticleKind::Diagnostic)
                                && h.family.as_ref() == Some(&f.id)
                        })
                        .map(|h| entry(h, Vec::new()))
                        .collect();
                    entry(f, hardware)
                })
                .collect();
            entry(v, families)
        })
        .collect::<Vec<_>>();
    let mut vendors = vendors;
    vendors.sort_by(|a, b| a.tier.cmp(&b.tier).then_with(|| a.title.cmp(&b.title)));
    let simple = |kind: ArticleKind| {
        let mut list: Vec<CatalogEntryDto> = of_kind(kind).map(|a| entry(a, Vec::new())).collect();
        list.sort_by(|a, b| a.area.cmp(&b.area).then_with(|| a.title.cmp(&b.title)));
        list
    };
    CatalogDto {
        vendors,
        protocols: simple(ArticleKind::Protocol),
        fundamentals: simple(ArticleKind::Topic),
        instructions: simple(ArticleKind::Instruction),
        troubleshooting: simple(ArticleKind::Troubleshooting),
    }
}

fn difficulty(d: Difficulty) -> &'static str {
    match d {
        Difficulty::Beginner => "beginner",
        Difficulty::Intermediate => "intermediate",
        Difficulty::Advanced => "advanced",
    }
}

fn exercise_kind(k: ExerciseKind) -> &'static str {
    match k {
        ExerciseKind::Practice => "practice",
        ExerciseKind::Troubleshooting => "troubleshooting",
        ExerciseKind::Commissioning => "commissioning",
    }
}

pub fn platform_of(e: &ExerciseDefinition) -> String {
    e.platform
        .as_ref()
        .map_or_else(|| "generic.iec".to_owned(), ToString::to_string)
}

fn max_score(e: &ExerciseDefinition) -> u32 {
    if e.kind.is_fault_finding() {
        100
    } else {
        e.criteria.iter().map(|c| c.points).sum()
    }
}

pub fn library(library: &KnowledgeLibrary, progress: &Progress, notices: &[String]) -> LibraryDto {
    let mut lessons: Vec<LessonSummaryDto> = library
        .lessons()
        .map(|l| {
            let p = progress.lessons.get(l.id.as_str());
            LessonSummaryDto {
                id: l.id.to_string(),
                number: l.number,
                title: l.title.clone(),
                summary: l.summary.clone(),
                level: l.level.clone(),
                minutes: l.minutes,
                completed: p.is_some_and(|p| p.completed),
                checkpoint_correct: p.map_or(0, |p| p.checkpoint_correct),
                checkpoint_total: p.map_or(0, |p| p.checkpoint_total),
            }
        })
        .collect();
    lessons.sort_by_key(|l| l.number);
    // Curriculum order, not difficulty or duration, decides the sequence a learner sees.
    let mut exercises: Vec<(u16, ExerciseSummaryDto)> = library
        .exercises()
        .map(|e| {
            let p = progress.exercises.get(e.id.as_str());
            (
                e.curriculum_order.unwrap_or(u16::MAX),
                ExerciseSummaryDto {
                    id: e.id.to_string(),
                    title: e.title.clone(),
                    kind: exercise_kind(e.kind).into(),
                    difficulty: difficulty(e.difficulty).into(),
                    summary: e.summary.clone(),
                    minutes: e.estimated_minutes,
                    scene: e.scene.to_string(),
                    platform: platform_of(e),
                    attempts: p.map_or(0, |p| p.attempts),
                    best_score: p.map_or(0, |p| p.best_score),
                    max_score: max_score(e),
                    passed: p.is_some_and(|p| p.passed),
                },
            )
        })
        .collect();
    exercises.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.id.cmp(&b.1.id)));
    let mut scenes: Vec<SceneSummaryDto> = library
        .scenes()
        .map(|s| SceneSummaryDto {
            id: s.id.to_string(),
            title: s.title.clone(),
            summary: s.summary.clone(),
        })
        .collect();
    scenes.sort_by(|a, b| a.title.cmp(&b.title));
    LibraryDto {
        lessons,
        exercises: exercises.into_iter().map(|(_, e)| e).collect(),
        platforms: library.profiles().iter().map(convert::platform).collect(),
        scenes,
        notices: notices.to_vec(),
    }
}

fn question(q: &knowledge_core::Question) -> QuestionDto {
    QuestionDto {
        question: q.question.clone(),
        options: q.options.clone(),
        answer: q.answer,
        explanation: q.explanation.clone(),
    }
}

pub fn lesson(library: &KnowledgeLibrary, id: &str) -> AppResult<LessonDto> {
    let l = library.lesson(id).ok_or_else(|| AppError::NotFound {
        what: "lesson",
        id: id.to_owned(),
    })?;
    Ok(LessonDto {
        id: l.id.to_string(),
        number: l.number,
        title: l.title.clone(),
        summary: l.summary.clone(),
        level: l.level.clone(),
        minutes: l.minutes,
        objectives: l.objectives.clone(),
        sections: l
            .sections
            .iter()
            .map(|s| LessonSectionDto {
                section_type: format!("{:?}", s.section_type).to_lowercase(),
                heading: s.heading.clone(),
                body: s.body.clone(),
                code: s.code.clone(),
                scene: s.scene.as_ref().map(ToString::to_string),
                exercise: s.exercise.as_ref().map(ToString::to_string),
                question: s.question.as_ref().map(question),
                diagram: s.diagram.clone(),
            })
            .collect(),
        checkpoint: l.checkpoint.iter().map(question).collect(),
        knowledge_refs: related(library, &l.knowledge_refs),
    })
}

fn tag_type(t: TagType) -> &'static str {
    match t {
        TagType::Bool => "BOOL",
        TagType::Int => "INT",
        TagType::Real => "REAL",
        TagType::Time => "TIME",
    }
}
pub fn direction(d: SignalDirection) -> &'static str {
    match d {
        SignalDirection::Input => "input",
        SignalDirection::Output => "output",
    }
}

/// The exercise as shown outside a workspace: troubleshooting repairs stay locked.
pub fn exercise(library: &KnowledgeLibrary, e: &ExerciseDefinition) -> ExerciseDto {
    exercise_view(library, e, false)
}

/// The exercise as shown in a workspace. Troubleshooting labs expose only their general
/// learning references; the repair options appear once `diagnosed` (a correct diagnosis
/// is on record), and cause-specific references only inside the debrief.
pub fn exercise_view(
    library: &KnowledgeLibrary,
    e: &ExerciseDefinition,
    diagnosed: bool,
) -> ExerciseDto {
    let refs = match &e.troubleshooting {
        Some(t) => &t.learning_refs,
        None => &e.knowledge_refs,
    };
    ExerciseDto {
        id: e.id.to_string(),
        revision: e.revision,
        title: e.title.clone(),
        kind: exercise_kind(e.kind).into(),
        difficulty: difficulty(e.difficulty).into(),
        summary: e.summary.clone(),
        objectives: e.objectives.clone(),
        minutes: e.estimated_minutes,
        scene: e.scene.to_string(),
        platform: platform_of(e),
        required_interface: e
            .required_interface
            .iter()
            .map(|t| RequiredTagDto {
                tag: t.tag.to_string(),
                data_type: tag_type(t.tag_type).into(),
                direction: direction(t.direction).into(),
            })
            .collect(),
        instructions: e.instructions.clone(),
        hints: e.hints.clone(),
        knowledge_refs: related(library, refs),
        criteria: if e.kind.is_fault_finding() {
            vec![
                CriterionDto {
                    id: "diagnosis".into(),
                    title: "Root cause identified from evidence".into(),
                    points: 30,
                },
                CriterionDto {
                    id: "repair".into(),
                    title: "Fault repaired".into(),
                    points: 20,
                },
                CriterionDto {
                    id: "verification".into(),
                    title: "System verified after repair".into(),
                    points: 50,
                },
            ]
        } else {
            e.criteria
                .iter()
                .map(|c| CriterionDto {
                    id: c.id.to_string(),
                    title: c.title.clone(),
                    points: c.points,
                })
                .collect()
        },
        troubleshooting: e.troubleshooting.as_ref().map(|t| TroubleshootingBriefDto {
            symptom: t.symptom.clone(),
            reported_by: t.reported_by.clone(),
            diagnosis_options: t
                .diagnosis_options
                .iter()
                .map(|o| OptionDto {
                    id: o.id.to_string(),
                    label: o.label.clone(),
                })
                .collect(),
            diagnosed,
            repairs: if diagnosed {
                repair_options(t)
            } else {
                Vec::new()
            },
        }),
    }
}

pub fn repair_options(t: &TroubleshootingSetup) -> Vec<OptionDto> {
    t.repairs
        .iter()
        .map(|o| OptionDto {
            id: o.id.to_string(),
            label: o.label.clone(),
        })
        .collect()
}

/// Cause-specific references of a troubleshooting lab, for its debrief.
pub fn debrief_references(library: &KnowledgeLibrary, e: &ExerciseDefinition) -> Vec<RelatedDto> {
    e.troubleshooting
        .as_ref()
        .map(|t| related(library, &t.debrief_refs))
        .unwrap_or_default()
}

pub fn exercise_by_id(library: &KnowledgeLibrary, id: &str) -> AppResult<ExerciseDto> {
    let e = library.exercise(id).ok_or_else(|| AppError::NotFound {
        what: "exercise",
        id: id.to_owned(),
    })?;
    Ok(exercise(library, e))
}
