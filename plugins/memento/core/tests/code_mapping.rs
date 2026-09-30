use std::{error::Error, fs, path::Path, process::Command};

use memento::{
    Store, ingest,
    mapping::{self, Correspondence, IssueKind, MappingMethod, MappingRequest},
    model::*,
    query::{self, LocationStatus},
    runtime,
    security::{RedactionPolicy, hash},
};

#[path = "support/evaluation.rs"]
mod evaluation;

type TestResult = Result<(), Box<dyn Error>>;
const PROJECT: &str = "mapping-fixture";
const RETRY: &str = "pub fn retry(delay: u64) {\n    sleep(delay);\n    upload();\n}\n";
const EXECUTE: &str = "pub fn execute(delay: u64) {\n    sleep(delay);\n    upload();\n}\n";

fn file(path: &str, content: &str) -> FileState {
    FileState {
        path: path.into(),
        working_hash: Some(hash(content.as_bytes())),
        working_content: Some(content.into()),
        working_kind: WorkingFileKind::File,
        ..FileState::default()
    }
}

fn state(id: &str, files: Vec<FileState>) -> CodeState {
    CodeState {
        id: id.into(),
        project_id: PROJECT.into(),
        source_id: "git".into(),
        repository_id: PROJECT.into(),
        worktree_id: Some("worktree-a".into()),
        commit_sha: None,
        observed_at: "2026-09-28T10:00:00Z".into(),
        changed_during_observation: false,
        files,
    }
}

fn original(state: &CodeState, path: &str, range: Option<TextRange>) -> CodeRef {
    CodeRef {
        state_id: state.id.clone(),
        path: path.into(),
        range,
    }
}

fn function_ref(state: &CodeState) -> CodeRef {
    original(
        state,
        "src/retry.rs",
        Some(TextRange {
            start_line: 1,
            end_line: 4,
        }),
    )
}

fn append(corpus: &mut Corpus, entity: Entity) {
    corpus.entries.push(Entry {
        sequence: corpus
            .entries
            .last()
            .map_or(1, |entry| entry.sequence.saturating_add(1)),
        captured_at: "2026-09-28T10:00:00Z".into(),
        entity,
    });
}

fn corpus(from: &CodeState, to: &CodeState) -> Corpus {
    let mut result = Corpus::default();
    append(
        &mut result,
        Entity::Source(ingest::source("git", PROJECT, SourceKind::Git)),
    );
    append(
        &mut result,
        Entity::Source(ingest::source("journal", PROJECT, SourceKind::Journal)),
    );
    append(&mut result, Entity::CodeState(from.clone()));
    append(&mut result, Entity::CodeState(to.clone()));
    let mut decision = Record::new(
        "D15",
        PROJECT,
        "journal",
        RecordKind::Decision,
        "Use bounded retry after the observed upload throttling; this decision applied to the original retry function. Destination behavior has not been reverified.",
    );
    decision.decision_status = Some(DecisionStatus::Accepted);
    decision.code_refs.push(function_ref(from));
    append(&mut result, Entity::Record(decision));
    append(
        &mut result,
        Entity::Relation(Relation {
            id: "reason-original".into(),
            project_id: PROJECT.into(),
            source_id: "journal".into(),
            from: Target::Code {
                state_id: from.id.clone(),
                path: "src/retry.rs".into(),
                range: Some(TextRange {
                    start_line: 1,
                    end_line: 4,
                }),
            },
            to: Target::Record { id: "D15".into() },
            kind: RelationKind::RelatedTo,
            nature: Nature::Reported,
            evidence: Vec::new(),
            applies_to: vec![from.id.clone()],
        }),
    );
    result
}

fn query(from: &CodeState, to: &CodeState, paths: &[&str]) -> Query {
    let mut query = Query::new(Operation::Trace, PROJECT);
    query.target = Some(Target::Code {
        state_id: from.id.clone(),
        path: "src/retry.rs".into(),
        range: Some(TextRange {
            start_line: 1,
            end_line: 4,
        }),
    });
    query.code_mapping = Some(MappingRequest {
        target_state_id: to.id.clone(),
        target_source_id: Some("git".into()),
        paths: paths.iter().map(|path| (*path).into()).collect(),
    });
    query.limit = Some(100);
    query
}

