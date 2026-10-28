//! The knowledge pack as a user installs it: the application discovers the pack below a
//! knowledge root, indexes it, answers typical searches and opens vendor and
//! troubleshooting pages with resolved references.
//!
//! `PLC_LAB_KNOWLEDGE_ROOT` selects the root. `scripts/knowledge-install-check.mjs` sets it
//! to a temporary folder (with a space in its path) into which the release ZIP was
//! extracted; without it the repository's source pack is used.
use app_contract::*;
use desktop_session::{AppInfo, AppPaths, DesktopApp};
use std::path::PathBuf;

const V: u32 = CONTRACT_VERSION;

fn knowledge_root() -> PathBuf {
    match std::env::var_os("PLC_LAB_KNOWLEDGE_ROOT") {
        Some(root) if !root.is_empty() => PathBuf::from(root),
        _ => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../knowledge"),
    }
}

fn app(data: &std::path::Path) -> DesktopApp {
    DesktopApp::new(
        AppPaths {
            knowledge_roots: vec![knowledge_root()],
            projects_dir: data.join("projects"),
            progress_file: data.join("progress.json"),
        },
        AppInfo {
            app_version: "test".into(),
            build_context: "test".into(),
        },
    )
}

fn search(app: &DesktopApp, query: &str) -> Vec<SearchHitDto> {
    app.knowledge_search(KnowledgeSearchRequest {
        contract_version: V,
        query: query.into(),
        category: None,
        vendor: None,
        doc_type: None,
        limit: 5,
    })
    .unwrap()
}

fn article(app: &DesktopApp, id: &str) -> ArticleDto {
    app.knowledge_article(IdRequest {
        contract_version: V,
        id: id.into(),
    })
    .unwrap_or_else(|e| panic!("{id}: {e:?}"))
}

