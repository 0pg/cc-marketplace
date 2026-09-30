use super::*;
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct Cursor {
    snapshot: u64,
    #[serde(default)]
    generation: u64,
    query_hash: String,
    guard: String,
    pub offset: usize,
    pub text_offset: usize,
    #[serde(default)]
    pub search_fingerprint: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct Checkpoint {
    pub scope_hash: String,
    pub sequence: u64,
    #[serde(default)]
    pub generation: u64,
}

pub(super) struct View<'a> {
    pub corpus: &'a Corpus,
    pub project_id: String,
    pub snapshot: u64,
    pub latest: BTreeMap<String, &'a Entry>,
    pub selected: BTreeMap<String, &'a Entry>,
    pub sources: Vec<&'a Source>,
    guard: String,
    pub search_fingerprint: Option<String>,
}

impl<'a> View<'a> {
    pub fn new(
        corpus: &'a Corpus,
        query: &Query,
        cursor: Option<&Cursor>,
    ) -> Result<Self, QueryError> {
        if query.scope.project_id.is_empty() {
            return Err(QueryError::InvalidScope("project_id is required".into()));
        }
        if cursor.is_some_and(|cursor| cursor.generation != corpus.compaction.generation) {
            return Err(QueryError::StaleCursor);
        }
        let maximum = corpus.entries.iter().map(|e| e.sequence).max().unwrap_or(0);
        let snapshot = cursor.map_or(maximum, |x| x.snapshot);
        if snapshot > maximum {
            return Err(QueryError::StaleCursor);
        }
        let mut latest = BTreeMap::new();
        let mut selected = BTreeMap::new();
        for entry in &corpus.entries {
            put_latest(&mut latest, entry);
            if entry.sequence <= snapshot {
                put_latest(&mut selected, entry);
            }
        }
        let sources: Vec<_> = latest
            .values()
            .filter_map(|e| match &e.entity {
                Entity::Source(s)
                    if s.authorized
                        && s.project_id == query.scope.project_id
                        && (query.scope.source_ids.is_empty()
                            || query.scope.source_ids.contains(&s.id)) =>
                {
                    Some(s)
                }
                _ => None,
            })
            .collect();
        if sources.is_empty()
            || query
                .scope
                .source_ids
                .iter()
                .any(|id| !sources.iter().any(|s| &s.id == id))
        {
            if cursor.is_some() {
                return Err(QueryError::StaleCursor);
            }
            return Err(QueryError::InvalidScope(
                "no accessible source in the requested scope".into(),
            ));
        }
        let guard_data: Vec<_> = selected
            .iter()
            .filter(|(_, e)| e.entity.project_id() == query.scope.project_id)
            .filter_map(|(key, old)| {
                latest
                    .get(key)
                    .map(|now| guard_value(&old.entity, &now.entity).map(|guard| (key, guard)))
            })
            .collect::<Result<_, _>>()?;
        let guard = digest(&guard_data)?;
        if let Some(cursor) = cursor
            && (cursor.query_hash != query_hash(query)? || cursor.guard != guard)
        {
            return Err(QueryError::StaleCursor);
        }
        Ok(Self {
            corpus,
            project_id: query.scope.project_id.clone(),
            snapshot,
            latest,
            selected,
            sources,
            guard,
            search_fingerprint: None,
        })
    }

    pub fn authorized(&self, source: &str) -> bool {
        self.sources.iter().any(|s| s.id == source)
    }

    pub fn visible(&self, entity: &Entity) -> bool {
        if entity.project_id() != self.project_id || !self.authorized(entity.source_id()) {
            return false;
        }
        if let Entity::Record(record) = entity {
            if record.availability == Availability::Deleted {
                return false;
            }
            if let Some(Entry {
                entity: Entity::Record(now),
                ..
            }) = self.latest.get(&entity.key()).copied()
                && now.availability == Availability::Deleted
            {
                return false;
            }
        }
        if let Entity::Relation(relation) = entity {
            return self.target_visible(&relation.from)
                && self.target_visible(&relation.to)
                && relation
                    .evidence
                    .iter()
                    .all(|evidence| self.authorized(&evidence.source_id));
        }
        true
    }

    pub fn entities(&self) -> impl Iterator<Item = &'a Entity> + '_ {
        self.selected
            .values()
            .filter(|e| self.visible(&e.entity))
            .map(|e| &e.entity)
    }

