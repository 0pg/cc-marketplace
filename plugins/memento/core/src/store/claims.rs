//! Structural attribution checks. These do not establish semantic entailment.
use super::*;
use crate::record_input::Diagnostic;

/// If a batch source changes during redaction, the selected text must retain its
/// coordinates and content. Otherwise callers must project or read it first.
pub(super) fn validate_projection(entities: &[Entity], policy: &RedactionPolicy) -> Result<()> {
    for (record_index, entity) in entities.iter().enumerate() {
        let Entity::Record(claim) = entity else {
            continue;
        };
        if claim.representation != Representation::Claim {
            continue;
        }
        for (evidence_index, reference) in claim.evidence.iter().enumerate() {
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
                    return Err(Diagnostic::claim(
                        claim,
                        &format!("/data/evidence/{evidence_index}"),
                        "redaction changed selected evidence text or coordinates",
                        "Project or persist/read the source before choosing evidence ranges.",
                    )
                    .with_evidence(evidence_index, reference)
                    .with_record_index(record_index)
                    .into());
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
    let invalid = |path: &str, message: &str, hint: &str| -> Error {
        Diagnostic::claim(record, path, message, hint).into()
    };
    if record.body.trim().is_empty()
        || !record.derived
        || !record.context_id.as_ref().is_some_and(|id| {
            !id.trim().is_empty() && id.len() <= 1024 && !id.chars().any(char::is_control)
        })
    {
        return Err(invalid(
            "/data",
            "claims require nonempty body/context_id and derived=true",
            "Provide nonempty body/context_id and set derived=true on this claim.",
        ));
    }
    if !source_accessible(&record.project_id, &record.source_id, entries) {
        return Err(invalid(
            "/data/source_id",
            "claim source is absent, unavailable or unauthorized",
            "Use an available authorized source in the claim project; do not bypass source access controls.",
        ));
    }
    let mut origins = 0;
    for (evidence_index, reference) in record.evidence.iter().enumerate() {
        let invalid_evidence = |field: &str, message: &str, hint: &str| -> Error {
            Diagnostic::claim(
                record,
                &format!("/data/evidence/{evidence_index}{field}"),
                message,
                hint,
            )
            .with_evidence(evidence_index, reference)
            .into()
        };
        if reference.purpose == EvidencePurpose::Unspecified {
            return Err(invalid_evidence(
                "/purpose",
                "claim evidence requires origin or support purpose",
                "Set purpose to origin or support for this evidence reference.",
            ));
        }
        if !matches!(
            reference.availability,
            Availability::Available | Availability::Redacted
        ) {
            if !record.partial {
                return Err(Diagnostic::claim(
                    record, "/data/partial",
                    "unavailable claim evidence requires partial=true",
                    "Set partial=true if retaining this unavailable evidence, or reference available complete evidence.",
                ).with_evidence(evidence_index, reference).into());
            }
            continue;
        }
        let id = reference.record_id.as_ref().ok_or_else(|| {
            invalid_evidence(
                "/record_id",
                "available claim evidence requires record_id",
                "Add the retained evidence record ID to this reference.",
            )
        })?;
        if !source_accessible(&record.project_id, &reference.source_id, entries) {
            return Err(invalid_evidence(
                "/source_id",
                "claim evidence source is absent, unavailable or unauthorized",
                "Use accessible evidence from an authorized source; do not expand access permissions.",
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
            .ok_or_else(|| invalid_evidence("/revision", "exact claim evidence revision is absent", "Use the exact source_id, record_id and revision returned by reading the retained evidence."))?;
        if exact.iter().any(|(_, other)| other.body != target.body) {
            return Err(invalid_evidence(
                "/revision",
                "claim evidence revision identifies conflicting bodies",
                "Use a distinct revision identity for different retained bodies.",
            ));
        }
        if !matches!(
            target.availability,
            Availability::Available | Availability::Redacted
        ) || target.body.trim().is_empty()
        {
            return Err(invalid_evidence(
                "/revision",
                "claim evidence revision is inaccessible",
                "Reference an accessible retained evidence revision or explicitly mark the evidence unavailable and the claim partial.",
            ));
        }
        if target.source_id == record.source_id
            && target.id == record.id
            && target.revision == record.revision
        {
            return Err(invalid_evidence(
                "/record_id",
                "claim cannot cite itself as evidence",
                "Reference independently retained evidence instead of the claim itself.",
            ));
        }
        validate_range(reference, &target.body).map_err(|(field, message)| {
            invalid_evidence(
                field,
                message,
                "Choose a valid line range or UTF-8 byte span in the retained evidence body.",
            )
        })?;
        if reference.purpose == EvidencePurpose::Origin {
            if target.representation == Representation::Claim {
                return Err(invalid_evidence(
                    "/purpose",
                    "claim origin must be evidence or legacy content",
                    "Reference an evidence or legacy source record as origin; an earlier claim can be support.",
                ));
            }
            if (target.partial || target.fidelity == Fidelity::SourceTruncated) && !record.partial {
                return Err(Diagnostic::claim(
                    record, "/data/partial",
                    "truncated origin requires partial=true",
                    "The referenced origin has partial=true or fidelity=source_truncated. Set this claim's partial=true, or retain/read a complete original and replace this origin reference with its exact revision and range.",
                ).with_evidence(evidence_index, reference).into());
            }
            if target.fidelity != Fidelity::Original && record.fidelity == Fidelity::Original {
                return Err(invalid_evidence(
                    "/revision",
                    "claim cannot present a summarized/truncated origin as original",
                    "Set the claim fidelity to match the retained origin or reference a complete original evidence revision.",
                ));
            }
            origins += 1;
        }
    }
    if origins == 0 {
        return Err(invalid(
            "/data/evidence",
            "claims require at least one accessible exact origin",
            "Add an accessible evidence/legacy origin with its exact revision and a valid range or span.",
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

fn validate_range(
    reference: &Evidence,
    body: &str,
) -> std::result::Result<(), (&'static str, &'static str)> {
    if reference.range.is_none() && reference.span.is_none() {
        return Err(("", "claim evidence requires a line range or byte span"));
    }
    if let Some(range) = &reference.range
        && (range.start_line == 0
            || range.end_line < range.start_line
            || range.end_line > body.lines().count())
    {
        return Err((
            "/range",
            "claim evidence line range is outside retained body",
        ));
    }
    if let Some(span) = &reference.span {
        if span.start >= span.end
            || span.end > body.len()
            || !body.is_char_boundary(span.start)
            || !body.is_char_boundary(span.end)
        {
            return Err((
                "/span",
                "claim evidence span must be a nonempty UTF-8 byte range",
            ));
        }
        if let Some(range) = &reference.range {
            let start = body.get(..span.start).ok_or(("/span", "invalid span"))?;
            let end = body.get(..span.end).ok_or(("/span", "invalid span"))?;
            let start_line = start.bytes().filter(|byte| *byte == b'\n').count() + 1;
            let end_line = end
                .strip_suffix('\n')
                .unwrap_or(end)
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            if start_line < range.start_line || end_line > range.end_line {
                return Err(("/span", "claim evidence span is outside its line range"));
            }
        }
    }
    Ok(())
}
