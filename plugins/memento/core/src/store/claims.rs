//! Structural attribution checks. These do not establish semantic entailment.
use super::*;

/// If a batch source changes during redaction, the selected text must retain its
/// coordinates and content. Otherwise callers must project or read it first.
pub(super) fn validate_projection(entities: &[Entity], policy: &RedactionPolicy) -> Result<()> {
    for entity in entities {
        let Entity::Record(claim) = entity else {
            continue;
        };
        if claim.representation != Representation::Claim {
            continue;
        }
        for reference in &claim.evidence {
            for source in entities.iter().filter_map(|entity| match entity {
                Entity::Record(record)
                    if record.project_id == claim.project_id
                        && record.source_id == reference.source_id
                        && Some(&record.id) == reference.record_id.as_ref()
                        && record.revision == reference.revision =>
                {
                    Some(record)
                }
                _ => None,
            }) {
                let projected = policy.redact(&source.body)?;
                if projected != source.body
                    && selected_text(reference, &source.body)
                        != selected_text(reference, &projected)
                {
                    return Err(Error::Invalid("redaction changed selected evidence text or coordinates; project or persist/read the source before choosing evidence ranges".into()));
                }
            }
        }
    }
    Ok(())
}

fn selected_text(reference: &Evidence, body: &str) -> Option<String> {
    if let Some(span) = &reference.span {
        return body.get(span.start..span.end).map(str::to_owned);
    }
    let range = reference.range.as_ref()?;
    if range.start_line == 0
        || range.start_line > range.end_line
        || range.end_line > body.lines().count()
    {
        return None;
    }
    Some(
        body.lines()
            .skip(range.start_line - 1)
            .take(range.end_line - range.start_line + 1)
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

pub(super) async fn validate_written(
    transaction: &mut toasty::Transaction<'_>,
    sequences: &[u64],
) -> Result<()> {
    let rows = StoredEntry::all().exec(&mut *transaction).await?;
    let entries = rows
        .into_iter()
        .filter(|row| !retention::is_metadata(row))
        .map(|row| {
            Ok(Entry {
                sequence: row.id,
                captured_at: row.captured_at,
                entity: serde_json::from_str(&row.payload)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    for entry in entries.iter().filter(|e| sequences.contains(&e.sequence)) {
        if let Entity::Record(record) = &entry.entity {
            validate_record(record, &entries)?;
        }
    }
    Ok(())
}

pub(crate) fn validate_record(record: &Record, entries: &[Entry]) -> Result<()> {
    if record.representation != Representation::Claim
        || !matches!(
            record.availability,
            Availability::Available | Availability::Redacted
        )
    {
        return Ok(());
    }
    if record.body.trim().is_empty()
        || !record.derived
        || !record.context_id.as_ref().is_some_and(|id| {
            !id.trim().is_empty() && id.len() <= 1024 && !id.chars().any(char::is_control)
        })
    {
        return Err(Error::Invalid(
            "claims require nonempty body/context_id and derived=true".into(),
        ));
    }
    if !source_accessible(&record.project_id, &record.source_id, entries) {
        return Err(Error::Invalid(
            "claim source is absent, unavailable or unauthorized".into(),
        ));
    }
    let mut origins = 0;
    for reference in &record.evidence {
        if reference.purpose == EvidencePurpose::Unspecified {
            return Err(Error::Invalid(
                "claim evidence requires origin or support purpose".into(),
            ));
        }
        if !matches!(
            reference.availability,
            Availability::Available | Availability::Redacted
        ) {
            if !record.partial {
                return Err(Error::Invalid(
                    "unavailable claim evidence requires partial=true".into(),
                ));
            }
            continue;
        }
        let id = reference
            .record_id
            .as_ref()
            .ok_or_else(|| Error::Invalid("available claim evidence requires record_id".into()))?;
        if !source_accessible(&record.project_id, &reference.source_id, entries) {
            return Err(Error::Invalid(
                "claim evidence source is absent, unavailable or unauthorized".into(),
            ));
        }
        let exact = entries
            .iter()
            .filter_map(|entry| match &entry.entity {
                Entity::Record(target)
                    if target.project_id == record.project_id
                        && target.source_id == reference.source_id
                        && &target.id == id
                        && target.revision == reference.revision =>
                {
                    Some((entry.sequence, target))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let target = exact
            .iter()
            .max_by_key(|(sequence, _)| *sequence)
            .map(|(_, target)| *target)
            .ok_or_else(|| Error::Invalid("exact claim evidence revision is absent".into()))?;
        if exact.iter().any(|(_, other)| other.body != target.body) {
            return Err(Error::Invalid(
                "claim evidence revision identifies conflicting bodies".into(),
            ));
        }
        if !matches!(
            target.availability,
            Availability::Available | Availability::Redacted
        ) || target.body.trim().is_empty()
        {
            return Err(Error::Invalid(
                "claim evidence revision is inaccessible".into(),
            ));
        }
        if target.source_id == record.source_id
            && target.id == record.id
            && target.revision == record.revision
        {
            return Err(Error::Invalid(
                "claim cannot cite itself as evidence".into(),
            ));
        }
        validate_range(reference, &target.body)?;
        if reference.purpose == EvidencePurpose::Origin {
            if target.representation == Representation::Claim {
                return Err(Error::Invalid(
                    "claim origin must be evidence or legacy content".into(),
                ));
            }
            if (target.partial || target.fidelity == Fidelity::SourceTruncated) && !record.partial {
                return Err(Error::Invalid(
                    "truncated origin requires partial=true".into(),
                ));
            }
            if target.fidelity != Fidelity::Original && record.fidelity == Fidelity::Original {
                return Err(Error::Invalid(
                    "claim cannot present a summarized/truncated origin as original".into(),
                ));
            }
            origins += 1;
        }
    }
    if origins == 0 {
        return Err(Error::Invalid(
            "claims require at least one accessible exact origin".into(),
        ));
    }
    Ok(())
}

fn source_accessible(project: &str, id: &str, entries: &[Entry]) -> bool {
    entries
        .iter()
        .filter_map(|entry| match &entry.entity {
            Entity::Source(source) if source.project_id == project && source.id == id => {
                Some((entry.sequence, source))
            }
            _ => None,
        })
        .max_by_key(|(sequence, _)| *sequence)
        .is_some_and(|(_, source)| source.authorized && source.available)
}

fn validate_range(reference: &Evidence, body: &str) -> Result<()> {
    if reference.range.is_none() && reference.span.is_none() {
        return Err(Error::Invalid(
            "claim evidence requires a line range or byte span".into(),
        ));
    }
    if let Some(range) = &reference.range
        && (range.start_line == 0
            || range.end_line < range.start_line
            || range.end_line > body.lines().count())
    {
        return Err(Error::Invalid(
            "claim evidence line range is outside retained body".into(),
        ));
    }
    if let Some(span) = &reference.span {
        if span.start >= span.end
            || span.end > body.len()
            || !body.is_char_boundary(span.start)
            || !body.is_char_boundary(span.end)
        {
            return Err(Error::Invalid(
                "claim evidence span must be a nonempty UTF-8 byte range".into(),
            ));
        }
        if let Some(range) = &reference.range {
            let start = body
                .get(..span.start)
                .ok_or_else(|| Error::Invalid("invalid span".into()))?;
            let end = body
                .get(..span.end)
                .ok_or_else(|| Error::Invalid("invalid span".into()))?;
            let start_line = start.bytes().filter(|byte| *byte == b'\n').count() + 1;
            let end_line = end
                .strip_suffix('\n')
                .unwrap_or(end)
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            if start_line < range.start_line || end_line > range.end_line {
                return Err(Error::Invalid(
                    "claim evidence span is outside its line range".into(),
                ));
            }
        }
    }
    Ok(())
}
