//! The passive CLI reports retained state without changing stores or original files.
use std::{
    collections::BTreeMap,
    error::Error,
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::SystemTime,
};

use memento::{
    Store,
    capture::{EventKind, Request, Resolution, Scope},
    ingest,
    model::{
        Association, Availability, Entity, Evidence, EvidencePurpose, Fidelity, Record, RecordKind,
        Representation, SourceKind,
    },
    security::RedactionPolicy,
};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;
const META: &str = "__work_context_compaction_v1__";

#[derive(Debug, PartialEq, Eq)]
struct FileState {
    directory: bool,
    bytes: Vec<u8>,
    modified: SystemTime,
    #[cfg(unix)]
    mode: u32,
}

fn snapshot(root: &Path) -> Result<BTreeMap<PathBuf, FileState>, Box<dyn Error>> {
    fn visit(root: &Path, path: &Path, files: &mut BTreeMap<PathBuf, FileState>) -> TestResult {
        let metadata = fs::metadata(path)?;
        files.insert(
            path.strip_prefix(root)?.to_path_buf(),
            FileState {
                directory: metadata.is_dir(),
                bytes: if metadata.is_file() {
                    fs::read(path)?
                } else {
                    Vec::new()
                },
                modified: metadata.modified()?,
                #[cfg(unix)]
                mode: {
                    use std::os::unix::fs::MetadataExt;
                    metadata.mode()
                },
            },
        );
        if metadata.is_dir() {
            for child in fs::read_dir(path)? {
                visit(root, &child?.path(), files)?;
            }
        }
        Ok(())
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files)?;
    Ok(files)
}

fn cli(path: &Path, arguments: &[&str]) -> Result<(bool, Value), Box<dyn Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_memento"))
        .args(["session-status", "--store"])
        .arg(path)
        .args(arguments)
        .output()?;
    Ok((
        output.status.success(),
        serde_json::from_slice(&output.stdout)?,
    ))
}

fn status(path: &Path, limit: &str) -> Result<Value, Box<dyn Error>> {
    let (ok, report) = cli(
        path,
        &[
            "--project",
            "p",
            "--work",
            "w",
            "--session",
            "s",
            "--limit",
            limit,
        ],
    )?;
    assert!(ok, "{report}");
    Ok(report)
}

async fn source(store: &mut Store, project: &str, id: &str) -> TestResult {
    store
        .append(Entity::Source(ingest::source(
            id,
            project,
            SourceKind::Journal,
        )))
        .await?;
    Ok(())
}

fn record(project: &str, source: &str, id: &str, body: &str) -> Record {
    let mut record = Record::new(id, project, source, RecordKind::Finding, body);
    record.title = format!("summary {id}");
    record.work_ids = vec!["w".into()];
    record.session_id = Some("s".into());
    record.association = Association::Explicit;
    record
}

#[derive(Debug, toasty::Model)]
#[table = "stored_entries"]
struct Row {
    #[key]
    #[auto]
    id: u64,
    #[unique]
    event_key: String,
    entity_key: String,
    captured_at: String,
    payload: String,
}

async fn raw(path: &Path) -> Result<toasty::Db, Box<dyn Error>> {
    let mut builder = toasty::Db::builder();
    builder.models(toasty::models!(Row));
    Ok(builder
        .build(toasty_driver_sqlite::Sqlite::open(path))
        .await?)
}

async fn metadata(path: &Path, mutate: impl FnOnce(&mut Value) -> TestResult) -> TestResult {
    let mut db = raw(path).await?;
    let mut row = Row::all()
        .exec(&mut db)
        .await?
        .into_iter()
        .find(|row| row.entity_key == META)
        .ok_or("metadata absent")?;
    let mut value = serde_json::from_str(&row.payload)?;
    mutate(&mut value)?;
    row.update()
        .payload(serde_json::to_string(&value)?)
        .exec(&mut db)
        .await?;
    Ok(())
}