#[test]
fn rust_ast_maps_renamed_function_with_module_and_impl_provenance() -> TestResult {
    let from = state("before", vec![file("src/retry.rs", RETRY)]);
    let to = state(
        "after",
        vec![file(
            "src/network/backoff.rs",
            "mod network {\n    pub fn execute(delay: u64) {\n        sleep(delay);\n        upload();\n    }\n}\n",
        )],
    );
    let report = mapping::map(
        &function_ref(&from),
        &from,
        &to,
        &["src/network/backoff.rs".into()],
    );
    assert_eq!(report.status, LocationStatus::Mapped);
    let candidate = report.candidates.first().ok_or("expected candidate")?;
    assert_eq!(candidate.method, MappingMethod::RustFunction);
    assert_eq!(candidate.correspondence, Correspondence::AstEquivalent);
    assert_eq!(
        candidate.destination.range,
        Some(TextRange {
            start_line: 2,
            end_line: 5
        })
    );
    let symbol = candidate
        .destination_symbol
        .as_ref()
        .ok_or("expected symbol")?;
    assert_eq!(symbol.name, "execute");
    assert_eq!(symbol.container, ["mod network"]);
    assert_eq!(
        candidate
            .original_symbol
            .as_ref()
            .map(|symbol| symbol.name.as_str()),
        Some("retry")
    );
    assert_eq!(report.original, function_ref(&from));
    assert!(
        report
            .limitations
            .iter()
            .any(|limit| limit.contains("verification"))
    );

    let from = state(
        "impl-before",
        vec![file(
            "src/client.rs",
            "struct Client;\nimpl Client {\n fn retry(&self) { upload(); }\n}\n",
        )],
    );
    let to = state(
        "impl-after",
        vec![file(
            "src/client.rs",
            "struct Client;\nimpl Client {\n fn execute(&self) { upload(); }\n}\n",
        )],
    );
    let report = mapping::map(
        &original(
            &from,
            "src/client.rs",
            Some(TextRange {
                start_line: 3,
                end_line: 3,
            }),
        ),
        &from,
        &to,
        &["src/client.rs".into()],
    );
    assert_eq!(report.status, LocationStatus::Mapped);
    assert_eq!(
        report
            .candidates
            .first()
            .and_then(|c| c.destination_symbol.as_ref())
            .map(|s| s.container.clone()),
        Some(vec!["impl Client".into()])
    );
    Ok(())
}

#[test]
fn file_rename_and_inserted_lines_use_same_layer_content_evidence() -> TestResult {
    let from = state("before", vec![file("src/retry.rs", RETRY)]);
    let mut to = state("after", vec![file("src/new.rs", RETRY)]);
    let report = mapping::map(&function_ref(&from), &from, &to, &["src/new.rs".into()]);
    assert_eq!(report.status, LocationStatus::Mapped);
    assert_eq!(
        report.candidates.first().map(|c| c.method),
        Some(MappingMethod::WorkingContent)
    );
    to.files = vec![file(
        "src/retry.rs",
        &format!("// a new header\n// second header\n{RETRY}"),
    )];
    let report = mapping::map(&function_ref(&from), &from, &to, &["src/retry.rs".into()]);
    assert_eq!(report.status, LocationStatus::Mapped);
    assert_eq!(
        report.candidates.first().map(|c| c.method),
        Some(MappingMethod::ExactLines)
    );
    assert_eq!(
        report
            .candidates
            .first()
            .and_then(|c| c.destination.range.clone()),
        Some(TextRange {
            start_line: 3,
            end_line: 6
        })
    );
    let data = corpus(&from, &to);
    evaluation::capture(
        "P1-C03",
        "A",
        "파일 앞에 주석 두 줄이 추가됐다. 과거 retry 함수 범위가 지금 어디로 이동했으며, 이전 결정을 현재 검증으로 볼 수 있나?",
        &data,
        &[query(&from, &to, &["src/retry.rs"])],
    )?;
    Ok(())
}