    pub fn records(&self) -> impl Iterator<Item = &'a Record> + '_ {
        self.entities().filter_map(|e| match e {
            Entity::Record(r) => Some(r),
            _ => None,
        })
    }

    pub fn relations(&self) -> impl Iterator<Item = &'a Relation> + '_ {
        self.entities().filter_map(|e| match e {
            Entity::Relation(r)
                if self.target_visible(&r.from)
                    && self.target_visible(&r.to)
                    && r.evidence.iter().all(|e| self.authorized(&e.source_id)) =>
            {
                Some(r)
            }
            _ => None,
        })
    }

    fn target_visible(&self, target: &Target) -> bool {
        self.selected
            .values()
            .map(|entry| &entry.entity)
            .filter(|entity| {
                !matches!(entity, Entity::Relation(_))
                    && self.visible(entity)
                    && target_matches(target, entity)
            })
            .take(2)
            .count()
            == 1
            || match target {
                Target::Code { state_id, path, .. } => {
                    self.selected.values().any(|entry| match &entry.entity {
                        Entity::Record(record) if self.visible(&entry.entity) => record
                            .code_refs
                            .iter()
                            .any(|code| &code.state_id == state_id && &code.path == path),
                        _ => false,
                    })
                }
                _ => false,
            }
    }

    pub fn evidence(&self, evidence: &[Evidence]) -> Vec<Evidence> {
        evidence
            .iter()
            .filter(|e| self.authorized(&e.source_id))
            .map(|e| {
                let mut result = e.clone();
                if let Some(id) = &e.record_id {
                    let now = self.latest.values().find_map(|entry| match &entry.entity {
                        Entity::Record(r)
                            if r.project_id == self.project_id
                                && &r.id == id
                                && r.source_id == e.source_id =>
                        {
                            Some(r)
                        }
                        _ => None,
                    });
                    match now {
                        Some(r) if r.availability == Availability::Deleted => {
                            result.availability = Availability::Deleted
                        }
                        Some(r)
                            if matches!(
                                r.availability,
                                Availability::Missing | Availability::Unsupported
                            ) =>
                        {
                            result.availability = r.availability
                        }
                        Some(r) if r.revision != e.revision => {
                            result.availability = self
                                .corpus
                                .entries
                                .iter()
                                .filter(|entry| entry.sequence <= self.snapshot)
                                .find_map(|entry| match &entry.entity {
                                    Entity::Record(old)
                                        if old.project_id == self.project_id
                                            && old.source_id == e.source_id
                                            && &old.id == id
                                            && old.revision == e.revision =>
                                    {
                                        Some(old.availability)
                                    }
                                    _ => None,
                                })
                                .unwrap_or(Availability::Missing);
                        }
                        None => result.availability = Availability::Missing,
                        _ => {}
                    }
                }
                result
            })
            .collect()
    }