#[test]
fn missing_and_empty_stores_are_reported_without_creating_anything() -> TestResult {
    let directory = tempfile::tempdir()?;
    let missing = directory.path().join("nested/context.sqlite");
    let before = snapshot(directory.path())?;
    let report = status(&missing, "20")?;
    assert_eq!(
        report.pointer("/store_format/state"),
        Some(&json!("missing"))
    );
    assert_eq!(report.get("read_only"), Some(&json!(true)));
    assert_eq!(report.get("record_count"), Some(&json!(0)));
    assert_eq!(report.get("last_saved_at"), Some(&Value::Null));
    assert_eq!(report.get("checkpoint_state"), Some(&json!("not_observed")));
    assert_eq!(report.get("store"), Some(&json!(missing)));
    assert_eq!(snapshot(directory.path())?, before);

    let empty = directory.path().join("empty.sqlite");
    fs::write(&empty, [])?;
    let before = snapshot(directory.path())?;
    assert_eq!(status(&empty, "20")?.get("record_count"), Some(&json!(0)));
    assert_eq!(snapshot(directory.path())?, before);
    Ok(())
}

#[tokio::test]
async fn latest_revisions_are_selected_before_scope_and_limit_filters() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("context.sqlite");
    let mut store = Store::open(&path, RedactionPolicy::default()).await?;
    for (project, id) in [("p", "a"), ("p", "b"), ("other", "a")] {
        source(&mut store, project, id).await?;
    }
    for (project, source_id, id, body) in [
        ("p", "a", "shared", "OLD_BODY_MUST_NOT_LEAK"),
        ("p", "b", "shared", "SECOND_SOURCE_BODY"),
        ("other", "a", "shared", "OTHER_PROJECT_BODY"),
        ("p", "a", "moved", "PREVIOUS_SESSION_BODY"),
        ("p", "a", "moved-work", "PREVIOUS_WORK_BODY"),
    ] {
        store
            .append(Entity::Record(record(project, source_id, id, body)))
            .await?;
    }
    let mut moved = record("p", "a", "moved", "NEW_SESSION_BODY");
    moved.session_id = Some("another-session".into());
    store.append(Entity::Record(moved)).await?;
    let mut moved_work = record("p", "a", "moved-work", "NEW_WORK_BODY");
    moved_work.work_ids = vec!["another-work".into()];
    store.append(Entity::Record(moved_work)).await?;
    let newest = record("p", "a", "shared", "NEWEST_BODY_MUST_NOT_LEAK");
    let revision = newest.revision.clone();
    let receipt = store.append(Entity::Record(newest)).await?;
    let captured = store
        .load()
        .await?
        .entries
        .into_iter()
        .find(|entry| entry.sequence == receipt.sequence)
        .ok_or("newest entry absent")?
        .captured_at;
    drop(store);

    let before = snapshot(directory.path())?;
    let report = status(&path, "1")?;
    assert_eq!(report.get("record_count"), Some(&json!(2)));
    assert_eq!(report.get("omitted_records"), Some(&json!(1)));
    assert_eq!(report.get("last_saved_at"), Some(&json!(captured)));
    assert_eq!(report.pointer("/records/0/source_id"), Some(&json!("a")));
    assert_eq!(
        report.pointer("/records/0/revision"),
        Some(&json!(revision))
    );
    assert_eq!(
        report.pointer("/records/0/sequence"),
        Some(&json!(receipt.sequence))
    );
    let all = status(&path, "100")?;
    assert_eq!(all.pointer("/records/1/source_id"), Some(&json!("b")));
    assert_eq!(
        all.get("scope"),
        Some(&json!({"project_id":"p","work_id":"w","session_id":"s"}))
    );
    assert!(!serde_json::to_string(&all)?.contains("BODY"));
    for summary in all
        .get("records")
        .and_then(Value::as_array)
        .ok_or("records absent")?
    {
        assert!(summary.get("body").is_none());
        assert!(summary.get("evidence").is_none());
    }
    assert_eq!(snapshot(directory.path())?, before);
    Ok(())
}

