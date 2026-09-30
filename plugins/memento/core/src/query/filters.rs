use super::*;

pub(super) fn validate(view: &View<'_>, query: &Query) -> Result<(), QueryError> {
    for time in [
        &query.filters.occurred_from,
        &query.filters.occurred_to,
        &query.filters.as_of,
    ]
    .into_iter()
    .flatten()
    {
        if timestamp(time).is_none() {
            return Err(QueryError::InvalidQuery(
                "time filters must be RFC3339 timestamps".into(),
            ));
        }
    }
    if let Some(range) = &query.range
        && (range.start_line == 0 || range.end_line < range.start_line)
    {
        return Err(QueryError::InvalidQuery(
            "invalid one-based text range".into(),
        ));
    }
    let values = serde_json::to_value(&query.filters)
        .map_err(|e| QueryError::InvalidQuery(e.to_string()))?;
    let scope_values =
        serde_json::to_value(&query.scope).map_err(|e| QueryError::InvalidQuery(e.to_string()))?;
    if let Some(object) = values.as_object() {
        for source in &view.sources {
            for name in &source.unsupported_filters {
                let base = name.strip_prefix("exclude_").unwrap_or(name);
                let excluded = format!("exclude_{base}");
                if [base, excluded.as_str()].iter().any(|filter| {
                    object
                        .get(*filter)
                        .or_else(|| scope_values.get(*filter))
                        .is_some_and(|value| {
                            !value.is_null()
                                && value.as_array().is_none_or(|values| !values.is_empty())
                        })
                }) {
                    return Err(QueryError::UnsupportedFilter(name.clone()));
                }
            }
        }
    }
    Ok(())
}

pub(super) fn record(view: &View<'_>, record: &Record, query: &Query) -> bool {
    let scope = &query.scope;
    let f = &query.filters;
    if !scope.work_ids.is_empty() && !record.work_ids.iter().any(|id| scope.work_ids.contains(id)) {
        return false;
    }
    if !scope.session_ids.is_empty()
        && !record
            .session_id
            .as_ref()
            .is_some_and(|id| scope.session_ids.contains(id))
    {
        return false;
    }
    if !scope.worktree_ids.is_empty()
        && !record
            .worktree_id
            .as_ref()
            .is_some_and(|id| scope.worktree_ids.contains(id))
    {
        return false;
    }
    if !accept(&record.kind, &f.record_kinds, &f.exclude_record_kinds)
        || !accept(&record.nature, &f.natures, &f.exclude_natures)
    {
        return false;
    }
    if !typed(
        record.kind == RecordKind::Decision,
        record.decision_status.unwrap_or(DecisionStatus::Unknown),
        &f.decision_statuses,
        &f.exclude_decision_statuses,
    ) || !typed(
        record.kind == RecordKind::Attempt,
        record.attempt_outcome.unwrap_or(AttemptOutcome::Unknown),
        &f.attempt_outcomes,
        &f.exclude_attempt_outcomes,
    ) || !typed(
        record.kind == RecordKind::Verification,
        record
            .verification_outcome
            .unwrap_or(VerificationOutcome::Unknown),
        &f.verification_outcomes,
        &f.exclude_verification_outcomes,
    ) {
        return false;
    }
    let actor = record
        .actor
        .as_ref()
        .and_then(|a| a.name.as_deref())
        .unwrap_or("unknown")
        .to_owned();
    if !accept(&actor, &f.actors, &f.exclude_actors)
        || !many(&record.paths, &f.paths, &f.exclude_paths)
        || !many(&record.commit_shas, &f.commit_shas, &f.exclude_commit_shas)
    {
        return false;
    }
    let works: Vec<_> = view
        .entities()
        .filter_map(|e| match e {
            Entity::Work(w) if record.work_ids.contains(&w.id) => Some(w.status),
            _ => None,
        })
        .collect();
    let works = if works.is_empty() {
        vec![WorkStatus::Unknown]
    } else {
        works
    };
    if !many(&works, &f.work_statuses, &f.exclude_work_statuses) {
        return false;
    }
    let session = view
        .entities()
        .find_map(|e| match e {
            Entity::Session(s) if record.session_id.as_deref() == Some(s.id.as_str()) => {
                Some(s.status)
            }
            _ => None,
        })
        .unwrap_or(SessionStatus::Unknown);
    if !accept(&session, &f.session_statuses, &f.exclude_session_statuses) {
        return false;
    }
    time(record.occurred_at.as_deref(), f)
}