    pub fn item(&self, entity: &Entity) -> QueryItem {
        let mut entity = entity.clone();
        let mut warnings = Vec::new();
        match &mut entity {
            Entity::Record(r) => {
                let evidence = self.evidence(&r.evidence);
                let lost = evidence.len() != r.evidence.len()
                    || evidence
                        .iter()
                        .any(|e| e.availability == Availability::Deleted);
                let stale = evidence.iter().any(|e| {
                    e.availability == Availability::Missing
                        || e.record_id.as_ref().is_some_and(|id| {
                            self.records().any(|current| {
                                current.source_id == e.source_id
                                    && &current.id == id
                                    && current.revision != e.revision
                            })
                        })
                });
                r.evidence = evidence;
                if r.fidelity != Fidelity::Original {
                    warnings.push(
                        match r.fidelity {
                            Fidelity::SummaryOnly => "summary_only",
                            Fidelity::SourceTruncated => "source_truncated",
                            Fidelity::Original => "original",
                        }
                        .into(),
                    );
                    if r.fidelity == Fidelity::SummaryOnly && r.nature == Nature::Observed {
                        r.nature = Nature::Reported;
                    }
                }
                if r.partial {
                    warnings.push("partial execution output; no final result established".into());
                }
                if let Some(execution) = &mut r.execution
                    && execution.ended_at.is_none()
                {
                    execution.liveness = Liveness::Unknown;
                    warnings.push(
                        "current execution liveness is unknown; last observation retained".into(),
                    );
                }
                if r.derived && stale {
                    warnings.push("stale_summary: cited revision changed or is unavailable".into());
                }
                if r.derived && lost {
                    r.body.clear();
                    r.title.clear();
                    r.alternatives.clear();
                    r.execution = None;
                    warnings.push("derived content unavailable after evidence removal".into());
                }
                if matches!(
                    r.availability,
                    Availability::Missing | Availability::Unsupported | Availability::Deleted
                ) {
                    r.body.clear();
                    warnings.push("retained metadata only; body unavailable".into());
                    if r.availability == Availability::Deleted {
                        r.title.clear();
                        r.alternatives.clear();
                        r.execution = None;
                    }
                }
                if r.evidence.is_empty() {
                    r.evidence.push(Evidence {
                        source_id: r.source_id.clone(),
                        record_id: Some(r.id.clone()),
                        revision: r.revision.clone(),
                        locator: format!("record:{}", r.id),
                        availability: r.availability,
                        range: None,
                    });
                }
            }
            Entity::Work(w) => w.evidence = self.evidence(&w.evidence),
            Entity::Relation(r) => r.evidence = self.evidence(&r.evidence),
            Entity::CodeState(state) => {
                for file in &mut state.files {
                    if matches!(
                        file.working_kind,
                        WorkingFileKind::Missing
                            | WorkingFileKind::Ignored
                            | WorkingFileKind::Directory
                            | WorkingFileKind::Symlink
                    ) {
                        file.working_content = None;
                    }
                }
            }
            _ => {}
        }
        let rendered_revision = match &entity {
            Entity::Record(record) => Some(crate::security::hash(record.body.as_bytes())),
            _ => None,
        };
        QueryItem {
            entity,
            rendered_revision,
            excerpt: None,
            match_locations: Vec::new(),
            match_ranges: Vec::new(),
            warnings,
            semantic: None,
        }
    }

    pub fn response(&self, query: &Query) -> QueryResponse {
        QueryResponse {
            operation: query.operation,
            scope: query.scope.clone(),
            query_snapshot: self.snapshot,
            checkpoint: None,
            next_cursor: None,
            status: ResponseStatus::Ok,
            truncated: false,
            omitted: if self.corpus.compaction.generation > 0 {
                vec![HISTORY_COMPACTED_WARNING.into()]
            } else {
                Vec::new()
            },
            coverage: self
                .sources
                .iter()
                .map(|s| {
                    let records: Vec<_> = self
                        .selected
                        .values()
                        .filter_map(|entry| match &entry.entity {
                            Entity::Record(record)
                                if record.project_id == self.project_id
                                    && record.source_id == s.id =>
                            {
                                Some(record)
                            }
                            _ => None,
                        })
                        .collect();
                    let kinds: std::collections::BTreeSet<_> = s
                        .record_kinds
                        .iter()
                        .copied()
                        .chain(records.iter().map(|r| r.kind))
                        .collect();
                    let mut gaps = s.gaps.clone();
                    for (label, count) in [
                        (
                            "missing retained bodies",
                            records
                                .iter()
                                .filter(|r| r.availability == Availability::Missing)
                                .count(),
                        ),
                        (
                            "deleted records; bodies unavailable",
                            records
                                .iter()
                                .filter(|r| r.availability == Availability::Deleted)
                                .count(),
                        ),
                        (
                            "summary_only records; original not established",
                            records
                                .iter()
                                .filter(|r| r.fidelity == Fidelity::SummaryOnly)
                                .count(),
                        ),
                        (
                            "source_truncated records; omitted original cannot be paged",
                            records
                                .iter()
                                .filter(|r| r.fidelity == Fidelity::SourceTruncated)
                                .count(),
                        ),
                    ] {
                        if count > 0 {
                            gaps.push(format!("{label}: {count}"));
                        }
                    }
                    SourceCoverage {
                        source_id: s.id.clone(),
                        record_kinds: kinds.into_iter().collect(),
                        gaps,
                        result_only: s.result_only,
                        time_unknown_records: records
                            .iter()
                            .filter(|r| r.occurred_at.is_none())
                            .count(),
                    }
                })
                .collect(),
            freshness: self
                .sources
                .iter()
                .map(|s| {
                    let retained = self
                        .selected
                        .values()
                        .filter(|entry| {
                            entry.entity.project_id() == self.project_id
                                && entry.entity.source_id() == s.id
                                && !matches!(entry.entity, Entity::Source(_))
                                && self.visible(&entry.entity)
                        })
                        .max_by_key(|entry| entry.sequence);
                    SourceFreshness {
                        source_id: s.id.clone(),
                        available: s.available,
                        last_captured_at: s
                            .last_captured_at
                            .clone()
                            .or_else(|| retained.map(|e| e.captured_at.clone())),
                        last_event_id: s
                            .last_event_id
                            .clone()
                            .or_else(|| retained.map(|e| e.entity.id().to_owned())),
                    }
                })
                .collect(),
            items: Vec::new(),
            relations: Vec::new(),
            brief: None,
            comparison: None,
            location: None,
            semantic: None,
        }
    }