#[tokio::test]
async fn retained_access_and_quality_are_reported_without_revalidating_original_files() -> TestResult
{
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("context.sqlite");
    let original = directory.path().join("original.txt");
    fs::write(&original, "ORIGINAL_FILE_BODY")?;
    let mut store = Store::open(&path, RedactionPolicy::default()).await?;
    let mut document = ingest::source("document", "p", SourceKind::Document);
    document.location = Some(original.to_string_lossy().into_owned());
    store.append(Entity::Source(document)).await?;
    source(&mut store, "p", "revoked").await?;
    let mut unavailable = ingest::source("unavailable", "p", SourceKind::Journal);
    unavailable.available = false;
    store.append(Entity::Source(unavailable)).await?;
    store
        .append(Entity::Record(record(
            "p",
            "unavailable",
            "source-down",
            "UNAVAILABLE_SOURCE_BODY",
        )))
        .await?;
    for id in ["retained", "deleted"] {
        store
            .append(Entity::Record(record("p", "document", id, "RETAINED_BODY")))
            .await?;
    }
    let mut deleted = record("p", "document", "deleted", "");
    deleted.availability = Availability::Deleted;
    store.append(Entity::Record(deleted)).await?;
    store
        .append(Entity::Record(record(
            "p",
            "revoked",
            "hidden",
            "REVOKED_BODY",
        )))
        .await?;
    let mut revoked = ingest::source("revoked", "p", SourceKind::Journal);
    revoked.authorized = false;
    store.append(Entity::Source(revoked)).await?;
    let mut partial = record("p", "document", "partial", "PARTIAL_BODY");
    partial.availability = Availability::Missing;
    partial.fidelity = Fidelity::SourceTruncated;
    partial.partial = true;
    partial.association = Association::Candidate;
    partial.representation = Representation::Evidence;
    store.append(Entity::Record(partial)).await?;
    let mut redacted = record("p", "document", "redacted", "REDACTED_BODY");
    redacted.availability = Availability::Redacted;
    redacted.fidelity = Fidelity::SummaryOnly;
    store.append(Entity::Record(redacted)).await?;
    drop(store);
    fs::remove_file(original)?;

    let before = snapshot(directory.path())?;
    let report = status(&path, "20")?;
    assert_eq!(report.get("record_count"), Some(&json!(4)));
    let records = report
        .get("records")
        .and_then(Value::as_array)
        .ok_or("records absent")?;
    let retained = records
        .iter()
        .find(|row| row.get("record_id") == Some(&json!("retained")))
        .ok_or("retained row absent")?;
    assert_eq!(retained.get("availability"), Some(&json!("available")));
    assert_eq!(retained.get("source_available"), Some(&json!(true)));
    assert_eq!(retained.get("association"), Some(&json!("explicit")));
    let unavailable = records
        .iter()
        .find(|row| row.get("record_id") == Some(&json!("source-down")))
        .ok_or("unavailable source row absent")?;
    assert_eq!(unavailable.get("source_available"), Some(&json!(false)));
    assert_eq!(unavailable.get("availability"), Some(&json!("available")));
    let partial = records
        .iter()
        .find(|row| row.get("record_id") == Some(&json!("partial")))
        .ok_or("partial row absent")?;
    assert_eq!(partial.get("availability"), Some(&json!("missing")));
    assert_eq!(partial.get("fidelity"), Some(&json!("source_truncated")));
    assert_eq!(partial.get("partial"), Some(&json!(true)));
    assert_eq!(partial.get("representation"), Some(&json!("evidence")));
    assert_eq!(partial.get("association"), Some(&json!("candidate")));
    let redacted = records
        .iter()
        .find(|row| row.get("record_id") == Some(&json!("redacted")))
        .ok_or("redacted row absent")?;
    assert_eq!(redacted.get("availability"), Some(&json!("redacted")));
    assert_eq!(redacted.get("fidelity"), Some(&json!("summary_only")));
    assert!(!serde_json::to_string(&report)?.contains("BODY"));
    assert_eq!(snapshot(directory.path())?, before);
    Ok(())
}

#[tokio::test]
async fn unavailable_derived_titles_do_not_reveal_revoked_or_deleted_evidence() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("context.sqlite");
    let mut store = Store::open(&path, RedactionPolicy::default()).await?;
    for id in ["journal", "revoked", "deleted"] {
        source(&mut store, "p", id).await?;
    }
    let mut revoked = ingest::source("revoked", "p", SourceKind::Journal);
    revoked.authorized = false;
    store.append(Entity::Source(revoked)).await?;
    let mut deleted = record("p", "deleted", "origin", "");
    deleted.availability = Availability::Deleted;
    store.append(Entity::Record(deleted)).await?;
    for source in ["revoked", "deleted"] {
        let mut derived = record("p", "journal", source, "SECRET_DERIVED_BODY");
        derived.title = "SECRET_DERIVED_TITLE".into();
        derived.availability = Availability::Missing;
        derived.derived = true;
        derived.evidence.push(Evidence {
            source_id: source.into(),
            record_id: Some("origin".into()),
            revision: "retained-revision".into(),
            locator: "original evidence".into(),
            availability: Availability::Available,
            range: None,
            purpose: EvidencePurpose::Origin,
            span: None,
        });
        store.append(Entity::Record(derived)).await?;
    }
    drop(store);
    let before = snapshot(directory.path())?;
    let report = status(&path, "20")?;
    assert_eq!(report.get("record_count"), Some(&json!(2)));
    for row in report
        .get("records")
        .and_then(Value::as_array)
        .ok_or("records absent")?
    {
        assert_eq!(row.get("title"), Some(&json!("")));
    }
    assert!(!serde_json::to_string(&report)?.contains("SECRET"));
    assert_eq!(snapshot(directory.path())?, before);
    Ok(())
}