#[test]
fn copied_function_and_modified_copy_remain_two_candidates() -> TestResult {
    let from = state("before", vec![file("src/retry.rs", RETRY)]);
    let to = state(
        "after",
        vec![
            file("src/a.rs", EXECUTE),
            file(
                "src/b.rs",
                "pub fn execute(delay: u64) {\n    sleep(delay);\n    upload();\n    audit();\n}\n",
            ),
        ],
    );
    let report = mapping::map(
        &function_ref(&from),
        &from,
        &to,
        &["src/a.rs".into(), "src/b.rs".into()],
    );
    assert_eq!(report.status, LocationStatus::Ambiguous);
    assert_eq!(report.candidates.len(), 2);
    assert!(
        report
            .candidates
            .iter()
            .any(|c| c.correspondence == Correspondence::AstEquivalent)
    );
    assert!(
        report
            .candidates
            .iter()
            .any(|c| c.correspondence == Correspondence::Partial)
    );
    let data = corpus(&from, &to);
    evaluation::capture(
        "P1-C02",
        "A",
        "retry를 두 모듈에 복사하고 한쪽만 audit 호출을 추가했다. 예전 함수의 후계자는 어디이며 어느 쪽에 예전 결정과 검증을 적용할 수 있나?",
        &data,
        &[query(&from, &to, &["src/a.rs", "src/b.rs"])],
    )?;
    let mut incomplete = to.clone();
    incomplete.files.retain(|f| f.path == "src/a.rs");
    let report = mapping::map(
        &function_ref(&from),
        &from,
        &incomplete,
        &["src/a.rs".into(), "src/b.rs".into()],
    );
    assert_eq!(report.status, LocationStatus::Ambiguous);
    assert_eq!(report.candidates.len(), 1);
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.kind == IssueKind::DestinationNotObserved)
    );
    evaluation::capture(
        "P1-C02",
        "B",
        "후계자 후보 두 경로를 지정했지만 b 경로는 관측되지 않았다. a를 유일한 후계자로 확정하고 과거 검증을 적용해도 되나?",
        &corpus(&from, &incomplete),
        &[query(&from, &incomplete, &["src/a.rs", "src/b.rs"])],
    )?;
    Ok(())
}

#[test]
fn split_merge_and_recreated_path_are_not_unique_successors() -> TestResult {
    let from = state("before", vec![file("src/retry.rs", RETRY)]);
    let split = state(
        "split",
        vec![file(
            "src/backoff.rs",
            "pub fn wait(delay: u64) {\n    sleep(delay);\n}\npub fn execute() {\n    upload();\n}\n",
        )],
    );
    let report = mapping::map(
        &function_ref(&from),
        &from,
        &split,
        &["src/backoff.rs".into()],
    );
    assert_eq!(report.status, LocationStatus::Ambiguous);
    assert_eq!(report.candidates.len(), 2);
    assert!(
        report
            .candidates
            .iter()
            .all(|c| c.correspondence == Correspondence::Partial)
    );
    // The two split functions can also be merged into one function. Preserve
    // both original supports even when their destination line ranges coincide.
    let merged = mapping::map(
        &original(
            &split,
            "src/backoff.rs",
            Some(TextRange {
                start_line: 1,
                end_line: 6,
            }),
        ),
        &split,
        &from,
        &["src/retry.rs".into()],
    );
    assert_eq!(merged.status, LocationStatus::Ambiguous);
    assert_eq!(merged.candidates.len(), 2);
    assert!(merged.candidates.iter().all(|candidate| {
        candidate.correspondence == Correspondence::Partial
            && candidate.destination.state_id == from.id
    }));
    let supports: Vec<_> = merged
        .candidates
        .iter()
        .filter_map(|candidate| {
            candidate
                .original_symbol
                .as_ref()
                .map(|symbol| symbol.name.as_str())
        })
        .collect();
    assert_eq!(supports, vec!["wait", "execute"]);
    let mut unstable = split.clone();
    unstable.changed_during_observation = true;
    evaluation::capture(
        "P1-C03",
        "B",
        "retry 함수가 두 함수로 나뉘었고 목적 관측 중 파일이 바뀌었다. 확정적으로 옮겨간 위치와 과거 결정의 현재 적용 여부를 설명해줘.",
        &corpus(&from, &unstable),
        &[query(&from, &unstable, &["src/backoff.rs"])],
    )?;
    let recreated = state(
        "recreated",
        vec![file(
            "src/retry.rs",
            "pub fn retry() { completely_different_work(); }\n",
        )],
    );
    let report = mapping::map(
        &function_ref(&from),
        &from,
        &recreated,
        &["src/retry.rs".into()],
    );
    assert_eq!(report.status, LocationStatus::Missing);
    assert!(report.candidates.is_empty());
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.kind == IssueKind::NoCorrespondence)
    );
    Ok(())
}