    pub fn cursor(
        &self,
        query: &Query,
        offset: usize,
        text_offset: usize,
    ) -> Result<String, QueryError> {
        encode(&Cursor {
            snapshot: self.snapshot,
            generation: self.corpus.compaction.generation,
            query_hash: query_hash(query)?,
            guard: self.guard.clone(),
            offset,
            text_offset,
            search_fingerprint: self.search_fingerprint.clone(),
        })
    }

    pub fn checkpoint(&self, query: &Query) -> Result<String, QueryError> {
        encode(&Checkpoint {
            scope_hash: digest(&query.scope)?,
            sequence: self.snapshot,
            generation: self.corpus.compaction.generation,
        })
    }
}

fn put_latest<'a>(map: &mut BTreeMap<String, &'a Entry>, entry: &'a Entry) {
    let key = entry.entity.key();
    if map
        .get(&key)
        .is_none_or(|prior| prior.sequence <= entry.sequence)
    {
        map.insert(key, entry);
    }
}

fn guard_value(old: &Entity, current: &Entity) -> Result<String, QueryError> {
    match (old, current) {
        (_, Entity::Source(s)) => Ok(s.authorized.to_string()),
        (_, Entity::Record(_) | Entity::CodeState(_) | Entity::Relation(_)) => digest(current),
        _ => Ok(format!("{}:{}", old.key(), current.key())),
    }
}

pub(super) fn target_matches(target: &Target, entity: &Entity) -> bool {
    match (target, entity) {
        (Target::Record { id }, Entity::Record(r)) => id == &r.id,
        (
            Target::Artifact {
                record_id,
                revision,
                ..
            },
            Entity::Record(r),
        ) => record_id == &r.id && revision == &r.revision,
        (Target::Work { id }, Entity::Work(w)) => id == &w.id,
        (Target::Session { id }, Entity::Session(s)) => id == &s.id,
        (Target::Code { state_id, path, .. }, Entity::CodeState(s)) => {
            state_id == &s.id && s.files.iter().any(|f| &f.path == path)
        }
        (
            Target::Commit {
                repository_id,
                commit_sha,
            },
            Entity::Commit(c),
        ) => repository_id == &c.repository_id && commit_sha == &c.sha,
        _ => false,
    }
}

pub(super) fn digest(value: &impl Serialize) -> Result<String, QueryError> {
    serde_json::to_vec(value)
        .map(|b| crate::security::hash(&b))
        .map_err(|e| QueryError::InvalidQuery(e.to_string()))
}

fn query_hash(query: &Query) -> Result<String, QueryError> {
    let mut query = query.clone();
    query.cursor = None;
    digest(&query)
}

fn encode(value: &impl Serialize) -> Result<String, QueryError> {
    let bytes = serde_json::to_vec(value).map_err(|e| QueryError::InvalidQuery(e.to_string()))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

pub(super) fn decode<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, QueryError> {
    if text.len() > 16_384 || !text.len().is_multiple_of(2) {
        return Err(QueryError::StaleCursor);
    }
    let bytes = text
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let value = std::str::from_utf8(pair).map_err(|_| QueryError::StaleCursor)?;
            u8::from_str_radix(value, 16).map_err(|_| QueryError::StaleCursor)
        })
        .collect::<Result<Vec<_>, _>>()?;
    serde_json::from_slice(&bytes).map_err(|_| QueryError::StaleCursor)
}
