use memento::model::*;
use memento::query::{QueryItem, execute};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const EVIDENCE_SOURCE: &str = "private-evidence-source";

fn source(id: &str) -> Source {
    Source {
        id: id.into(),
        project_id: "project".into(),
        kind: SourceKind::Journal,
        name: id.into(),
        location: None,
        content_revision: None,
        authorized: true,
        available: true,
        record_kinds: Vec::new(),
        unsupported_filters: Vec::new(),
        last_captured_at: None,
        last_event_id: None,
        gaps: Vec::new(),
        result_only: false,
        import_completeness: ImportCompleteness::FullSnapshot,
    }
}

fn add(corpus: &mut Corpus, entity: Entity) {
    let sequence = corpus
        .entries
        .last()
        .map_or(1, |entry| entry.sequence.saturating_add(1));
    corpus.entries.push(Entry {
        sequence,
        captured_at: "2026-10-21T00:00:00Z".into(),
        entity,
    });
}

fn corpus(evidence_source: Option<Source>) -> Corpus {
    let mut corpus = Corpus::default();
    add(&mut corpus, Entity::Source(source("journal")));
    if let Some(source) = evidence_source {
        add(&mut corpus, Entity::Source(source));
    }
    let mut origin = Record::new(
        "private-origin",
        "project",
        EVIDENCE_SOURCE,
        RecordKind::ToolResult,
        "private original body",
    );
    origin.revision = "origin-v1".into();
    origin.representation = Representation::Evidence;
    add(&mut corpus, Entity::Record(origin));
    let mut claim = Record::new(
        "claim",
        "project",
        "journal",
        RecordKind::Finding,
        "retained summary",
    );
    claim.representation = Representation::Claim;
    claim.derived = true;
    claim.fidelity = Fidelity::SummaryOnly;
    claim.context_id = Some("diagnostic-test".into());
    claim.evidence.push(Evidence {
        source_id: EVIDENCE_SOURCE.into(),
        record_id: Some("private-origin".into()),
        revision: "origin-v1".into(),
        locator: "private-locator".into(),
        availability: Availability::Available,
        range: Some(TextRange {
            start_line: 1,
            end_line: 1,
        }),
        purpose: EvidencePurpose::Origin,
        span: None,
    });
    add(&mut corpus, Entity::Record(claim));
    corpus
}

fn response(
    corpus: &Corpus,
    source_ids: &[&str],
) -> Result<memento::query::QueryResponse, Box<dyn std::error::Error>> {
    let mut query = Query::new(Operation::Read, "project");
    query.scope.source_ids = source_ids.iter().map(|source| (*source).into()).collect();
    query.target = Some(Target::Record { id: "claim".into() });
    Ok(execute(corpus, &query)?)
}

fn read(corpus: &Corpus, source_ids: &[&str]) -> Result<QueryItem, Box<dyn std::error::Error>> {
    response(corpus, source_ids)?
        .items
        .into_iter()
        .find(|item| item.entity.id() == "claim")
        .ok_or_else(|| "claim item missing".into())
}

fn record(item: &QueryItem) -> Result<&Record, Box<dyn std::error::Error>> {
    match &item.entity {
        Entity::Record(record) => Ok(record),
        _ => Err("expected a record".into()),
    }
}

fn has_reason(item: &QueryItem, reason: &str) -> bool {
    item.warnings
        .iter()
        .any(|warning| warning.starts_with(&format!("{reason}:")))
}

