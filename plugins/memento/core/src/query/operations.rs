use super::*;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use view::{Checkpoint, target_matches};

pub(super) fn resolve_location(
    view: &View<'_>,
    query: &Query,
) -> Result<(Query, Option<LocationResolution>), QueryError> {
    let Some(Target::Code {
        state_id,
        path,
        range,
    }) = &query.target
    else {
        return Ok((query.clone(), None));
    };
    if let Some(range) = range
        && (range.start_line == 0 || range.end_line < range.start_line)
    {
        return Err(QueryError::InvalidQuery(
            "invalid one-based code range".into(),
        ));
    }
    let states: Vec<_> = view
        .entities()
        .filter_map(|entity| match entity {
            Entity::CodeState(state)
                if filters::scoped(view, entity, query)
                    && (state_id.is_empty() || &state.id == state_id) =>
            {
                Some(state)
            }
            _ => None,
        })
        .collect();
    let path_matches: Vec<_> = states
        .iter()
        .filter(|state| state.files.iter().any(|file| &file.path == path))
        .collect();
    let matching: Vec<_> = path_matches
        .iter()
        .copied()
        .filter(|state| {
            state.files.iter().any(|file| {
                &file.path == path
                    && match file.working_kind {
                        WorkingFileKind::File => true,
                        WorkingFileKind::Unknown => {
                            file.working_content.is_some()
                                || file.working_hash.is_some()
                                || file.head_hash.is_some()
                                || file.index_hash.is_some()
                        }
                        WorkingFileKind::Missing
                        | WorkingFileKind::Ignored
                        | WorkingFileKind::Symlink
                        | WorkingFileKind::Directory => false,
                    }
            })
        })
        .collect();
    let status = if matching.len() > 1 {
        LocationStatus::Ambiguous
    } else if matching.len() == 1 {
        LocationStatus::Exact
    } else if states.is_empty()
        || path_matches.iter().any(|state| {
            state
                .files
                .iter()
                .any(|file| &file.path == path && file.working_kind != WorkingFileKind::Missing)
        })
    {
        LocationStatus::Unavailable
    } else {
        LocationStatus::Missing
    };
    let candidates: Vec<_> = matching
        .iter()
        .take(100)
        .map(|state| Target::Code {
            state_id: state.id.clone(),
            path: path.clone(),
            range: range.clone(),
        })
        .collect();
    let mut notes = vec![
        "references identify retained historical observations; current checkout is not inferred"
            .into(),
        "location.status resolves the original retained address; use explicit code_mapping to compare a destination state and selected paths".into(),
    ];
    if status == LocationStatus::Ambiguous {
        notes.push("select a state, source, or worktree before reading or tracing; no newest candidate is chosen".into());
    }
    if status == LocationStatus::Missing {
        notes.push("path absent in this working observation; historical HEAD/index hashes do not establish a present working file".into());
    }
    if status == LocationStatus::Unavailable {
        notes.push("code was not retained as an accessible regular-file observation; ignored paths, symlinks, and directories are not read through".into());
    }
    if matching.len() > 100 {
        notes.push("candidate list limited to 100; narrow source/worktree scope".into());
    }
    for state in matching.iter().take(100) {
        notes.push(format!(
            "candidate source {}, state {}, observed {}",
            state.source_id, state.id, state.observed_at
        ));
        if state.changed_during_observation {
            notes.push("changed_during_observation: retained file observation was not a single immutable workspace state".into());
        }
    }
    let mut resolved = query.clone();
    if status == LocationStatus::Exact {
        resolved.target = candidates.first().cloned();
    }
    Ok((
        resolved,
        Some(LocationResolution {
            status,
            candidates,
            notes,
            mapping: None,
            mapping_status: None,
        }),
    ))
}