#[test]
fn hash_only_masked_and_wrong_hash_layer_cannot_establish_mapping() {
    let full = state("before", vec![file("src/retry.rs", RETRY)]);
    let to = state("after", vec![file("src/new.rs", RETRY)]);
    let reference = function_ref(&full);
    for variant in ["hash-only", "masked", "wrong-layer"] {
        let mut from = full.clone();
        if let Some(file) = from.files.first_mut() {
            match variant {
                "hash-only" => file.working_content = None,
                "masked" => file.working_content = Some("[REDACTED]\n".into()),
                _ => {
                    file.head_hash = file.working_hash.take();
                    file.index_hash = file.head_hash.clone();
                }
            }
        }
        let report = mapping::map(&reference, &from, &to, &["src/new.rs".into()]);
        assert_eq!(report.status, LocationStatus::Unavailable);
        assert!(report.candidates.is_empty());
    }
}

#[test]
fn missing_unobserved_unstable_and_unparsed_states_are_distinguished() {
    let from = state("before", vec![file("src/retry.rs", RETRY)]);
    let mut to = state(
        "after",
        vec![FileState {
            path: "src/new.rs".into(),
            working_kind: WorkingFileKind::Missing,
            ..FileState::default()
        }],
    );
    let report = mapping::map(&function_ref(&from), &from, &to, &["src/new.rs".into()]);
    assert_eq!(report.status, LocationStatus::Missing);
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.kind == IssueKind::ExplicitlyMissing)
    );
    let report = mapping::map(
        &function_ref(&from),
        &from,
        &to,
        &["src/unobserved.rs".into()],
    );
    assert_eq!(report.status, LocationStatus::Unavailable);
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.kind == IssueKind::DestinationNotObserved)
    );
    to.files = vec![file("src/new.rs", EXECUTE)];
    to.changed_during_observation = true;
    assert_eq!(
        mapping::map(&function_ref(&from), &from, &to, &["src/new.rs".into()]).status,
        LocationStatus::Ambiguous
    );
    to.changed_during_observation = false;
    to.files.push(file("src/broken.rs", "fn cannot_parse( {"));
    let report = mapping::map(
        &function_ref(&from),
        &from,
        &to,
        &["src/new.rs".into(), "src/broken.rs".into()],
    );
    assert_eq!(report.status, LocationStatus::Ambiguous);
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.kind == IssueKind::ParseFailed)
    );
}

fn git(repository: &Path, args: &[&str]) -> Result<(), Box<dyn Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(args)
        .output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    Ok(())
}