#[test]
fn out_of_scope_evidence_explains_required_scope_without_expanding_it() -> TestResult {
    let corpus = corpus(Some(source(EVIDENCE_SOURCE)));
    let scoped = read(&corpus, &["journal"])?;
    assert!(record(&scoped)?.body.is_empty());
    assert!(record(&scoped)?.title.is_empty());
    assert!(has_reason(&scoped, "evidence_out_of_scope"));
    assert!(!has_reason(&scoped, "evidence_deleted"));
    assert!(!has_reason(&scoped, "evidence_inaccessible"));
    assert!(scoped.warnings.iter().any(|warning| {
        warning.contains(EVIDENCE_SOURCE)
            && warning.contains("scope.source_ids")
            && warning.contains("retry the same query")
    }));
    assert!(
        record(&scoped)?
            .evidence
            .iter()
            .all(|reference| reference.source_id == "journal")
    );
    assert!(!serde_json::to_string(&scoped)?.contains("private original body"));

    let expanded = read(&corpus, &["journal", EVIDENCE_SOURCE])?;
    assert_eq!(record(&expanded)?.body, "retained summary");
    assert!(
        record(&expanded)?
            .evidence
            .iter()
            .any(|reference| reference.source_id == EVIDENCE_SOURCE)
    );
    assert!(!has_reason(&expanded, "evidence_out_of_scope"));
    Ok(())
}

#[test]
fn deleted_accessible_evidence_reports_deletion() -> TestResult {
    let mut corpus = corpus(Some(source(EVIDENCE_SOURCE)));
    let origin = corpus
        .entries
        .iter_mut()
        .find_map(|entry| match &mut entry.entity {
            Entity::Record(record) if record.id == "private-origin" => Some(record),
            _ => None,
        })
        .ok_or("origin missing")?;
    origin.availability = Availability::Deleted;
    origin.body.clear();
    let item = read(&corpus, &[])?;
    assert!(record(&item)?.body.is_empty());
    assert!(has_reason(&item, "evidence_deleted"));
    assert!(!has_reason(&item, "evidence_out_of_scope"));
    assert!(!has_reason(&item, "evidence_inaccessible"));
    Ok(())
}

#[test]
fn unauthorized_and_absent_sources_have_the_same_private_diagnostic() -> TestResult {
    let mut revoked = source(EVIDENCE_SOURCE);
    revoked.authorized = false;
    let inaccessible_corpus = corpus(Some(revoked));
    let absent_corpus = corpus(None);
    let inaccessible = read(&inaccessible_corpus, &["journal"])?;
    let absent = read(&absent_corpus, &["journal"])?;
    assert_eq!(inaccessible.warnings, absent.warnings);
    for (corpus, item) in [
        (&inaccessible_corpus, inaccessible),
        (&absent_corpus, absent),
    ] {
        assert!(record(&item)?.body.is_empty());
        assert!(has_reason(&item, "evidence_inaccessible"));
        assert!(!has_reason(&item, "evidence_out_of_scope"));
        assert!(!has_reason(&item, "evidence_deleted"));
        let rendered = serde_json::to_string(&response(corpus, &["journal"])?)?;
        for private in [
            EVIDENCE_SOURCE,
            "private-origin",
            "private-locator",
            "private original body",
        ] {
            assert!(!rendered.contains(private));
        }
    }
    Ok(())
}

#[test]
fn another_projects_source_does_not_enable_scope_guidance() -> TestResult {
    let mut other = source(EVIDENCE_SOURCE);
    other.project_id = "another-project".into();
    let item = read(&corpus(Some(other)), &["journal"])?;
    assert!(has_reason(&item, "evidence_inaccessible"));
    assert!(!has_reason(&item, "evidence_out_of_scope"));
    assert!(!serde_json::to_string(&item)?.contains(EVIDENCE_SOURCE));
    Ok(())
}

#[test]
fn missing_record_keeps_the_existing_stale_summary_policy() -> TestResult {
    let mut corpus = corpus(Some(source(EVIDENCE_SOURCE)));
    corpus.entries.retain(
        |entry| !matches!(&entry.entity, Entity::Record(record) if record.id == "private-origin"),
    );
    let item = read(&corpus, &[])?;
    assert_eq!(record(&item)?.body, "retained summary");
    assert!(has_reason(&item, "stale_summary"));
    assert!(!has_reason(&item, "evidence_deleted"));
    assert!(!has_reason(&item, "evidence_out_of_scope"));
    Ok(())
}

