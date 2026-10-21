use memento::{ingest, model::*, query};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn fixture() -> Corpus {
    let mut entries = vec![Entry {
        sequence: 1,
        captured_at: "2026-10-20T00:00:00Z".into(),
        entity: Entity::Source(ingest::source("journal", "p", SourceKind::Journal)),
    }];
    let cases = [
        (
            "dashboard",
            RecordKind::Decision,
            "GC dashboard enhancement approved.",
        ),
        ("old-goal", RecordKind::Request, "Review the GC dashboard."),
        (
            "old-state",
            RecordKind::Status,
            "The dashboard review is complete.",
        ),
        (
            "profiling",
            RecordKind::Finding,
            "GC collection requires enabled profiling.",
        ),
        (
            "logs",
            RecordKind::Finding,
            "GC collection requires verbose logs for per-cycle data.",
        ),
        (
            "recent-state",
            RecordKind::Status,
            "GC collection is active in the current work.",
        ),
    ];
    for (index, (id, kind, body)) in cases.into_iter().enumerate() {
        let mut record = Record::new(id, "p", "journal", kind, body);
        record.work_ids = vec!["w".into()];
        record.association = Association::Explicit;
        if kind == RecordKind::Decision {
            record.decision_status = Some(DecisionStatus::Accepted);
        }
        entries.push(Entry {
            sequence: index as u64 + 2,
            captured_at: format!("2026-10-20T00:00:{:02}Z", index + 1),
            entity: Entity::Record(record),
        });
    }
    Corpus {
        entries,
        compaction: CompactionState::default(),
    }
}

fn brief() -> Query {
    let mut query = Query::new(Operation::Brief, "p");
    query.scope.work_ids = vec!["w".into()];
    query.purpose = Some(BriefPurpose::Resume);
    query.limit = Some(4);
    query.budget_bytes = Some(65536);
    query
}

fn ids(reply: &query::QueryResponse) -> Vec<&str> {
    reply
        .items
        .iter()
        .filter_map(|item| match &item.entity {
            Entity::Record(record) => Some(record.id.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn focus_reaches_gc_conditions_and_recent_state_before_old_approval() -> TestResult {
    let corpus = fixture();
    let baseline = query::execute(&corpus, &brief())?;
    assert!(!ids(&baseline).contains(&"profiling"));
    assert!(!ids(&baseline).contains(&"logs"));
    let mut focused = brief();
    focused.focus = Some("GC collection profiling logs".into());
    let reply = query::execute(&corpus, &focused)?;
    assert!(ids(&reply).contains(&"profiling"));
    assert!(ids(&reply).contains(&"logs"));
    assert!(ids(&reply).contains(&"recent-state"));
    assert!(!ids(&reply).contains(&"dashboard"));
    Ok(())
}

#[test]
fn unfocused_brief_orders_latest_state_before_older_state() -> TestResult {
    let reply = query::execute(&fixture(), &brief())?;
    let statuses: Vec<_> = reply
        .items
        .iter()
        .filter_map(|item| match &item.entity {
            Entity::Record(record) if record.kind == RecordKind::Status => Some(record.id.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(statuses, vec!["recent-state", "old-state"]);
    Ok(())
}

#[test]
fn focus_preserves_explicit_query_and_work_scope() -> TestResult {
    let mut corpus = fixture();
    let mut unrelated = Record::new(
        "other-work",
        "p",
        "journal",
        RecordKind::Finding,
        "GC collection profiling logs",
    );
    unrelated.work_ids = vec!["other".into()];
    corpus.entries.push(Entry {
        sequence: 20,
        captured_at: "2026-10-21T00:00:00Z".into(),
        entity: Entity::Record(unrelated),
    });
    let mut focused = brief();
    focused.focus = Some("GC collection profiling logs".into());
    focused.query = Some(TextQuery {
        text: "dashboard".into(),
        mode: SearchMode::Literal,
    });
    let reply = query::execute(&corpus, &focused)?;
    assert!(reply.items.iter().all(|item| match &item.entity {
        Entity::Record(record) => record.body.contains("dashboard"),
        _ => false,
    }));
    assert!(!ids(&reply).contains(&"other-work"));
    Ok(())
}

#[test]
fn focus_is_part_of_cursor_identity_and_rejects_invalid_use() -> TestResult {
    let corpus = fixture();
    let mut focused = brief();
    focused.focus = Some("GC collection".into());
    focused.limit = Some(1);
    let first = query::execute(&corpus, &focused)?;
    focused.cursor = first.next_cursor.clone();
    assert!(focused.cursor.is_some());
    let next = query::execute(&corpus, &focused)?;
    assert_ne!(ids(&first), ids(&next));
    focused.focus = Some("dashboard".into());
    assert!(matches!(
        query::execute(&corpus, &focused),
        Err(query::QueryError::StaleCursor)
    ));
    let mut invalid = brief();
    invalid.focus = Some("x".repeat(4097));
    assert!(matches!(
        query::execute(&corpus, &invalid),
        Err(query::QueryError::InvalidQuery(_))
    ));
    invalid.focus = Some("GC".into());
    invalid.operation = Operation::Search;
    assert!(matches!(
        query::execute(&corpus, &invalid),
        Err(query::QueryError::InvalidQuery(_))
    ));
    Ok(())
}

#[test]
fn relevance_does_not_rank_withheld_derived_text() -> TestResult {
    let mut corpus = fixture();
    let mut hidden = Record::new(
        "hidden",
        "p",
        "journal",
        RecordKind::Finding,
        "GC collection profiling logs",
    );
    hidden.derived = true;
    hidden.work_ids = vec!["w".into()];
    hidden.evidence = serde_json::from_value(json!([{
        "source_id": "restricted", "record_id": "secret", "revision": "v1",
        "locator": "private", "availability": "available"
    }]))?;
    corpus.entries.push(Entry {
        sequence: 30,
        captured_at: "2026-10-21T00:00:00Z".into(),
        entity: Entity::Record(hidden),
    });
    let mut focused = brief();
    focused.focus = Some("GC collection profiling logs".into());
    let reply = query::execute(&corpus, &focused)?;
    assert!(!ids(&reply).contains(&"hidden"));
    assert!(ids(&reply).contains(&"profiling"));
    Ok(())
}