pub(super) fn select(
    view: &View<'_>,
    query: &Query,
    response: &mut QueryResponse,
) -> Result<Vec<QueryItem>, QueryError> {
    match query.operation {
        Operation::Sources => Ok(view
            .sources
            .iter()
            .map(|s| view.item(&Entity::Source((*s).clone())))
            .collect()),
        Operation::ListWork => Ok(view
            .entities()
            .filter(|e| matches!(e, Entity::Work(_)))
            .filter(|e| filters::scoped(view, e, query))
            .filter_map(|e| matched(view.item(e), query))
            .collect()),
        Operation::Search | Operation::Timeline | Operation::Brief => {
            let mut rows: Vec<_> = view
                .records()
                .filter(|r| filters::record(view, r, query))
                .filter(|r| {
                    query.operation != Operation::Brief
                        || r.representation != Representation::Evidence
                })
                .filter(|r| brief_target(r, query.target.as_ref()))
                .filter_map(|r| matched(view.item(&Entity::Record(r.clone())), query))
                .collect();
            if query.operation == Operation::Timeline {
                for row in &mut rows {
                    row.warnings.push("session source order is retained; cross-session ordering does not imply causality".into());
                }
            }
            if query.operation == Operation::Brief {
                if query
                    .target
                    .as_ref()
                    .is_some_and(|t| !matches!(t, Target::Work { .. } | Target::Session { .. }))
                {
                    let mut tracing = query.clone();
                    tracing.operation = Operation::Trace;
                    tracing.direction = Some(Direction::Both);
                    let mut tracing_response = view.response(&tracing);
                    rows = trace(view, &tracing, &mut tracing_response)?
                        .into_iter()
                        .filter(|item| {
                            matches!(&item.entity, Entity::Record(record)
                                if record.representation != Representation::Evidence)
                        })
                        .collect();
                    response.omitted.extend(tracing_response.omitted);
                }
                let target_work = match &query.target {
                    Some(Target::Work { id }) => Some(id),
                    _ => None,
                };
                if query.target.is_none() || target_work.is_some() {
                    rows.extend(view.entities().filter_map(|e| match e {
                        Entity::Work(w)
                            if filters::work(view, w, query)
                                && target_work.is_none_or(|id| id == &w.id) =>
                        {
                            Some(view.item(e))
                        }
                        _ => None,
                    }));
                }
                // The actual source text is the claim. The pure engine does not invent a rationale.
                rows.sort_by_key(|i| match &i.entity {
                    Entity::Record(r) => priority(r.kind),
                    _ => 10,
                });
            }
            Ok(rows)
        }
        Operation::Read => read(view, query),
        Operation::Trace => trace(view, query, response),
        Operation::Compare => compare(view, query, response),
    }
}

pub(super) fn brief_target(record: &Record, target: Option<&Target>) -> bool {
    match target {
        None => true,
        Some(Target::Work { id }) => record.work_ids.contains(id),
        Some(Target::Session { id }) => record.session_id.as_ref() == Some(id),
        Some(Target::Record { id }) => &record.id == id,
        Some(Target::Code { state_id, path, .. }) => record
            .code_refs
            .iter()
            .any(|r| &r.state_id == state_id && &r.path == path),
        Some(Target::Commit { commit_sha, .. }) => record.commit_shas.contains(commit_sha),
        Some(Target::Artifact {
            record_id,
            revision,
            ..
        }) => &record.id == record_id && &record.revision == revision,
    }
}

