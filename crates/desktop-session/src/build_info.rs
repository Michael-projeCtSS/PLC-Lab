//! What the running executable is: facts the build recorded (source commit, uncommitted
//! changes, build time) and, for a staged distribution, its `resources/product.json`.
//! The manifest is believed only when it names the commit compiled into this executable.

/// The distribution kinds the About page distinguishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Distribution {
    /// A debug build, for example started by TEST_PLC_LAB.bat.
    DevelopmentRun,
    /// A release build that was not staged (no manifest beside it).
    ReleaseBuild,
    UnsignedDevelopmentBuild,
    SignedProductionRelease,
    /// The manifest beside the executable describes another build.
    ManifestMismatch,
}

impl Distribution {
    pub fn code(self) -> &'static str {
        match self {
            Self::DevelopmentRun => "DEVELOPMENT_RUN",
            Self::ReleaseBuild => "RELEASE_BUILD",
            Self::UnsignedDevelopmentBuild => "UNSIGNED_DEVELOPMENT_BUILD",
            Self::SignedProductionRelease => "SIGNED_PRODUCTION_RELEASE",
            Self::ManifestMismatch => "MANIFEST_MISMATCH",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildInfo {
    pub source_commit: Option<String>,
    /// Whether the checkout had changes the commit does not contain; `None` if unknown.
    pub source_dirty: Option<bool>,
    pub built_at_unix: Option<u64>,
    pub distribution: Distribution,
    /// Signed and verified when the release was staged.
    pub signed: bool,
}

impl BuildInfo {
    /// `commit`, `dirty` ("clean" | "dirty" | other) and `built_unix` come from the build
    /// script; `manifest` is the text of `resources/product.json` beside the executable.
    pub fn describe(
        commit: &str,
        dirty: &str,
        built_unix: &str,
        debug: bool,
        manifest: Option<&str>,
    ) -> Self {
        let source_commit = (commit.len() == 40 && commit.chars().all(|c| c.is_ascii_hexdigit()))
            .then(|| commit.to_ascii_lowercase());
        let mut info = Self {
            source_commit: source_commit.clone(),
            source_dirty: match dirty {
                "dirty" => Some(true),
                "clean" => Some(false),
                _ => None,
            },
            built_at_unix: built_unix.parse().ok().filter(|t| *t > 0),
            distribution: if debug {
                Distribution::DevelopmentRun
            } else {
                Distribution::ReleaseBuild
            },
            signed: false,
        };
        if debug {
            return info;
        }
        let Some(manifest) =
            manifest.and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok())
        else {
            return info;
        };
        let named = manifest["sourceCommit"]
            .as_str()
            .map(str::to_ascii_lowercase);
        if named.is_none() || named != source_commit {
            info.distribution = Distribution::ManifestMismatch;
            return info;
        }
        match manifest["releaseKind"].as_str() {
            Some("UNSIGNED_DEVELOPMENT_BUILD") if manifest.get("signing").is_none() => {
                info.distribution = Distribution::UnsignedDevelopmentBuild;
            }
            Some("SIGNED_PRODUCTION_RELEASE") if manifest["signing"].is_object() => {
                info.distribution = Distribution::SignedProductionRelease;
                info.signed = true;
            }
            _ => info.distribution = Distribution::ManifestMismatch,
        }
        info
    }

    /// The build time as RFC 3339 UTC.
    pub fn built_at(&self) -> Option<String> {
        self.built_at_unix.map(rfc3339)
    }
}

/// Seconds since 1970 as `YYYY-MM-DDTHH:MM:SSZ` (proleptic Gregorian, UTC).
fn rfc3339(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let rest = seconds % 86_400;
    // Days to civil date (H. Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3_600,
        rest % 3_600 / 60,
        rest % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMMIT: &str = "d950c2f584491f803cb1e45e954576acf897654f";

    fn manifest(kind: &str, commit: &str, signed: bool) -> String {
        format!(
            r#"{{"releaseKind": "{kind}", "sourceCommit": "{commit}"{}}}"#,
            if signed {
                r#", "signing": {"sha256": "x"}"#
            } else {
                ""
            }
        )
    }

    #[test]
    fn a_debug_build_is_a_development_run_whatever_lies_beside_it() {
        let info = BuildInfo::describe(COMMIT, "dirty", "1794170000", true, None);
        assert_eq!(info.distribution, Distribution::DevelopmentRun);
        assert_eq!(info.source_commit.as_deref(), Some(COMMIT));
        assert_eq!(info.source_dirty, Some(true));
        assert!(!info.signed);
    }

    #[test]
    fn a_staged_build_is_described_by_its_manifest_only_for_the_same_commit() {
        let staged = manifest("UNSIGNED_DEVELOPMENT_BUILD", COMMIT, false);
        let info = BuildInfo::describe(COMMIT, "clean", "1", false, Some(&staged));
        assert_eq!(info.distribution, Distribution::UnsignedDevelopmentBuild);
        assert!(!info.signed);
        let signed = manifest("SIGNED_PRODUCTION_RELEASE", COMMIT, true);
        let info = BuildInfo::describe(COMMIT, "clean", "1", false, Some(&signed));
        assert_eq!(info.distribution, Distribution::SignedProductionRelease);
        assert!(info.signed);
        // A manifest of another build, a production label without a signing record, or
        // an unsigned label with one, describes nothing trustworthy.
        for text in [
            manifest(
                "UNSIGNED_DEVELOPMENT_BUILD",
                "0123456789abcdef0123456789abcdef01234567",
                false,
            ),
            manifest("SIGNED_PRODUCTION_RELEASE", COMMIT, false),
            manifest("UNSIGNED_DEVELOPMENT_BUILD", COMMIT, true),
            "not json".into(),
        ] {
            let info = BuildInfo::describe(COMMIT, "clean", "1", false, Some(&text));
            assert!(!info.signed, "{text}");
            assert_ne!(info.distribution, Distribution::SignedProductionRelease);
        }
        assert_eq!(
            BuildInfo::describe(COMMIT, "clean", "1", false, None).distribution,
            Distribution::ReleaseBuild
        );
    }

    #[test]
    fn unknown_build_facts_stay_unknown() {
        let info = BuildInfo::describe("", "unknown", "0", false, None);
        assert_eq!(info.source_commit, None);
        assert_eq!(info.source_dirty, None);
        assert_eq!(info.built_at(), None);
    }

    #[test]
    fn build_times_are_rfc3339_utc() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(rfc3339(1_794_170_334), "2026-11-08T20:38:54Z");
    }
}