#[tokio::test]
async fn checkpoints_survive_repository_removal_and_keep_each_repository_scope() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("context.sqlite");
    let mut store = Store::open(&path, RedactionPolicy::default()).await?;
    let repositories = [directory.path().join("a"), directory.path().join("b")];
    for (index, repository) in repositories.iter().enumerate() {
        fs::create_dir(repository)?;
        let scope = Scope {
            project_id: "p".into(),
            repository: repository.to_string_lossy().into_owned(),
            work_id: "w".into(),
            session_id: "s".into(),
            turn_id: "t".into(),
        };
        store
            .checkpoint(Request::Open {
                scope: scope.clone(),
                event_id: "capture".into(),
                kind: EventKind::UserPrompt,
                detail: "request requires preservation".into(),
                commit_binding: None,
            })
            .await?;
        if index == 1 {
            store
                .checkpoint(Request::Resolve {
                    scope,
                    event_id: "capture".into(),
                    resolution: Resolution::CaptureIncomplete {
                        reason: "interrupted before capture".into(),
                    },
                })
                .await?;
        }
    }
    drop(store);
    let canonical_repositories = repositories
        .iter()
        .map(fs::canonicalize)
        .collect::<Result<Vec<_>, _>>()?;
    for repository in &repositories {
        fs::remove_dir(repository)?;
    }
    let before = snapshot(directory.path())?;
    let report = status(&path, "20")?;
    assert_eq!(report.get("checkpoint_state"), Some(&json!("observed")));
    assert_eq!(
        report.pointer("/checkpoints/0/repository"),
        Some(&json!(
            canonical_repositories.first().ok_or("repository absent")?
        ))
    );
    assert_eq!(
        report.pointer("/checkpoints/0/pending/0/event_id"),
        Some(&json!("capture"))
    );
    assert_eq!(
        report.pointer("/checkpoints/1/capture_incomplete/0/reason"),
        Some(&json!("interrupted before capture"))
    );
    let (ok, other) = cli(
        &path,
        &["--project", "p", "--work", "w", "--session", "new-session"],
    )?;
    assert!(ok);
    assert_eq!(other.get("checkpoint_state"), Some(&json!("not_observed")));
    assert_eq!(other.get("checkpoints"), Some(&json!([])));
    assert_eq!(snapshot(directory.path())?, before);
    Ok(())
}

#[tokio::test]
async fn known_legacy_stores_are_read_without_migration() -> TestResult {
    for version in [0, 1] {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("context.sqlite");
        let mut store = Store::open(&path, RedactionPolicy::default()).await?;
        source(&mut store, "p", "journal").await?;
        store
            .append(Entity::Record(record(
                "p",
                "journal",
                "finding",
                "LEGACY_BODY",
            )))
            .await?;
        drop(store);
        metadata(&path, |value| {
            if version == 0 {
                value
                    .as_object_mut()
                    .ok_or("metadata object absent")?
                    .remove("store_format");
            } else {
                *value
                    .pointer_mut("/store_format/version")
                    .ok_or("format version absent")? = json!(1);
            }
            Ok(())
        })
        .await?;
        let before = snapshot(directory.path())?;
        let report = status(&path, "20")?;
        assert_eq!(
            report.pointer("/store_format/state"),
            Some(&json!("migration_required"))
        );
        assert_eq!(
            report.pointer("/store_format/format_version"),
            Some(&json!(version))
        );
        assert_eq!(report.get("record_count"), Some(&json!(1)));
        assert_eq!(snapshot(directory.path())?, before);
    }
    Ok(())
}