fn read(view: &View<'_>, query: &Query) -> Result<Vec<QueryItem>, QueryError> {
    let target = query
        .target
        .as_ref()
        .ok_or_else(|| QueryError::InvalidQuery("read requires target".into()))?;
    let mut rows: Vec<_> = if matches!(target, Target::Artifact { .. }) {
        let mut revisions = BTreeMap::new();
        for entry in &view.corpus.entries {
            if entry.sequence <= view.snapshot
                && target_matches(target, &entry.entity)
                && view.visible(&entry.entity)
                && filters::scoped(view, &entry.entity, query)
            {
                revisions.insert(entry.entity.key(), view.item(&entry.entity));
            }
        }
        revisions.into_values().collect()
    } else {
        view.entities()
            .filter(|e| target_matches(target, e) && filters::scoped(view, e, query))
            .map(|e| view.item(e))
            .collect()
    };
    if rows.len() > 1 {
        return Err(QueryError::InvalidScope(
            "target is ambiguous; select a source".into(),
        ));
    }
    if let Target::Code { path, range, .. } = target {
        for item in &mut rows {
            if let Entity::CodeState(state) = &mut item.entity {
                state.files.retain(|file| &file.path == path);
                if let Some(text) = state
                    .files
                    .first_mut()
                    .and_then(|file| file.working_content.as_mut())
                {
                    item.rendered_revision = Some(crate::security::hash(text.as_bytes()));
                    if let Some(range) = query.range.as_ref().or(range.as_ref()) {
                        let context = query.context_lines.unwrap_or(0).min(100);
                        let start = range.start_line.saturating_sub(1).saturating_sub(context);
                        let end = range.end_line.saturating_add(context);
                        *text = text
                            .lines()
                            .skip(start)
                            .take(end.saturating_sub(start))
                            .collect::<Vec<_>>()
                            .join("\n");
                        item.warnings.push(format!(
                            "code window at retained state: lines {}..{}",
                            start.saturating_add(1),
                            end
                        ));
                    }
                } else {
                    item.warnings.push(
                        "retained code content unavailable; hashes do not reconstruct text".into(),
                    );
                }
            }
        }
    }
    if let Some(QueryItem {
        entity: Entity::Record(record),
        warnings,
        ..
    }) = rows.first_mut()
    {
        let target_range = match target {
            Target::Artifact { range, .. } => range.as_ref(),
            _ => None,
        };
        if let Some(range) = query.range.as_ref().or(target_range) {
            if range.start_line == 0 || range.end_line < range.start_line {
                return Err(QueryError::InvalidQuery(
                    "invalid one-based text range".into(),
                ));
            }
            let context = query.context_lines.unwrap_or(0).min(100);
            let start = range.start_line.saturating_sub(1).saturating_sub(context);
            let end = range.end_line.saturating_add(context);
            record.body = record
                .body
                .lines()
                .skip(start)
                .take(end.saturating_sub(start))
                .collect::<Vec<_>>()
                .join("\n");
            warnings.push(format!(
                "text window at retained revision: lines {}..{}",
                start.saturating_add(1),
                end
            ));
        } else if let Some(context) = query.context_lines.filter(|n| *n > 0) {
            let record_id = record.id.clone();
            let session = record.session_id.clone();
            let mut neighbors: Vec<_> = view
                .records()
                .filter(|r| r.session_id == session && filters::record(view, r, query))
                .collect();
            neighbors.sort_by_key(|r| r.source_order);
            if let Some(at) = neighbors.iter().position(|r| r.id == record_id) {
                let start = at.saturating_sub(context.min(100));
                let count = context.min(100).saturating_mul(2).saturating_add(1);
                rows = neighbors
                    .into_iter()
                    .skip(start)
                    .take(count)
                    .map(|r| view.item(&Entity::Record(r.clone())))
                    .collect();
            }
        }
    }
    Ok(rows)
}

fn matched(mut item: QueryItem, query: &Query) -> Option<QueryItem> {
    let Some(text_query) = query.query.as_ref().filter(|q| !q.text.is_empty()) else {
        return Some(item);
    };
    let fields: Vec<(&str, &str)> = match &item.entity {
        Entity::Record(r) => {
            let mut fields = vec![("title", r.title.as_str()), ("body", r.body.as_str())];
            if let Some(e) = &r.execution {
                fields.push(("tool_input", e.command.as_str()));
            }
            fields
        }
        Entity::Work(w) => vec![("title", &w.title), ("goal", &w.goal)],
        _ => return None,
    };
    let terms: Vec<String> = match text_query.mode {
        SearchMode::Literal => vec![text_query.text.clone()],
        SearchMode::Tokens => text_query
            .text
            .split_whitespace()
            .map(str::to_lowercase)
            .collect(),
        SearchMode::Semantic => return None,
    };
    let field_text: Vec<_> = fields
        .iter()
        .map(|(_, s)| {
            if text_query.mode == SearchMode::Tokens {
                s.to_lowercase()
            } else {
                (*s).to_owned()
            }
        })
        .collect();
    if !terms
        .iter()
        .all(|term| field_text.iter().any(|s| s.contains(term)))
    {
        return None;
    }
    for ((name, original), text) in fields.iter().zip(&field_text) {
        for term in &terms {
            let Some(position) = text.find(term) else {
                continue;
            };
            let end = position.saturating_add(term.len());
            let (position, end) = if text_query.mode == SearchMode::Tokens {
                original_span(original, position, end)
            } else {
                (position, end)
            };
            if !item.match_locations.iter().any(|field| field == name) {
                item.match_locations.push((*name).to_owned());
            }
            let start_line = original.get(..position).map_or(1, |prefix| {
                prefix
                    .bytes()
                    .filter(|byte| *byte == b'\n')
                    .count()
                    .saturating_add(1)
            });
            let final_character = original
                .get(..end)
                .and_then(|prefix| prefix.char_indices().last().map(|(offset, _)| offset))
                .unwrap_or(position);
            let end_line = original
                .get(..final_character)
                .map_or(start_line, |prefix| {
                    prefix
                        .bytes()
                        .filter(|byte| *byte == b'\n')
                        .count()
                        .saturating_add(1)
                });
            item.match_ranges.push(MatchRange {
                field: (*name).into(),
                range: TextRange {
                    start_line,
                    end_line,
                },
            });
            if item.excerpt.is_none() {
                let start = super::boundary(original, position.saturating_sub(120));
                let end = super::boundary(original, end.saturating_add(500));
                item.excerpt = original.get(start..end).map(str::to_owned);
                if *name == "body" {
                    item.warnings.push(format!(
                        "match at lines {start_line}..{end_line}; use read with the returned match range"
                    ));
                }
            }
        }
    }
    Some(item)
}