pub(super) fn work(view: &View<'_>, work: &Work, query: &Query) -> bool {
    let scope = &query.scope;
    if !scope.work_ids.is_empty() && !scope.work_ids.contains(&work.id) {
        return false;
    }
    if !accept(
        &work.status,
        &query.filters.work_statuses,
        &query.filters.exclude_work_statuses,
    ) {
        return false;
    }
    if !time(work.observed_at.as_deref(), &query.filters) {
        return false;
    }
    if !scope.session_ids.is_empty() || !scope.worktree_ids.is_empty() {
        let matches = view.entities().any(|e| match e {
            Entity::Session(s) => {
                s.work_ids.contains(&work.id)
                    && (scope.session_ids.is_empty() || scope.session_ids.contains(&s.id))
                    && (scope.worktree_ids.is_empty()
                        || s.worktree_id
                            .as_ref()
                            .is_some_and(|w| scope.worktree_ids.contains(w)))
            }
            _ => false,
        });
        if !matches {
            return false;
        }
    }
    let mut record_query = query.clone();
    record_query.filters.work_statuses.clear();
    record_query.filters.exclude_work_statuses.clear();
    record_query.filters.occurred_from = None;
    record_query.filters.occurred_to = None;
    record_query.filters.as_of = None;
    record_query.filters.time_unknown = None;
    if record_query.filters != Filters::default()
        && !view
            .records()
            .any(|r| r.work_ids.contains(&work.id) && record(view, r, &record_query))
    {
        return false;
    }
    true
}

pub(super) fn scoped(view: &View<'_>, entity: &Entity, query: &Query) -> bool {
    match entity {
        Entity::Record(r) => record(view, r, query),
        Entity::Work(w) => work(view, w, query),
        Entity::Session(s) => {
            (query.scope.work_ids.is_empty()
                || s.work_ids
                    .iter()
                    .any(|id| query.scope.work_ids.contains(id)))
                && (query.scope.session_ids.is_empty() || query.scope.session_ids.contains(&s.id))
                && (query.scope.worktree_ids.is_empty()
                    || s.worktree_id
                        .as_ref()
                        .is_some_and(|id| query.scope.worktree_ids.contains(id)))
        }
        Entity::CodeState(s) => {
            query.scope.worktree_ids.is_empty()
                || s.worktree_id
                    .as_ref()
                    .is_some_and(|id| query.scope.worktree_ids.contains(id))
        }
        Entity::Commit(c) => {
            (query.filters.commit_shas.is_empty() || query.filters.commit_shas.contains(&c.sha))
                && (query.scope.worktree_ids.is_empty()
                    || c.observed_worktree
                        .as_ref()
                        .is_some_and(|id| query.scope.worktree_ids.contains(id)))
        }
        Entity::Source(_) | Entity::Relation(_) => true,
    }
}

fn accept<T: PartialEq>(value: &T, include: &[T], exclude: &[T]) -> bool {
    (include.is_empty() || include.contains(value)) && !exclude.contains(value)
}

fn many<T: PartialEq>(values: &[T], include: &[T], exclude: &[T]) -> bool {
    (include.is_empty() || values.iter().any(|v| include.contains(v)))
        && !values.iter().any(|v| exclude.contains(v))
}

fn typed<T: PartialEq>(applicable: bool, value: T, include: &[T], exclude: &[T]) -> bool {
    if include.is_empty() && exclude.is_empty() {
        return true;
    }
    applicable && accept(&value, include, exclude)
}

pub(super) fn timestamp(value: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|d| d.timestamp_millis())
}

fn time(value: Option<&str>, filters: &Filters) -> bool {
    let value = value.and_then(timestamp);
    if filters
        .time_unknown
        .is_some_and(|want| want == value.is_some())
    {
        return false;
    }
    let bounds = [&filters.occurred_from, &filters.occurred_to, &filters.as_of];
    if bounds.iter().any(|b| b.is_some()) && value.is_none() {
        return false;
    }
    let Some(value) = value else {
        return true;
    };
    if filters
        .occurred_from
        .as_deref()
        .and_then(timestamp)
        .is_some_and(|from| value < from)
    {
        return false;
    }
    if filters
        .occurred_to
        .as_deref()
        .and_then(timestamp)
        .is_some_and(|to| value > to)
    {
        return false;
    }
    filters
        .as_of
        .as_deref()
        .and_then(timestamp)
        .is_none_or(|to| value <= to)
}