#[test]
fn stale_revision_probe_uses_the_exact_project_and_source_record() -> TestResult {
    let mut corpus = corpus(Some(source(EVIDENCE_SOURCE)));
    let mut collision = Record::new(
        "private-origin",
        "project",
        "journal",
        RecordKind::ToolResult,
        "unrelated source record",
    );
    collision.revision = "unrelated-v2".into();
    add(&mut corpus, Entity::Record(collision));
    let origin = corpus
        .entries
        .iter()
        .find_map(|entry| match &entry.entity {
            Entity::Record(record)
                if record.id == "private-origin" && record.source_id == EVIDENCE_SOURCE =>
            {
                Some(record.clone())
            }
            _ => None,
        })
        .ok_or("origin missing")?;
    let mut another_project = origin.clone();
    another_project.project_id = "another-project".into();
    another_project.revision = "another-project-v2".into();
    add(&mut corpus, Entity::Record(another_project));
    assert!(!has_reason(&read(&corpus, &[])?, "stale_summary"));

    let mut changed = origin;
    changed.revision = "origin-v2".into();
    add(&mut corpus, Entity::Record(changed));
    let item = read(&corpus, &[])?;
    assert!(has_reason(&item, "stale_summary"));
    assert_eq!(record(&item)?.body, "retained summary");
    Ok(())
}

#[test]
fn relation_dense_brief_preserves_current_evidence_and_page_contents() -> TestResult {
    let mut corpus = corpus(Some(source(EVIDENCE_SOURCE)));
    let template = record(&read(&corpus, &[])?)?.clone();
    for index in 0..12 {
        let mut claim = template.clone();
        claim.id = format!("dense-{index}");
        add(&mut corpus, Entity::Record(claim));
    }
    for from in 0..12 {
        for to in 0..12 {
            if from == to {
                continue;
            }
            add(
                &mut corpus,
                Entity::Relation(Relation {
                    id: format!("dense-edge-{from}-{to}"),
                    project_id: "project".into(),
                    source_id: "journal".into(),
                    from: Target::Record {
                        id: format!("dense-{from}"),
                    },
                    to: Target::Record {
                        id: format!("dense-{to}"),
                    },
                    kind: RelationKind::RelatedTo,
                    nature: Nature::Observed,
                    evidence: Vec::new(),
                    applies_to: Vec::new(),
                }),
            );
        }
    }
    let mut query = Query::new(Operation::Brief, "project");
    query.limit = Some(3);
    let response = execute(&corpus, &query)?;
    assert_eq!(response.items.len(), 3);
    for item in &response.items {
        assert_eq!(record(item)?.body, "retained summary");
        assert!(!has_reason(item, "stale_summary"));
        assert!(!has_reason(item, "evidence_out_of_scope"));
    }
    assert_eq!(
        response
            .brief
            .as_ref()
            .ok_or("brief missing")?
            .sections
            .iter()
            .map(|section| section.claims.len())
            .sum::<usize>(),
        3
    );
    Ok(())
}

fn status_fixture(status_source: &str) -> Result<Corpus, Box<dyn std::error::Error>> {
    let mut corpus = corpus(Some(source(EVIDENCE_SOURCE)));
    let claim = corpus
        .entries
        .iter_mut()
        .find_map(|entry| match &mut entry.entity {
            Entity::Record(record) if record.id == "claim" => Some(record),
            _ => None,
        })
        .ok_or("claim missing")?;
    claim.work_ids = vec!["work".into()];
    claim.session_id = Some("session".into());
    add(
        &mut corpus,
        Entity::Work(Work {
            id: "work".into(),
            project_id: "project".into(),
            source_id: status_source.into(),
            title: "active work".into(),
            goal: "test status filters".into(),
            status: WorkStatus::Active,
            observed_at: None,
            evidence: Vec::new(),
            completion_conditions: Vec::new(),
        }),
    );
    add(
        &mut corpus,
        Entity::Session(Session {
            id: "session".into(),
            project_id: "project".into(),
            source_id: status_source.into(),
            work_ids: vec!["work".into()],
            status: SessionStatus::Active,
            started_at: None,
            ended_at: None,
            worktree_id: None,
            parent_id: None,
            working_directory: None,
        }),
    );
    Ok(corpus)
}