fn original_span(original: &str, normalized_start: usize, normalized_end: usize) -> (usize, usize) {
    let mut normalized_offset = 0usize;
    let mut start = None;
    let mut end = None;
    for (offset, character) in original.char_indices() {
        let length: usize = character.to_lowercase().map(char::len_utf8).sum();
        let next = normalized_offset.saturating_add(length);
        if start.is_none() && normalized_start < next {
            start = Some(offset);
        }
        if normalized_end <= next {
            end = Some(offset.saturating_add(character.len_utf8()));
            break;
        }
        normalized_offset = next;
    }
    (
        start.unwrap_or(original.len()),
        end.unwrap_or(original.len()),
    )
}

pub(super) fn sort(items: &mut [QueryItem], query: &Query) {
    if query.operation == Operation::Brief {
        return;
    }
    if query.operation == Operation::Read {
        return;
    }
    if query.operation == Operation::Timeline {
        items.sort_by_key(|item| match &item.entity {
            Entity::Record(r) => (
                r.session_id.clone(),
                r.source_order,
                r.occurred_at.clone(),
                r.id.clone(),
            ),
            e => (None, None, None, e.id().to_owned()),
        });
        return;
    }
    let sort = query.sort.unwrap_or(Sort::Oldest);
    items.sort_by(|a, b| {
        let order = time_of(&a.entity)
            .cmp(&time_of(&b.entity))
            .then_with(|| a.entity.key().cmp(&b.entity.key()));
        match sort {
            Sort::Oldest => order,
            Sort::Newest => order.reverse(),
            Sort::Relevance => b
                .match_locations
                .len()
                .cmp(&a.match_locations.len())
                .then(order),
        }
    });
}

fn time_of(entity: &Entity) -> Option<i64> {
    let time = match entity {
        Entity::Record(r) => r.occurred_at.as_deref(),
        Entity::Work(w) => w.observed_at.as_deref(),
        Entity::Session(s) => s.started_at.as_deref(),
        Entity::Commit(c) => c.occurred_at.as_deref(),
        _ => None,
    };
    time.and_then(filters::timestamp)
}

fn trace(
    view: &View<'_>,
    query: &Query,
    response: &mut QueryResponse,
) -> Result<Vec<QueryItem>, QueryError> {
    let start = query
        .target
        .as_ref()
        .ok_or_else(|| QueryError::InvalidQuery("trace requires target".into()))?;
    let depth = query.max_depth.unwrap_or(3);
    if depth > 32 {
        return Err(QueryError::InvalidQuery(
            "trace max_depth cannot exceed 32".into(),
        ));
    }
    let direction = query.direction.unwrap_or(Direction::Both);
    let mut pending = VecDeque::from([(start.clone(), 0usize)]);
    let mut visited = BTreeSet::new();
    let mut rows = BTreeMap::new();
    let mut edges = BTreeMap::new();
    while let Some((target, level)) = pending.pop_front() {
        if !visited.insert(view::digest(&target)?) {
            continue;
        }
        if visited.len() > 1000 {
            response
                .omitted
                .push("trace node bound reached; restart from a returned target".into());
            break;
        }
        for entity in view
            .entities()
            .filter(|e| target_matches(&target, e) && filters::scoped(view, e, query))
        {
            rows.insert(entity.key(), view.item(entity));
        }
        if level >= depth {
            continue;
        }
        for relation in view
            .relations()
            .filter(|r| query.relations.is_empty() || query.relations.contains(&r.kind))
        {
            let symmetric = matches!(
                relation.kind,
                RelationKind::RelatedTo | RelationKind::Contradicts
            );
            let next = if target_eq(&target, &relation.from)
                && (direction != Direction::Incoming || symmetric)
            {
                Some(&relation.to)
            } else if target_eq(&target, &relation.to)
                && (direction != Direction::Outgoing || symmetric)
            {
                Some(&relation.from)
            } else {
                None
            };
            if let Some(next) = next {
                if !view
                    .entities()
                    .any(|e| target_matches(next, e) && filters::scoped(view, e, query))
                {
                    continue;
                }
                if edges.len() >= 1000 {
                    response.omitted.push("trace edge bound reached".into());
                    break;
                }
                edges.insert(relation.id.clone(), relation.clone());
                pending.push_back((next.clone(), level.saturating_add(1)));
            }
        }
    }
    if edges.is_empty() {
        response
            .omitted
            .push("reason_not_recorded: no supported relation in the requested scope".into());
    }
    for edge in edges.into_values() {
        let entity = Entity::Relation(edge);
        rows.insert(entity.key(), view.item(&entity));
    }
    Ok(rows.into_values().collect())
}