#[tokio::test]
async fn real_git_move_rename_reopen_and_historical_trace_preserve_original_decision() -> TestResult
{
    let temp = tempfile::tempdir()?;
    let repository = temp.path().join("repository");
    fs::create_dir_all(repository.join("src/network"))?;
    git(&repository, &["init", "--quiet", "--initial-branch=main"])?;
    git(&repository, &["config", "user.name", "Mapping fixture"])?;
    git(
        &repository,
        &["config", "user.email", "mapping@example.invalid"],
    )?;
    git(&repository, &["config", "commit.gpgsign", "false"])?;
    git(
        &repository,
        &["config", "core.hooksPath", ".git/test-hooks"],
    )?;
    fs::write(repository.join("src/retry.rs"), RETRY)?;
    git(&repository, &["add", "src/retry.rs"])?;
    git(&repository, &["commit", "--quiet", "-m", "retry baseline"])?;
    let database = temp.path().join("context.sqlite");
    let mut store = Store::open(&database, RedactionPolicy::default()).await?;
    let from = runtime::observe(&mut store, PROJECT, &repository, &["src/retry.rs".into()]).await?;
    git(
        &repository,
        &["mv", "src/retry.rs", "src/network/backoff.rs"],
    )?;
    fs::write(repository.join("src/network/backoff.rs"), EXECUTE)?;
    let to = runtime::observe(
        &mut store,
        PROJECT,
        &repository,
        &["src/retry.rs".into(), "src/network/backoff.rs".into()],
    )
    .await?;
    for entry in corpus(&from, &to).entries {
        if entry.entity.source_id() == "journal" {
            store.append(entry.entity).await?;
        }
    }
    drop(store);
    let mut reopened = Store::open(&database, RedactionPolicy::default()).await?;
    let data = reopened.load().await?;
    let request = query(&from, &to, &["src/retry.rs", "src/network/backoff.rs"]);
    let response = query::execute(&data, &request)?;
    let report = response
        .location
        .as_ref()
        .and_then(|location| location.mapping.as_ref())
        .ok_or("missing mapping report")?;
    assert_eq!(report.status, LocationStatus::Mapped);
    assert!(response.items.iter().any(|item| item.entity.id() == "D15"));
    assert!(
        response
            .relations
            .iter()
            .all(|relation| relation.kind != RelationKind::Verifies)
    );
    assert!(
        response.relations.iter().all(
            |relation| !matches!(&relation.from,Target::Code{state_id,..} if state_id == &to.id)
        )
    );
    assert_eq!(report.original.state_id, from.id);
    assert_eq!(
        report
            .candidates
            .first()
            .map(|candidate| candidate.destination.path.as_str()),
        Some("src/network/backoff.rs")
    );
    evaluation::capture(
        "P1-C01",
        "A",
        "retry 함수를 network/backoff.rs의 execute로 옮기고 이름을 바꿨다. 실제 목적 위치와 과거 D15 결정의 근거를 찾아줘. 목적 코드도 이미 검증되었다고 말할 수 있나?",
        &data,
        std::slice::from_ref(&request),
    )?;
    let mut missing = from.clone();
    if let Some(file) = missing.files.first_mut() {
        file.working_content = None;
    }
    let data = corpus(&missing, &to);
    let response = query::execute(&data, &request)?;
    assert_eq!(
        response
            .location
            .as_ref()
            .and_then(|location| location.mapping.as_ref())
            .map(|report| report.status),
        Some(LocationStatus::Unavailable)
    );
    assert!(response.items.iter().any(|item| item.entity.id() == "D15"));
    evaluation::capture(
        "P1-C01",
        "B",
        "과거 retry 본문이 보존되지 않고 hash와 D15 결정만 남았다. 목적 execute로의 이동을 확정할 수 있나? 무엇은 알려져 있고 무엇이 빠졌나?",
        &data,
        &[request],
    )?;
    Ok(())
}

#[test]
fn mapping_scope_revocation_and_invalid_requests_never_expose_other_states() -> TestResult {
    let from = state("before", vec![file("src/retry.rs", RETRY)]);
    let to = state("after", vec![file("src/new.rs", EXECUTE)]);
    let request = query(&from, &to, &["src/new.rs"]);
    let mut data = corpus(&from, &to);
    let mut secret = to.clone();
    secret.id = "hidden".into();
    secret.source_id = "restricted".into();
    append(&mut data, Entity::CodeState(secret));
    let mut hidden_request = request.clone();
    if let Some(mapping) = &mut hidden_request.code_mapping {
        mapping.target_state_id = "hidden".into();
        mapping.target_source_id = Some("restricted".into());
    }
    assert!(query::execute(&data, &hidden_request).is_err());
    let mut source = ingest::source("git", PROJECT, SourceKind::Git);
    source.authorized = false;
    append(&mut data, Entity::Source(source));
    assert!(query::execute(&data, &request).is_err());
    let mut other = to.clone();
    other.repository_id = "other".into();
    assert_eq!(
        mapping::map(&function_ref(&from), &from, &other, &["src/new.rs".into()]).status,
        LocationStatus::Unavailable
    );
    assert!(
        mapping::map(&function_ref(&from), &from, &to, &[])
            .issues
            .iter()
            .any(|issue| issue.kind == IssueKind::InvalidRequest)
    );
    Ok(())
}

