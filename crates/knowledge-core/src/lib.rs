//! Knowledge pack discovery, validation and search.
//!
//! Packs are folders containing `pack.json`. Content is canonical JSON with Markdown
//! section bodies; the search index is derived data rebuilt on load. A malformed pack
//! or file is reported with its path and reason and never aborts loading other content.
//! Discovery never follows symbolic links and enforces file count and size limits.

pub mod model;
pub mod search;

pub use model::*;
pub use search::{IndexedDocument, SearchHit, SearchIndex, SearchQuery, tokenize};

use lab_model::{
    Identifier, exercise::ExerciseDefinition, profile::PlatformProfile, scene::SceneDefinition,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MAX_FILES_PER_PACK: usize = 5_000;
pub const MAX_FILE_BYTES: u64 = 1024 * 1024;
const MAX_DEPTH: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackStatus {
    Loaded,
    /// Loaded with some files rejected.
    Partial,
    Invalid,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackCounts {
    pub articles: usize,
    pub lessons: usize,
    pub exercises: usize,
    pub scenes: usize,
    pub sources: usize,
    pub profiles: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackReport {
    pub path: String,
    pub manifest: Option<PackManifest>,
    pub status: PackStatus,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub counts: PackCounts,
}

#[derive(Debug, Clone, Default)]
pub struct KnowledgeLibrary {
    packs: Vec<PackReport>,
    articles: BTreeMap<Identifier, Article>,
    sources: BTreeMap<Identifier, SourceRecord>,
    lessons: BTreeMap<Identifier, Lesson>,
    exercises: BTreeMap<Identifier, ExerciseDefinition>,
    scenes: BTreeMap<Identifier, SceneDefinition>,
    profiles: BTreeMap<Identifier, PlatformProfile>,
    origin: BTreeMap<Identifier, usize>,
    index: SearchIndex,
}

enum Loaded {
    Article(Box<Article>),
    Sources(SourceFile),
    Lesson(Box<Lesson>),
    Exercise(Box<ExerciseDefinition>),
    Scene(Box<SceneDefinition>),
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn collect_json(
    root: &Path,
    dir: &Path,
    depth: usize,
    out: &mut Vec<PathBuf>,
    errors: &mut Vec<String>,
) {
    if depth > MAX_DEPTH {
        errors.push(format!(
            "{}: directory nesting exceeds {MAX_DEPTH} levels",
            relative(root, dir)
        ));
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        errors.push(format!("{}: directory cannot be read", relative(root, dir)));
        return;
    };
    let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.file_type().is_symlink() {
            errors.push(format!(
                "{}: symbolic links are not followed",
                relative(root, &path)
            ));
        } else if meta.is_dir() {
            collect_json(root, &path, depth + 1, out, errors);
        } else if path.extension().is_some_and(|e| e == "json")
            && path.file_name().is_some_and(|n| n != "pack.json")
        {
            if out.len() >= MAX_FILES_PER_PACK {
                errors.push(format!(
                    "pack exceeds {MAX_FILES_PER_PACK} content files; remaining files skipped"
                ));
                return;
            }
            if meta.len() > MAX_FILE_BYTES {
                errors.push(format!(
                    "{}: file exceeds {MAX_FILE_BYTES} bytes",
                    relative(root, &path)
                ));
                continue;
            }
            out.push(path);
        }
    }
}

fn parse_file(root: &Path, path: &Path) -> Result<Option<Loaded>, String> {
    let rel = relative(root, path);
    let top = rel.split('/').next().unwrap_or_default();
    if top == "schemas" {
        // Schemas document the format for authors and tools; they are validated by CI.
        return Ok(None);
    }
    let text = fs::read_to_string(path).map_err(|e| format!("{rel}: {e}"))?;
    let parsed = match top {
        "scenes" => serde_json::from_str(&text).map(|v| Loaded::Scene(Box::new(v))),
        "exercises" => serde_json::from_str(&text).map(|v| Loaded::Exercise(Box::new(v))),
        "lessons" => serde_json::from_str(&text).map(|v| Loaded::Lesson(Box::new(v))),
        "sources" => serde_json::from_str(&text).map(Loaded::Sources),
        _ => serde_json::from_str(&text).map(|v| Loaded::Article(Box::new(v))),
    };
    parsed.map(Some).map_err(|e| format!("{rel}: {e}"))
}

impl KnowledgeLibrary {
    /// Discover packs: each root is a pack if it holds `pack.json`, otherwise every
    /// direct subdirectory holding `pack.json` is a pack. Roots that do not exist are skipped.
    pub fn load(roots: &[PathBuf]) -> Self {
        let mut library = Self::default();
        let mut seen = BTreeSet::new();
        for root in roots {
            let mut candidates = Vec::new();
            if root.join("pack.json").is_file() {
                candidates.push(root.clone());
            } else if let Ok(entries) = fs::read_dir(root) {
                let mut dirs: Vec<PathBuf> = entries
                    .filter_map(Result::ok)
                    .filter(|e| fs::symlink_metadata(e.path()).is_ok_and(|m| m.is_dir()))
                    .map(|e| e.path())
                    .filter(|p| p.join("pack.json").is_file())
                    .collect();
                dirs.sort();
                candidates.extend(dirs);
            }
            for dir in candidates {
                let canonical = fs::canonicalize(&dir).unwrap_or(dir.clone());
                if seen.insert(canonical) {
                    library.load_pack(&dir);
                }
            }
        }
        library.cross_validate();
        library.rebuild_index();
        library
    }

    fn load_pack(&mut self, dir: &Path) {
        let pack_index = self.packs.len();
        let mut report = PackReport {
            path: dir.to_string_lossy().into_owned(),
            manifest: None,
            status: PackStatus::Loaded,
            errors: Vec::new(),
            warnings: Vec::new(),
            counts: PackCounts::default(),
        };
        let manifest = fs::read_to_string(dir.join("pack.json"))
            .map_err(|e| e.to_string())
            .and_then(|t| serde_json::from_str::<PackManifest>(&t).map_err(|e| e.to_string()));
        match manifest {
            Err(e) => {
                report.errors.push(format!("pack.json: {e}"));
                report.status = PackStatus::Invalid;
                self.packs.push(report);
                return;
            }
            Ok(m) if m.pack_format != PACK_FORMAT => {
                report.errors.push(format!(
                    "pack.json: pack format {} is not supported by this application (supported: {PACK_FORMAT})",
                    m.pack_format
                ));
                report.manifest = Some(m);
                report.status = PackStatus::Invalid;
                self.packs.push(report);
                return;
            }
            Ok(m) => {
                if self.packs.iter().any(|p| {
                    p.manifest
                        .as_ref()
                        .is_some_and(|o| o.id == m.id && p.status != PackStatus::Invalid)
                }) {
                    report.errors.push(format!(
                        "pack '{}' is already installed from another folder",
                        m.id
                    ));
                    report.manifest = Some(m);
                    report.status = PackStatus::Invalid;
                    self.packs.push(report);
                    return;
                }
                report.manifest = Some(m);
            }
        }
        if !dir.join("schemas").is_dir() {
            report.warnings.push(
                "schemas/ folder is missing; authoring tools cannot validate this pack".into(),
            );
        }
        let mut files = Vec::new();
        collect_json(dir, dir, 0, &mut files, &mut report.errors);
        for path in files {
            match parse_file(dir, &path) {
                Ok(None) => {}
                Ok(Some(item)) => self.insert(item, pack_index, &relative(dir, &path), &mut report),
                Err(e) => report.errors.push(e),
            }
        }
        if !report.errors.is_empty() {
            report.status = PackStatus::Partial;
        }
        self.packs.push(report);
    }

    fn claim(&mut self, id: &Identifier, pack: usize, rel: &str, report: &mut PackReport) -> bool {
        if self.origin.contains_key(id) {
            report
                .errors
                .push(format!("{rel}: identifier '{id}' is already defined"));
            return false;
        }
        self.origin.insert(id.clone(), pack);
        true
    }

    fn insert(&mut self, item: Loaded, pack: usize, rel: &str, report: &mut PackReport) {
        match item {
            Loaded::Article(a) => {
                if let Some(profile) = &a.simulation {
                    if self.profiles.contains_key(&profile.id)
                        || profile.id.as_str() == "generic.iec"
                    {
                        report.errors.push(format!(
                            "{rel}: simulation profile '{}' is duplicated",
                            profile.id
                        ));
                    } else {
                        self.profiles.insert(profile.id.clone(), profile.clone());
                        report.counts.profiles += 1;
                    }
                }
                if self.claim(&a.id, pack, rel, report) {
                    self.articles.insert(a.id.clone(), *a);
                    report.counts.articles += 1;
                }
            }
            Loaded::Sources(file) => {
                for s in file.sources {
                    if self.claim(&s.id, pack, rel, report) {
                        if !s.url.starts_with("https://") {
                            report.warnings.push(format!(
                                "{rel}: source '{}' does not use an https URL",
                                s.id
                            ));
                        }
                        self.sources.insert(s.id.clone(), s);
                        report.counts.sources += 1;
                    }
                }
            }
            Loaded::Lesson(l) => {
                let bad = l
                    .checkpoint
                    .iter()
                    .chain(l.sections.iter().filter_map(|s| s.question.as_ref()))
                    .any(|q| usize::from(q.answer) >= q.options.len());
                if bad {
                    report
                        .errors
                        .push(format!("{rel}: a question answer index is out of range"));
                } else if self.claim(&l.id, pack, rel, report) {
                    self.lessons.insert(l.id.clone(), *l);
                    report.counts.lessons += 1;
                }
            }
            Loaded::Exercise(e) => {
                if self.claim(&e.id, pack, rel, report) {
                    self.exercises.insert(e.id.clone(), *e);
                    report.counts.exercises += 1;
                }
            }
            Loaded::Scene(s) => match lab_model::validate_scene(&s) {
                Err(err) => report.errors.push(format!("{rel}: {err}")),
                Ok(()) => {
                    if self.claim(&s.id, pack, rel, report) {
                        self.scenes.insert(s.id.clone(), *s);
                        report.counts.scenes += 1;
                    }
                }
            },
        }
    }

    /// Cross-reference checks run after all packs are loaded.
    fn cross_validate(&mut self) {
        let mut rejected_exercises = Vec::new();
        for (id, exercise) in &self.exercises {
            let pack = self.origin[id];
            let problem = match self.scenes.get(&exercise.scene) {
                None => Some(format!(
                    "exercise '{id}' references missing scene '{}'",
                    exercise.scene
                )),
                Some(scene) => lab_model::validate_exercise(exercise, scene)
                    .err()
                    .map(|e| format!("exercise '{id}': {e}")),
            }
            .or_else(|| {
                exercise
                    .platform
                    .as_ref()
                    .filter(|p| p.as_str() != "generic.iec" && !self.profiles.contains_key(*p))
                    .map(|p| format!("exercise '{id}' references missing platform profile '{p}'"))
            });
            if let Some(problem) = problem {
                self.packs[pack].errors.push(problem);
                rejected_exercises.push(id.clone());
            }
        }
        for id in rejected_exercises {
            self.exercises.remove(&id);
            if let Some(pack) = self.origin.get(&id).copied() {
                self.packs[pack].counts.exercises -= 1;
            }
        }
        let known: BTreeSet<&Identifier> = self.origin.keys().collect();
        let mut rejected_articles = Vec::new();
        for (id, article) in &self.articles {
            let pack = self.origin[id];
            let missing: Vec<&Identifier> = article
                .sources
                .iter()
                .filter(|s| !self.sources.contains_key(*s))
                .collect();
            let fact_missing = article
                .facts
                .iter()
                .flat_map(|f| f.sources.iter())
                .any(|s| !self.sources.contains_key(s));
            if !missing.is_empty() || fact_missing {
                self.packs[pack].errors.push(format!(
                    "article '{id}' cites unknown sources: {}",
                    missing
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                rejected_articles.push(id.clone());
                continue;
            }
            if article.sources.is_empty() && article.kind != ArticleKind::Topic {
                self.packs[pack]
                    .warnings
                    .push(format!("article '{id}' has no provenance sources"));
            }
            for r in &article.related {
                if !known.contains(r) {
                    self.packs[pack]
                        .warnings
                        .push(format!("article '{id}' relates to unknown item '{r}'"));
                }
            }
        }
        for id in rejected_articles {
            self.articles.remove(&id);
            if let Some(pack) = self.origin.get(&id).copied() {
                self.packs[pack].counts.articles -= 1;
            }
        }
        for (id, lesson) in &self.lessons {
            let pack = self.origin[id];
            for s in &lesson.sections {
                if let Some(scene) = &s.scene
                    && !self.scenes.contains_key(scene)
                {
                    self.packs[pack]
                        .warnings
                        .push(format!("lesson '{id}' references missing scene '{scene}'"));
                }
                if let Some(ex) = &s.exercise
                    && !self.exercises.contains_key(ex)
                {
                    self.packs[pack]
                        .warnings
                        .push(format!("lesson '{id}' references missing exercise '{ex}'"));
                }
            }
        }
        for report in &mut self.packs {
            if report.status == PackStatus::Loaded && !report.errors.is_empty() {
                report.status = PackStatus::Partial;
            }
        }
    }

    fn rebuild_index(&mut self) {
        let mut docs = Vec::new();
        for a in self.articles.values() {
            let mut fields: Vec<(u8, String)> = vec![(4, a.tags.join(" "))];
            fields.extend(
                a.sections
                    .iter()
                    .map(|s| (1u8, format!("{}\n{}", s.heading, s.body))),
            );
            fields.extend(
                a.facts
                    .iter()
                    .map(|f| (2u8, format!("{} {}", f.label, f.value))),
            );
            if let Some(t) = &a.troubleshooting {
                fields.push((2, t.symptoms.join("\n")));
                fields.push((
                    1,
                    format!(
                        "{}\n{}\n{}",
                        t.evidence.join("\n"),
                        t.diagnostic_method.join("\n"),
                        t.repair.join("\n")
                    ),
                ));
            }
            if let Some(i) = &a.instruction {
                fields.push((
                    2,
                    i.equivalents
                        .iter()
                        .map(|e| format!("{} {}", e.platform, e.name))
                        .collect::<Vec<_>>()
                        .join(" "),
                ));
            }
            docs.push(IndexedDocument {
                id: a.id.to_string(),
                doc_type: serde_json::to_value(a.kind)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_default(),
                category: a.category(),
                title: a.title.clone(),
                summary: a.summary.clone(),
                vendor: a.vendor.as_ref().map(ToString::to_string),
                family: a.family.as_ref().map(ToString::to_string),
                protocol: a.protocol.as_ref().map(ToString::to_string),
                fields,
            });
        }
        for l in self.lessons.values() {
            docs.push(IndexedDocument {
                id: l.id.to_string(),
                doc_type: "lesson".into(),
                category: Category::PlcLabSimulation,
                title: l.title.clone(),
                summary: l.summary.clone(),
                vendor: None,
                family: None,
                protocol: None,
                fields: l
                    .sections
                    .iter()
                    .map(|s| (1u8, format!("{}\n{}", s.heading, s.body)))
                    .collect(),
            });
        }
        for e in self.exercises.values() {
            docs.push(IndexedDocument {
                id: e.id.to_string(),
                doc_type: "exercise".into(),
                category: Category::PlcLabSimulation,
                title: e.title.clone(),
                summary: e.summary.clone(),
                vendor: None,
                family: None,
                protocol: None,
                fields: vec![(2, e.objectives.join("\n")), (1, e.instructions.join("\n"))],
            });
        }
        self.index = SearchIndex::build(docs);
    }

    pub fn packs(&self) -> &[PackReport] {
        &self.packs
    }
    pub fn search(&self, query: &SearchQuery) -> Vec<SearchHit> {
        self.index.search(query)
    }
    pub fn index_size(&self) -> usize {
        self.index.len()
    }
    pub fn article(&self, id: &str) -> Option<&Article> {
        self.articles.values().find(|a| a.id.as_str() == id)
    }
    pub fn articles(&self) -> impl Iterator<Item = &Article> {
        self.articles.values()
    }
    pub fn source(&self, id: &Identifier) -> Option<&SourceRecord> {
        self.sources.get(id)
    }
    pub fn lessons(&self) -> impl Iterator<Item = &Lesson> {
        self.lessons.values()
    }
    pub fn lesson(&self, id: &str) -> Option<&Lesson> {
        self.lessons.values().find(|l| l.id.as_str() == id)
    }
    pub fn exercises(&self) -> impl Iterator<Item = &ExerciseDefinition> {
        self.exercises.values()
    }
    pub fn exercise(&self, id: &str) -> Option<&ExerciseDefinition> {
        self.exercises.values().find(|e| e.id.as_str() == id)
    }
    pub fn scene(&self, id: &str) -> Option<&SceneDefinition> {
        self.scenes.values().find(|s| s.id.as_str() == id)
    }
    pub fn scenes(&self) -> impl Iterator<Item = &SceneDefinition> {
        self.scenes.values()
    }
    /// Pack profiles plus the built-in generic profile.
    pub fn profile(&self, id: &str) -> Option<PlatformProfile> {
        if id == "generic.iec" {
            return Some(PlatformProfile::generic());
        }
        self.profiles
            .values()
            .find(|p| p.id.as_str() == id)
            .cloned()
    }
    pub fn profiles(&self) -> Vec<PlatformProfile> {
        let mut all = vec![PlatformProfile::generic()];
        all.extend(self.profiles.values().cloned());
        all
    }
    pub fn title_of(&self, id: &Identifier) -> Option<String> {
        self.articles
            .get(id)
            .map(|a| a.title.clone())
            .or_else(|| self.lessons.get(id).map(|l| l.title.clone()))
            .or_else(|| self.exercises.get(id).map(|e| e.title.clone()))
            .or_else(|| self.scenes.get(id).map(|s| s.title.clone()))
    }
}