fn target_eq(left: &Target, right: &Target) -> bool {
    match (left, right) {
        (
            Target::Code {
                state_id: a,
                path: p,
                range: x,
            },
            Target::Code {
                state_id: b,
                path: q,
                range: y,
            },
        ) => {
            a == b
                && p == q
                && match (x, y) {
                    (Some(x), Some(y)) => x.start_line <= y.end_line && y.start_line <= x.end_line,
                    _ => true,
                }
        }
        _ => left == right,
    }
}

fn compare(
    view: &View<'_>,
    query: &Query,
    response: &mut QueryResponse,
) -> Result<Vec<QueryItem>, QueryError> {
    if query.since_checkpoint.is_none() && query.from.is_none() && query.to.is_none() {
        return Err(QueryError::RescanRequired);
    }
    if let Some(checkpoint) = &query.since_checkpoint {
        let checkpoint: Checkpoint =
            view::decode(checkpoint).map_err(|_| QueryError::RescanRequired)?;
        if checkpoint.scope_hash != view::digest(&query.scope)?
            || checkpoint.sequence > view.snapshot
            || checkpoint.generation != view.corpus.compaction.generation
        {
            return Err(QueryError::RescanRequired);
        }
        let rows: Vec<_> = view.selected.values().filter(|entry| {
            if entry.entity.project_id() != query.scope.project_id || entry.sequence <= checkpoint.sequence || !view.authorized(entry.entity.source_id()) || !filters::scoped(view, &entry.entity, query) { return false; }
            if view.visible(&entry.entity) { return true; }
            matches!(&entry.entity, Entity::Record(record) if record.availability == Availability::Deleted)
                && view.corpus.entries.iter().any(|old| old.sequence <= checkpoint.sequence && old.entity.key() == entry.entity.key())
        })
            .map(|entry| {
                let mut item = view.item(&entry.entity);
                item.warnings.push(format!("newly captured or revised at {}; occurrence time may be earlier", entry.captured_at));
                item
            }).collect();
        response.comparison = Some(Comparison {
            mode: "since_checkpoint".into(),
            added: Vec::new(),
            removed: Vec::new(),
            code: None,
            notes: vec!["capture changes; not proof of newly occurring activity".into()],
        });
        return Ok(rows);
    }
    let from = query.from.as_ref().ok_or_else(|| {
        QueryError::InvalidQuery("compare requires from and to or since_checkpoint".into())
    })?;
    let to = query
        .to
        .as_ref()
        .ok_or_else(|| QueryError::InvalidQuery("compare requires to".into()))?;
    let has_history = from.occurred_at.is_some() || !from.session_last_records.is_empty();
    let mut comparison = Comparison {
        mode: "history".into(),
        added: Vec::new(),
        removed: Vec::new(),
        code: None,
        notes: Vec::new(),
    };
    let mut rows = Vec::new();
    if has_history && (to.occurred_at.is_some() || !to.session_last_records.is_empty()) {
        let from_records = at_history(view, query, from)?;
        let to_records = at_history(view, query, to)?;
        for (key, record) in &to_records {
            if !from_records.contains_key(key) {
                comparison.added.push(record.id.clone());
                rows.push(view.item(&Entity::Record((*record).clone())));
            }
        }
        comparison.removed = from_records
            .iter()
            .filter(|(key, _)| !to_records.contains_key(*key))
            .map(|(_, r)| r.id.clone())
            .collect();
        for (key, record) in &from_records {
            if !to_records.contains_key(key) {
                let mut item = view.item(&Entity::Record((*record).clone()));
                item.warnings
                    .push("present at from history point; absent from to history point".into());
                rows.push(item);
            }
        }
    } else {
        comparison.mode = "code_only".into();
        comparison
            .notes
            .push("historical goals and decisions are unknown without both history points".into());
    }
    if from.code_state_id.is_some() || to.code_state_id.is_some() {
        comparison.code = Some(compare_code(
            view,
            from.code_state_id.as_deref(),
            to.code_state_id.as_deref(),
        ));
    }
    response.comparison = Some(comparison);
    Ok(rows)
}