#[test]
fn bounded_symbol_scope_and_unsupported_languages_are_explicit() {
    let from = state("before", vec![file("src/retry.rs", RETRY)]);
    let many = (0..513)
        .map(|index| format!("fn f{index}() {{ upload(); }}\n"))
        .collect::<String>();
    let to = state("after", vec![file("src/many.rs", &many)]);
    let report = mapping::map(&function_ref(&from), &from, &to, &["src/many.rs".into()]);
    assert_eq!(report.status, LocationStatus::Unavailable);
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.kind == IssueKind::LimitReached)
    );
    let other = state(
        "other",
        vec![file("src/backoff.py", "def execute():\n    upload()\n")],
    );
    let report = mapping::map(
        &function_ref(&from),
        &from,
        &other,
        &["src/backoff.py".into()],
    );
    assert_eq!(report.status, LocationStatus::Unavailable);
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.kind == IssueKind::UnsupportedLanguage)
    );
    let nonrust = state("config-before", vec![file("before.conf", "alpha\nbeta\n")]);
    let moved = state("config-after", vec![file("after.conf", "alpha\nbeta\n")]);
    let report = mapping::map(
        &original(&nonrust, "before.conf", None),
        &nonrust,
        &moved,
        &["after.conf".into()],
    );
    assert_eq!(report.status, LocationStatus::Mapped);
    assert_eq!(
        report.candidates.first().map(|candidate| candidate.method),
        Some(MappingMethod::WorkingContent)
    );
}

#[test]
fn selected_inner_range_moves_without_duplicating_its_enclosing_symbol() {
    let from = state("before", vec![file("src/retry.rs", RETRY)]);
    let to = state(
        "after",
        vec![file("src/retry.rs", &format!("// header\n{EXECUTE}"))],
    );
    let original = original(
        &from,
        "src/retry.rs",
        Some(TextRange {
            start_line: 2,
            end_line: 3,
        }),
    );
    let report = mapping::map(&original, &from, &to, &["src/retry.rs".into()]);
    assert_eq!(report.status, LocationStatus::Mapped);
    assert_eq!(report.candidates.len(), 1);
    assert_eq!(
        report
            .candidates
            .first()
            .and_then(|candidate| candidate.destination.range.clone()),
        Some(TextRange {
            start_line: 3,
            end_line: 4
        })
    );
    assert_eq!(
        report
            .candidates
            .first()
            .and_then(|candidate| candidate.destination_symbol.as_ref())
            .map(|symbol| symbol.name.as_str()),
        Some("execute")
    );
}