#[test]
fn installed_pack_is_discovered_indexed_searchable_and_resolvable() {
    let data = std::env::temp_dir().join(format!("plclab-install-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data);
    let app = app(&data);
    let root = knowledge_root();
    println!("knowledge root: {}", root.display());

    let status = app
        .knowledge_status(VersionedRequest {
            contract_version: V,
        })
        .unwrap();
    assert_eq!(status.packs.len(), 1, "{:#?}", status.packs);
    let pack = &status.packs[0];
    assert_eq!(pack.status, "LOADED", "{:#?}", pack.errors);
    assert!(pack.warnings.is_empty(), "{:#?}", pack.warnings);
    assert_eq!(pack.id.as_deref(), Some("plclab.core"));
    let canonical_root = root.canonicalize().unwrap();
    assert!(
        PathBuf::from(&pack.path)
            .canonicalize()
            .unwrap()
            .starts_with(&canonical_root),
        "pack {} is not below {}",
        pack.path,
        canonical_root.display()
    );
    println!(
        "pack {} {}: {} articles, {} lessons, {} exercises, {} scenes, {} sources, {} profiles; {} indexed items",
        pack.id.as_deref().unwrap_or("?"),
        pack.version.as_deref().unwrap_or("?"),
        pack.articles,
        pack.lessons,
        pack.exercises,
        pack.scenes,
        pack.sources,
        pack.profiles,
        status.indexed_items
    );
    assert!(status.indexed_items > 100, "{}", status.indexed_items);

    // (query, acceptable first hits, page that must be among the first three)
    for (query, first, top3) in [
        (
            "4-20 mA",
            &["lesson.4-20ma", "fundamentals.4-20ma"][..],
            "fundamentals.4-20ma",
        ),
        (
            "PROFINET device name",
            &["protocol.profinet"],
            "protocol.profinet",
        ),
        (
            "Modbus register offset",
            &["troubleshooting.modbus-wrong-register"],
            "protocol.modbus-tcp",
        ),
        (
            "VFD remote mode",
            &["troubleshooting.vfd-local-remote"],
            "fundamentals.drives-basics",
        ),
        (
            "ET200SP",
            &["family.siemens-et200sp"],
            "family.siemens-et200sp",
        ),
        (
            "CompactLogix",
            &["family.rockwell-compactlogix"],
            "family.rockwell-compactlogix",
        ),
    ] {
        let hits = search(&app, query);
        let ids: Vec<&str> = hits.iter().map(|h| h.id.as_str()).collect();
        println!("search {query:?}: {}", ids.join(", "));
        assert!(
            ids.first().is_some_and(|id| first.contains(id)),
            "{query}: {ids:?}"
        );
        assert!(ids.iter().take(3).any(|id| *id == top3), "{query}: {ids:?}");
    }

    // Vendor and troubleshooting pages open with their provenance and resolved links.
    let et200sp = article(&app, "family.siemens-et200sp");
    assert_eq!(et200sp.vendor.as_deref(), Some("vendor.siemens"));
    assert!(!et200sp.sources.is_empty() && !et200sp.sections.is_empty());
    let siemens = article(&app, "vendor.siemens");
    assert!(!siemens.related.is_empty());
    for r in &siemens.related {
        assert!(!r.title.is_empty(), "vendor.siemens -> {}", r.id);
        article(&app, &r.id);
    }
    let vfd = article(&app, "troubleshooting.vfd-local-remote");
    let t = vfd.troubleshooting.as_ref().expect("troubleshooting body");
    assert_eq!(
        t.simulated_in
            .iter()
            .map(|r| r.id.as_str())
            .collect::<Vec<_>>(),
        ["ts.vfd-local"]
    );
    for r in vfd.related.iter().chain(&t.simulated_in) {
        assert!(!r.title.is_empty(), "vfd -> {}", r.id);
    }
    let exercise = app
        .exercise(IdRequest {
            contract_version: V,
            id: "ts.vfd-local".into(),
        })
        .unwrap();
    assert_eq!(exercise.kind, "troubleshooting");

    // Every catalog entry opens.
    let catalog = app
        .knowledge_catalog(VersionedRequest {
            contract_version: V,
        })
        .unwrap();
    let mut opened = 0;
    let mut stack: Vec<&CatalogEntryDto> = catalog
        .vendors
        .iter()
        .chain(&catalog.protocols)
        .chain(&catalog.fundamentals)
        .chain(&catalog.instructions)
        .chain(&catalog.troubleshooting)
        .collect();
    while let Some(entry) = stack.pop() {
        let a = article(&app, &entry.id);
        for r in &a.related {
            assert!(!r.title.is_empty(), "{} -> {}", a.id, r.id);
        }
        opened += 1;
        stack.extend(&entry.children);
    }
    println!("catalog pages opened: {opened}");
    assert!(opened >= 60, "{opened}");

    // Every lesson and exercise opens, and their references resolve to installed content.
    let library = app
        .library(VersionedRequest {
            contract_version: V,
        })
        .unwrap();
    assert!(library.lessons.len() >= 20 && library.exercises.len() >= 17);
    let id = |id: &str| IdRequest {
        contract_version: V,
        id: id.into(),
    };
    for summary in &library.lessons {
        let lesson = app.lesson(id(&summary.id)).unwrap();
        assert!(!lesson.sections.is_empty() && !lesson.checkpoint.is_empty());
        for r in &lesson.knowledge_refs {
            assert!(!r.title.is_empty(), "{} -> {}", lesson.id, r.id);
        }
        for s in &lesson.sections {
            if let Some(e) = &s.exercise {
                app.exercise(id(e))
                    .unwrap_or_else(|e2| panic!("{} -> {e}: {e2:?}", lesson.id));
            }
            if let Some(scene) = &s.scene {
                assert!(
                    library.scenes.iter().any(|x| &x.id == scene),
                    "{} -> {scene}",
                    lesson.id
                );
            }
        }
    }
    for summary in &library.exercises {
        let exercise = app.exercise(id(&summary.id)).unwrap();
        assert!(library.scenes.iter().any(|x| x.id == exercise.scene));
        for r in &exercise.knowledge_refs {
            assert!(!r.title.is_empty(), "{} -> {}", exercise.id, r.id);
        }
    }
    println!(
        "lessons opened: {}, exercises opened: {}",
        library.lessons.len(),
        library.exercises.len()
    );
    let _ = std::fs::remove_dir_all(&data);
}