fn at_history<'a>(
    view: &'a View<'a>,
    query: &Query,
    point: &HistoryPoint,
) -> Result<BTreeMap<String, &'a Record>, QueryError> {
    let cutoff = point
        .occurred_at
        .as_deref()
        .map(|s| {
            filters::timestamp(s)
                .ok_or_else(|| QueryError::InvalidQuery("history time must be RFC3339".into()))
        })
        .transpose()?;
    let mut last = BTreeMap::new();
    for id in &point.session_last_records {
        let record = view.records().find(|r| &r.id == id).ok_or_else(|| {
            QueryError::InvalidQuery("history boundary record is unavailable".into())
        })?;
        let (Some(session), Some(order)) = (&record.session_id, record.source_order) else {
            return Err(QueryError::InvalidQuery(
                "history boundary requires session and source order".into(),
            ));
        };
        last.insert(session.clone(), order);
    }
    Ok(view
        .records()
        .filter(|r| filters::record(view, r, query))
        .filter(|r| {
            if let Some(cutoff) = cutoff {
                return r
                    .occurred_at
                    .as_deref()
                    .and_then(filters::timestamp)
                    .is_some_and(|t| t <= cutoff);
            }
            r.session_id
                .as_ref()
                .and_then(|s| last.get(s))
                .zip(r.source_order)
                .is_some_and(|(limit, order)| order <= *limit)
        })
        .map(|r| (format!("{}:{}", r.source_id, r.id), r))
        .collect())
}

fn compare_code(view: &View<'_>, from: Option<&str>, to: Option<&str>) -> CodeComparison {
    let find = |id: Option<&str>| {
        view.entities().find_map(|e| match e {
            Entity::CodeState(s) if Some(s.id.as_str()) == id => Some(s),
            _ => None,
        })
    };
    let mut result = CodeComparison {
        from_state: from.map(str::to_owned),
        to_state: to.map(str::to_owned),
        status: "unavailable".into(),
        changed_paths: Vec::new(),
        notes: vec![
            "code equality does not establish environment equality or test coverage".into(),
        ],
    };
    if let (Some(a), Some(b)) = (find(from), find(to)) {
        if a.repository_id != b.repository_id {
            result.notes.push("different repositories".into());
            return result;
        }
        let paths: BTreeSet<_> = a
            .files
            .iter()
            .chain(&b.files)
            .map(|f| f.path.clone())
            .collect();
        result.changed_paths = paths
            .into_iter()
            .filter(|path| {
                a.files.iter().find(|f| &f.path == path) != b.files.iter().find(|f| &f.path == path)
            })
            .collect();
        result.status = if a.changed_during_observation || b.changed_during_observation {
            "changed_during_observation"
        } else if result.changed_paths.is_empty() {
            "same_observed_files"
        } else {
            "different"
        }
        .into();
        result.notes.push(format!(
            "observed {} and {}; only retained file scope compared",
            a.observed_at, b.observed_at
        ));
    }
    result
}