#[test]
fn same_line_symbols_preserve_each_correspondence_identity() -> TestResult {
    let before = state("before", vec![file("a.rs", "fn retry() { upload(); }\n")]);
    let after = state(
        "after",
        vec![file(
            "b.rs",
            "/* 한글 */ fn one() { upload(); } fn two() { upload(); }\n",
        )],
    );
    let reference = original(
        &before,
        "a.rs",
        Some(TextRange {
            start_line: 1,
            end_line: 1,
        }),
    );
    let report = mapping::map(&reference, &before, &after, &["b.rs".into()]);
    assert_eq!(report.status, LocationStatus::Ambiguous);
    assert_eq!(report.candidates.len(), 2);
    let symbols = report
        .candidates
        .iter()
        .map(|candidate| candidate.destination_symbol.as_ref().ok_or("symbol absent"))
        .collect::<Result<Vec<_>, _>>()?;
    let names: Vec<_> = symbols.iter().map(|symbol| symbol.name.as_str()).collect();
    assert_eq!(names, vec!["one", "two"]);
    let first = symbols
        .first()
        .and_then(|symbol| symbol.byte_range.as_ref())
        .ok_or("first byte range absent")?;
    let second = symbols
        .get(1)
        .and_then(|symbol| symbol.byte_range.as_ref())
        .ok_or("second byte range absent")?;
    assert!(first.end <= second.start);
    let content = after
        .files
        .first()
        .and_then(|file| file.working_content.as_deref())
        .ok_or("content absent")?;
    assert_eq!(
        content.get(first.start..first.end),
        Some("fn one() { upload(); }")
    );
    assert_eq!(
        content.get(second.start..second.end),
        Some("fn two() { upload(); }")
    );

    let conditional = state(
        "conditional",
        vec![file(
            "b.rs",
            "#[cfg(unix)] fn next() { upload(); } #[cfg(windows)] fn next() { upload(); }\n",
        )],
    );
    let conditional_report = mapping::map(&reference, &before, &conditional, &["b.rs".into()]);
    assert_eq!(conditional_report.status, LocationStatus::Ambiguous);
    assert_eq!(conditional_report.candidates.len(), 2);
    assert!(conditional_report.candidates.iter().all(|candidate| {
        candidate
            .destination_symbol
            .as_ref()
            .is_some_and(|symbol| symbol.name == "next")
    }));

    // A line reference can also include multiple originals mapping to one target.
    let reverse = mapping::map(
        &original(&after, "b.rs", reference.range.clone()),
        &after,
        &before,
        &["a.rs".into()],
    );
    assert_eq!(reverse.status, LocationStatus::Ambiguous);
    assert_eq!(reverse.candidates.len(), 2);

    // Equal line content remains one exact text correspondence, without naming
    // an arbitrary function as if that line identified a single symbol.
    let same = mapping::map(
        &original(&after, "b.rs", reference.range),
        &after,
        &after,
        &["b.rs".into()],
    );
    assert_eq!(same.status, LocationStatus::Mapped);
    assert_eq!(same.candidates.len(), 1);
    assert!(
        same.candidates
            .iter()
            .all(|candidate| candidate.original_symbol.is_none()
                && candidate.destination_symbol.is_none())
    );
    Ok(())
}

#[test]
fn unstable_original_missing_observation_is_not_confirmed_absence() {
    let mut before = state(
        "before",
        vec![FileState {
            path: "a.rs".into(),
            working_kind: WorkingFileKind::Missing,
            ..FileState::default()
        }],
    );
    let after = state("after", vec![file("b.rs", "fn next() {}\n")]);
    let reference = original(&before, "a.rs", None);
    assert_eq!(
        mapping::map(&reference, &before, &after, &["b.rs".into()]).status,
        LocationStatus::Missing
    );
    before.changed_during_observation = true;
    let report = mapping::map(&reference, &before, &after, &["b.rs".into()]);
    assert_eq!(report.status, LocationStatus::Unavailable);
    assert!(report.candidates.is_empty());
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.kind == IssueKind::ChangedDuringObservation)
    );
}

#[test]
fn mapping_one_function_does_not_cover_other_code_on_the_requested_line() {
    for extra in ["fn other() { download(); }", "const LIMIT: u8 = 3;"] {
        let before = state(
            "before",
            vec![file(
                "a.rs",
                &format!("fn retry() {{ upload(); }} {extra}\n"),
            )],
        );
        let after = state("after", vec![file("b.rs", "fn one() { upload(); }\n")]);
        let reference = original(
            &before,
            "a.rs",
            Some(TextRange {
                start_line: 1,
                end_line: 1,
            }),
        );
        let report = mapping::map(&reference, &before, &after, &["b.rs".into()]);
        assert_eq!(report.status, LocationStatus::Ambiguous);
        assert_eq!(report.candidates.len(), 1);
        assert!(
            report
                .candidates
                .iter()
                .all(|candidate| candidate.correspondence == Correspondence::Partial)
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.kind == IssueKind::PartialCorrespondence)
        );
    }
}