#[tokio::test]
async fn unknown_and_corrupt_stores_fail_with_typed_errors_without_changes() -> TestResult {
    let directory = tempfile::tempdir()?;
    let unknown = directory.path().join("unknown.sqlite");
    fs::write(&unknown, "not a database")?;
    let before = snapshot(directory.path())?;
    let (ok, error) = cli(
        &unknown,
        &["--project", "p", "--work", "w", "--session", "s"],
    )?;
    assert!(!ok);
    assert_eq!(
        error.pointer("/error/code"),
        Some(&json!("store_format_invalid"))
    );
    assert_eq!(snapshot(directory.path())?, before);

    let path = directory.path().join("context.sqlite");
    drop(Store::open(&path, RedactionPolicy::default()).await?);
    metadata(&path, |value| {
        *value
            .pointer_mut("/store_format/version")
            .ok_or("version absent")? = json!(3);
        Ok(())
    })
    .await?;
    let before = snapshot(directory.path())?;
    let (ok, error) = cli(&path, &["--project", "p", "--work", "w", "--session", "s"])?;
    assert!(!ok);
    assert_eq!(
        error.pointer("/error/code"),
        Some(&json!("store_format_unsupported"))
    );
    assert_eq!(snapshot(directory.path())?, before);
    metadata(&path, |value| {
        *value
            .pointer_mut("/store_format/version")
            .ok_or("version absent")? = json!(2);
        value
            .as_object_mut()
            .ok_or("metadata object absent")?
            .insert("unknown_field".into(), json!(true));
        Ok(())
    })
    .await?;
    let before = snapshot(directory.path())?;
    let (ok, error) = cli(&path, &["--project", "p", "--work", "w", "--session", "s"])?;
    assert!(!ok);
    assert_eq!(
        error.pointer("/error/code"),
        Some(&json!("store_format_invalid"))
    );
    assert_eq!(snapshot(directory.path())?, before);
    Ok(())
}

#[test]
fn exact_scope_and_limit_are_validated_before_inspecting_the_store() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("unknown.sqlite");
    fs::write(&path, "not a database")?;
    let before = snapshot(directory.path())?;
    for (args, missing) in [
        (vec![], "--project"),
        (vec!["--project", "p"], "--work"),
        (vec!["--project", "p", "--work", "w"], "--session"),
        (
            vec!["--project", "", "--work", "w", "--session", "s"],
            "session-status requires",
        ),
    ] {
        let (ok, error) = cli(&path, &args)?;
        assert!(!ok);
        assert!(
            error
                .get("error")
                .and_then(Value::as_str)
                .is_some_and(|text| text.contains(missing))
        );
    }
    for limit in ["0", "101", "-1", "garbage"] {
        let (ok, error) = cli(
            &path,
            &[
                "--project",
                "p",
                "--work",
                "w",
                "--session",
                "s",
                "--limit",
                limit,
            ],
        )?;
        assert!(!ok);
        assert!(
            error
                .get("error")
                .and_then(Value::as_str)
                .is_some_and(|text| text.contains("--limit"))
        );
    }
    assert_eq!(snapshot(directory.path())?, before);
    Ok(())
}

#[tokio::test]
async fn exclusive_sqlite_lock_returns_a_structured_cli_failure_without_changes() -> TestResult {
    struct ExclusiveLock(Child);
    impl Drop for ExclusiveLock {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    let directory = tempfile::tempdir()?;
    let path = directory.path().join("context.sqlite");
    drop(Store::open(&path, RedactionPolicy::default()).await?);
    let script = "import sqlite3,sys\nc=sqlite3.connect(sys.argv[1])\nc.execute('BEGIN EXCLUSIVE')\nprint('locked',flush=True)\nsys.stdin.readline()\nc.rollback()\nc.close()\n";
    let mut lock = ExclusiveLock(
        Command::new("python3")
            .args(["-c", script])
            .arg(&path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?,
    );
    let output = lock.0.stdout.take().ok_or("lock handshake absent")?;
    let mut ready = String::new();
    BufReader::new(output).read_line(&mut ready)?;
    assert_eq!(ready.trim(), "locked");
    let before = snapshot(directory.path())?;
    let output = Command::new(env!("CARGO_BIN_EXE_memento"))
        .args(["session-status", "--store"])
        .arg(&path)
        .args(["--project", "p", "--work", "w", "--session", "s"])
        .output()?;
    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        error.pointer("/error/code"),
        Some(&json!("store_inspection_unavailable"))
    );
    assert_eq!(snapshot(directory.path())?, before);
    Ok(())
}