fn status_query() -> Query {
    let mut query = Query::new(Operation::Read, "project");
    query.target = Some(Target::Record { id: "claim".into() });
    query
}

#[test]
fn active_status_filters_preserve_known_include_and_exclude_behavior() -> TestResult {
    let corpus = status_fixture("journal")?;
    let mut query = status_query();
    assert_eq!(execute(&corpus, &query)?.items.len(), 1);
    query.filters.work_statuses = vec![WorkStatus::Active];
    query.filters.session_statuses = vec![SessionStatus::Active];
    assert_eq!(execute(&corpus, &query)?.items.len(), 1);

    query.filters.exclude_work_statuses = vec![WorkStatus::Active];
    assert!(execute(&corpus, &query)?.items.is_empty());
    query.filters.exclude_work_statuses = vec![WorkStatus::Completed];
    assert_eq!(execute(&corpus, &query)?.items.len(), 1);
    query.filters.work_statuses = vec![WorkStatus::Completed];
    assert!(execute(&corpus, &query)?.items.is_empty());

    query.filters.work_statuses = vec![WorkStatus::Active];
    query.filters.session_statuses = vec![SessionStatus::Ended];
    assert!(execute(&corpus, &query)?.items.is_empty());
    query.filters.session_statuses = vec![SessionStatus::Active];
    query.filters.exclude_session_statuses = vec![SessionStatus::Active];
    assert!(execute(&corpus, &query)?.items.is_empty());
    query.filters.exclude_session_statuses = vec![SessionStatus::Ended];
    assert_eq!(execute(&corpus, &query)?.items.len(), 1);
    Ok(())
}

#[test]
fn active_status_filters_preserve_unknown_fallback() -> TestResult {
    let corpus = status_fixture("unregistered-status-source")?;
    let mut query = status_query();
    query.filters.work_statuses = vec![WorkStatus::Unknown];
    query.filters.session_statuses = vec![SessionStatus::Unknown];
    assert_eq!(execute(&corpus, &query)?.items.len(), 1);
    query.filters.exclude_work_statuses = vec![WorkStatus::Unknown];
    assert!(execute(&corpus, &query)?.items.is_empty());
    query.filters.exclude_work_statuses.clear();
    query.filters.exclude_session_statuses = vec![SessionStatus::Unknown];
    assert!(execute(&corpus, &query)?.items.is_empty());
    query.filters.exclude_session_statuses.clear();
    query.filters.work_statuses = vec![WorkStatus::Active];
    assert!(execute(&corpus, &query)?.items.is_empty());
    Ok(())
}

#[test]
fn unauthorized_status_entities_do_not_affect_active_status_filters() -> TestResult {
    let mut corpus = status_fixture("hidden-status-source")?;
    let mut hidden = source("hidden-status-source");
    hidden.authorized = false;
    add(&mut corpus, Entity::Source(hidden));
    let mut query = status_query();
    query.filters.work_statuses = vec![WorkStatus::Unknown];
    query.filters.session_statuses = vec![SessionStatus::Unknown];
    let response = execute(&corpus, &query)?;
    assert_eq!(response.items.len(), 1);
    assert!(!serde_json::to_string(&response)?.contains("hidden-status-source"));
    query.filters.work_statuses = vec![WorkStatus::Active];
    query.filters.session_statuses = vec![SessionStatus::Active];
    assert!(execute(&corpus, &query)?.items.is_empty());
    Ok(())
}