pub(super) fn brief(view: &View<'_>, query: &Query, items: &[QueryItem]) -> Brief {
    let mut sections: BTreeMap<(u8, String), Vec<BriefClaim>> = BTreeMap::new();
    let relations: Vec<_> = view.relations().collect();
    let mut ids = BTreeSet::new();
    for item in items {
        if let Entity::Work(work) = &item.entity {
            let evidence = if work.evidence.is_empty() {
                vec![Evidence {
                    source_id: work.source_id.clone(),
                    record_id: None,
                    revision: crate::security::hash(work.goal.as_bytes()),
                    locator: format!("work:{}", work.id),
                    availability: Availability::Available,
                    range: None,
                    purpose: EvidencePurpose::Unspecified,
                    span: None,
                }]
            } else {
                work.evidence.clone()
            };
            sections
                .entry((0, "goals".into()))
                .or_default()
                .push(BriefClaim {
                    record_id: format!("work:{}", work.id),
                    revision: crate::security::hash(work.goal.as_bytes()),
                    text: work.goal.clone(),
                    nature: Nature::Reported,
                    fidelity: Fidelity::Original,
                    decision_status: None,
                    attempt_outcome: None,
                    verification_outcome: None,
                    evidence,
                    applies_to: Vec::new(),
                    warnings: vec![format!(
                        "last reported work status: {:?}; observed {:?}",
                        work.status, work.observed_at
                    )],
                    representation: Representation::Legacy,
                    context_id: None,
                });
            continue;
        }
        let Entity::Record(record) = &item.entity else {
            continue;
        };
        if record.representation == Representation::Evidence {
            continue;
        }
        ids.insert(record.id.clone());
        let mut warnings = item.warnings.clone();
        let mut applies_to = record.applies_to.clone();
        for relation in relations.iter().filter(|r| {
            r.kind == RelationKind::Supersedes
                && matches!(&r.to, Target::Record { id } if id == &record.id)
        }) {
            if let Target::Record { id } = &relation.from
                && !view.records().any(|r| {
                    &r.id == id
                        && filters::record(view, r, query)
                        && r.decision_status != Some(DecisionStatus::Proposed)
                })
            {
                continue;
            }
            if relation.applies_to.is_empty() {
                warnings.push("superseded: retained for historical context".into());
            } else {
                applies_to.retain(|scope| !relation.applies_to.contains(scope));
                warnings.push(format!("superseded only for {:?}; original text retained, other clauses remain applicable", relation.applies_to));
            }
        }
        if record.association != Association::Explicit {
            warnings.push(
                "candidate or unassigned context; not an established part of this work".into(),
            );
        }
        if record.derived {
            warnings.push("derived evidence; do not count as independent corroboration".into());
        }
        let name = match record.kind {
            RecordKind::Request => "goals",
            RecordKind::Constraint | RecordKind::Decision => "constraints_and_decisions",
            RecordKind::Attempt
            | RecordKind::Finding
            | RecordKind::ToolResult
            | RecordKind::Feedback => "process_and_attempts",
            RecordKind::Verification => "verification",
            RecordKind::Change | RecordKind::GitEvent => "changes",
            RecordKind::Status => "last_reported_state_and_open_items",
        };
        let claim = BriefClaim {
            record_id: record.id.clone(),
            revision: record.revision.clone(),
            text: record.body.clone(),
            nature: record.nature,
            fidelity: record.fidelity,
            decision_status: record.decision_status,
            attempt_outcome: record.attempt_outcome,
            verification_outcome: record.verification_outcome,
            evidence: record.evidence.clone(),
            applies_to,
            warnings,
            representation: record.representation,
            context_id: record.context_id.clone(),
        };
        sections
            .entry((priority(record.kind), name.into()))
            .or_default()
            .push(claim);
    }
    let conflicts = relations
        .into_iter()
        .filter(|r| {
            r.kind == RelationKind::Contradicts
                && [&r.from, &r.to]
                    .iter()
                    .all(|target| matches!(target, Target::Record { id } if ids.contains(id)))
        })
        .cloned()
        .collect();
    Brief { purpose: query.purpose.unwrap_or(BriefPurpose::Resume), sections: sections.into_iter().map(|((_, name), claims)| BriefSection { name, claims }).collect(), conflicts, notes: vec!["claims retain their observed/reported/inferred provenance; no absent rationale is reconstructed".into(), "completion reports, verification, and integration are distinct facts".into()] }
}

fn priority(kind: RecordKind) -> u8 {
    match kind {
        RecordKind::Request => 0,
        RecordKind::Constraint => 1,
        RecordKind::Status => 2,
        RecordKind::Decision => 3,
        RecordKind::Feedback => 4,
        RecordKind::Attempt => 5,
        RecordKind::Verification => 6,
        RecordKind::Finding | RecordKind::ToolResult => 7,
        RecordKind::Change | RecordKind::GitEvent => 8,
    }
}
