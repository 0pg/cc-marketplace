//! Bounded, on-demand mapping between two explicitly retained working observations.
//! This is correspondence evidence, never ownership, rationale, or verification evidence.

mod rust;

use crate::model::{CodeRef, CodeState, FileState, TextRange, WorkingFileKind};
use crate::query::LocationStatus;
use crate::security::hash;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const VERSION: &str = "working-sha256-lines-rust-syn-v2";
const MAX_PATHS: usize = 128;
const MAX_FILE_BYTES: usize = 1024 * 1024;
const MAX_TOTAL_BYTES: usize = 8 * 1024 * 1024;
const MAX_LINES: usize = 32_768;
const MAX_CANDIDATES: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MappingRequest {
    pub target_state_id: String,
    pub target_source_id: Option<String>,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateBasis {
    pub state_id: String,
    pub source_id: String,
    pub repository_id: String,
    pub worktree_id: Option<String>,
    pub observed_at: String,
    pub changed_during_observation: bool,
}

impl From<&CodeState> for StateBasis {
    fn from(state: &CodeState) -> Self {
        Self {
            state_id: state.id.clone(),
            source_id: state.source_id.clone(),
            repository_id: state.repository_id.clone(),
            worktree_id: state.worktree_id.clone(),
            observed_at: state.observed_at.clone(),
            changed_during_observation: state.changed_during_observation,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MappingMethod {
    WorkingContent,
    ExactLines,
    RustFunction,
    RustStatements,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Correspondence {
    ExactContent,
    AstEquivalent,
    Partial,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Symbol {
    pub language: String,
    pub kind: String,
    pub name: String,
    pub container: Vec<String>,
    pub range: TextRange,
    /// Zero-based, end-exclusive bytes in the retained file. Lines alone cannot
    /// distinguish separate Rust functions written on the same line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byte_range: Option<SymbolByteRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SymbolByteRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MappingCandidate {
    pub destination: CodeRef,
    pub method: MappingMethod,
    pub method_version: String,
    pub correspondence: Correspondence,
    pub original_support: TextRange,
    pub destination_support: TextRange,
    pub original_symbol: Option<Symbol>,
    pub destination_symbol: Option<Symbol>,
    pub original_working_sha256: String,
    pub destination_working_sha256: String,
    pub shared_statements: Option<usize>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IssueKind {
    InvalidRequest,
    RepositoryMismatch,
    OriginalNotObserved,
    DestinationNotObserved,
    ExplicitlyMissing,
    ContentNotRetained,
    RawHashUnavailable,
    MaskedOrChangedContent,
    NonRegularFile,
    ChangedDuringObservation,
    ParseFailed,
    UnsupportedLanguage,
    LimitReached,
    NoCorrespondence,
    PartialCorrespondence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MappingIssue {
    pub kind: IssueKind,
    pub state_id: String,
    pub path: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingReport {
    pub status: LocationStatus,
    pub original: CodeRef,
    pub from_basis: StateBasis,
    pub to_basis: StateBasis,
    pub inspected_paths: Vec<String>,
    pub candidates: Vec<MappingCandidate>,
    pub issues: Vec<MappingIssue>,
    pub limitations: Vec<String>,
}

impl MappingReport {
    fn issue(&mut self, kind: IssueKind, state: &CodeState, path: Option<&str>, detail: &str) {
        self.issues.push(MappingIssue {
            kind,
            state_id: state.id.clone(),
            path: path.map(str::to_owned),
            detail: detail.into(),
        });
    }

    fn candidate(&mut self, candidate: MappingCandidate) {
        if self.candidates.iter().any(|old| {
            old.destination == candidate.destination
                && old.original_symbol == candidate.original_symbol
                && old.destination_symbol == candidate.destination_symbol
        }) {
            // The same line can contain distinct source or destination symbols.
            // Only identical correspondence identities are duplicates.
            return;
        }
        if self.candidates.len() < MAX_CANDIDATES {
            self.candidates.push(candidate);
        }
    }
}

/// Callers must select both states under current project/source authorization first.
/// No repository access or source lookup occurs here; only retained content is read.
/// `paths` is the explicit destination search scope, not a claim of repository completeness.
pub fn map(
    original: &CodeRef,
    from: &CodeState,
    to: &CodeState,
    paths: &[String],
) -> MappingReport {
    let mut report = MappingReport {
        status: LocationStatus::Unavailable,
        original: original.clone(),
        from_basis: from.into(),
        to_basis: to.into(),
        inspected_paths: Vec::new(),
        candidates: Vec::new(),
        issues: Vec::new(),
        limitations: vec![
            "correspondence is limited to the explicitly selected retained working observations and destination paths".into(),
            "text or AST correspondence does not establish behavioral equivalence, authorship, work ownership, decision applicability, or verification of destination code".into(),
            "Rust macros are not expanded; external modules, types, bindings, and call graphs are not resolved".into(),
            "Git HEAD/index object IDs are not compared to raw working SHA-256; no unretained source is fetched".into(),
        ],
    };
    if original.state_id != from.id || paths.is_empty() || paths.iter().any(String::is_empty) {
        report.issue(
            IssueKind::InvalidRequest,
            from,
            None,
            "an explicit original state and nonempty destination paths are required",
        );
        return report;
    }
    if from.project_id != to.project_id || from.repository_id != to.repository_id {
        report.issue(
            IssueKind::RepositoryMismatch,
            to,
            None,
            "both states must belong to the same project and repository",
        );
        return report;
    }
    for state in [from, to] {
        if state.changed_during_observation {
            report.issue(IssueKind::ChangedDuringObservation, state, None, "observation changed while captured; candidates cannot establish one stable correspondence");
        }
    }
    let Some(source_file) = selected_file(from, &original.path) else {
        report.issue(IssueKind::OriginalNotObserved, from, Some(&original.path), "original path was not uniquely observed; absence from selected files does not prove deletion");
        return report;
    };
    let Some(source_text) = verified_text(source_file, from, &mut report) else {
        if source_file.working_kind == WorkingFileKind::Missing
            && !from.changed_during_observation
            && !to.changed_during_observation
        {
            report.status = LocationStatus::Missing;
        }
        return report;
    };
    let source_lines: Vec<_> = source_text.lines().collect();
    let original_range = original.range.clone().unwrap_or(TextRange {
        start_line: 1,
        end_line: source_lines.len().max(1),
    });
    if original_range.start_line == 0
        || original_range.end_line < original_range.start_line
        || original_range.end_line > source_lines.len().max(1)
    {
        report.issue(
            IssueKind::InvalidRequest,
            from,
            Some(&original.path),
            "original range must lie within retained one-based source lines",
        );
        return report;
    }
    let selected_lines =
        source_lines.get(original_range.start_line.saturating_sub(1)..original_range.end_line);
    let source_ast = if original.path.ends_with(".rs") {
        match rust::functions(source_text) {
            Ok(functions) => Some(functions),
            Err(error) => {
                report.issue(if matches!(error, rust::ParseError::Limit) {IssueKind::LimitReached} else {IssueKind::ParseFailed}, from, Some(&original.path), "retained Rust did not parse or exceeded 512 functions/2048 statements per function; exact content may still correspond, but symbol search is incomplete");
                None
            }
        }
    } else {
        None
    };
    let mut bytes = source_text.len();
    let unique_paths: BTreeSet<_> = paths.iter().collect();
    if unique_paths.len() > MAX_PATHS {
        report.issue(
            IssueKind::LimitReached,
            to,
            None,
            "destination path limit is 128; narrow scope",
        );
    }
    for path in unique_paths.into_iter().take(MAX_PATHS) {
        if report.candidates.len() >= MAX_CANDIDATES {
            report.issue(
                IssueKind::LimitReached,
                to,
                Some(path),
                "candidate limit is 100; narrow scope",
            );
            break;
        }
        let Some(target_file) = selected_file(to, path) else {
            report.issue(
                IssueKind::DestinationNotObserved,
                to,
                Some(path),
                "destination path was not uniquely observed; absence does not prove deletion",
            );
            continue;
        };
        report.inspected_paths.push(path.clone());
        let Some(target_text) = verified_text(target_file, to, &mut report) else {
            continue;
        };
        bytes = bytes.saturating_add(target_text.len());
        if bytes > MAX_TOTAL_BYTES {
            report.issue(
                IssueKind::LimitReached,
                to,
                Some(path),
                "total retained text comparison limit is 8 MiB; narrow scope",
            );
            break;
        }
        let target_lines: Vec<_> = target_text.lines().collect();
        if source_text == target_text {
            let candidate = candidate(
                from,
                to,
                source_file,
                target_file,
                &original_range,
                &original_range,
                MappingMethod::WorkingContent,
                Correspondence::ExactContent,
            );
            report.candidate(candidate);
        } else if let Some(selected) = selected_lines
            && !selected.is_empty()
        {
            for (offset, window) in target_lines.windows(selected.len()).enumerate() {
                if window == selected {
                    let destination_range = TextRange {
                        start_line: offset.saturating_add(1),
                        end_line: offset.saturating_add(selected.len()),
                    };
                    report.candidate(candidate(
                        from,
                        to,
                        source_file,
                        target_file,
                        &original_range,
                        &destination_range,
                        MappingMethod::ExactLines,
                        Correspondence::ExactContent,
                    ));
                    if report.candidates.len() >= MAX_CANDIDATES {
                        report.issue(
                            IssueKind::LimitReached,
                            to,
                            Some(path),
                            "candidate limit is 100; narrow scope",
                        );
                        break;
                    }
                }
            }
        }
        if let Some(source_functions) = &source_ast {
            if path.ends_with(".rs") {
                match rust::functions(target_text) {
                    Ok(target_functions) => {
                        let (matches, limited) = rust::correspondences(source_functions, &target_functions, &original_range);
                        if limited {report.issue(IssueKind::LimitReached, to, Some(path), "symbol comparison limit is 4096 pairs or 100 candidates; narrow the original range and destination paths");}
                        for (source, target, equivalent, shared) in &matches {
                            if let Some(existing) = report.candidates.iter_mut().find(|candidate| {
                                candidate.destination.path == target_file.path
                                    && candidate.correspondence == Correspondence::ExactContent
                                    && candidate.destination.range.as_ref().is_some_and(|range| range.start_line >= target.symbol.range.start_line && range.end_line <= target.symbol.range.end_line)
                            }) {
                                // Exact lines identify text, but do not identify a
                                // unique symbol when multiple functions share a line.
                                let unique_symbol_pair = matches.iter().filter(|(from_symbol, to_symbol, _, _)| {
                                    from_symbol.symbol.range.start_line <= original_range.start_line
                                        && from_symbol.symbol.range.end_line >= original_range.end_line
                                        && existing.destination.range.as_ref().is_some_and(|range| {
                                            range.start_line >= to_symbol.symbol.range.start_line
                                                && range.end_line <= to_symbol.symbol.range.end_line
                                        })
                                }).take(2).count() == 1;
                                if unique_symbol_pair {
                                    existing.original_symbol = Some(source.symbol.clone());
                                    existing.destination_symbol = Some(target.symbol.clone());
                                    existing.shared_statements = Some(*shared);
                                }
                                continue;
                            }
                            let full_source = symbol_covers_lines(source_text, &source.symbol, &original_range);
                            let mut candidate = candidate(from, to, source_file, target_file, &source.symbol.range, &target.symbol.range,
                                if *equivalent { MappingMethod::RustFunction } else { MappingMethod::RustStatements },
                                if *equivalent && full_source { Correspondence::AstEquivalent } else { Correspondence::Partial });
                            candidate.original_symbol = Some(source.symbol.clone());
                            candidate.destination_symbol = Some(target.symbol.clone());
                            candidate.shared_statements = Some(*shared);
                            if !*equivalent {
                                candidate.limitations.push("shared statements are partial evidence only; this may be a modified copy, split, merge, or unrelated reuse".into());
                            }
                            if !full_source {
                                candidate.limitations.push("symbol support differs from the requested range; inspect both original and destination spans".into());
                            }
                            report.candidate(candidate);
                        }
                    }
                    Err(error) => report.issue(if matches!(error, rust::ParseError::Limit) {IssueKind::LimitReached} else {IssueKind::ParseFailed}, to, Some(path), "retained Rust did not parse or exceeded 512 functions/2048 statements per function; exact content may still correspond, but symbol search is incomplete"),
                }
            } else {
                report.issue(
                    IssueKind::UnsupportedLanguage,
                    to,
                    Some(path),
                    "function correspondence supports Rust only; exact lines are still compared",
                );
            }
        } else if !original.path.ends_with(".rs") {
            report.issue(
                IssueKind::UnsupportedLanguage,
                from,
                Some(&original.path),
                "function correspondence supports Rust only; exact lines are still compared",
            );
        }
    }
    report.candidates.sort_by(|a, b| {
        a.destination.path.cmp(&b.destination.path).then_with(|| {
            a.destination
                .range
                .as_ref()
                .map(|r| r.start_line)
                .cmp(&b.destination.range.as_ref().map(|r| r.start_line))
        })
    });
    let incomplete = report.issues.iter().any(|issue| match issue.kind {
        IssueKind::ExplicitlyMissing => false,
        IssueKind::UnsupportedLanguage => {
            report.candidates.is_empty()
                || report.candidates.iter().any(|candidate| {
                    matches!(
                        candidate.method,
                        MappingMethod::RustFunction | MappingMethod::RustStatements
                    )
                })
        }
        _ => true,
    });
    let partial = report
        .candidates
        .iter()
        .any(|candidate| candidate.correspondence == Correspondence::Partial);
    report.status = match report.candidates.len() {
        0 if incomplete => LocationStatus::Unavailable,
        0 => LocationStatus::Missing,
        1 if !incomplete && !partial => LocationStatus::Mapped,
        _ => LocationStatus::Ambiguous,
    };
    if report.candidates.is_empty() {
        report.issue(IssueKind::NoCorrespondence, to, None, "no supported counterpart was found within selected observed paths; this is not proof of deletion throughout the repository");
    }
    if partial {
        report.issue(IssueKind::PartialCorrespondence, to, None, "partial symbol overlap cannot identify a single successor or transfer past decisions/tests");
    }
    report
}

fn symbol_covers_lines(text: &str, symbol: &Symbol, selected: &TextRange) -> bool {
    if symbol.range != *selected {
        return false;
    }
    let Some(bytes) = &symbol.byte_range else {
        return false;
    };
    let (Some(before), Some(after)) = (text.get(..bytes.start), text.get(bytes.end..)) else {
        return false;
    };
    // A line-based target may include other code beside this function. Matching
    // one function cannot establish correspondence for the entire selected line.
    before
        .rsplit('\n')
        .next()
        .is_some_and(|tail| tail.trim().is_empty())
        && after
            .split('\n')
            .next()
            .is_some_and(|head| head.trim().is_empty())
}

fn selected_file<'a>(state: &'a CodeState, path: &str) -> Option<&'a FileState> {
    let mut matches = state.files.iter().filter(|file| file.path == path);
    let file = matches.next()?;
    if matches.next().is_some() {
        None
    } else {
        Some(file)
    }
}

fn verified_text<'a>(
    file: &'a FileState,
    state: &CodeState,
    report: &mut MappingReport,
) -> Option<&'a str> {
    if file.working_kind == WorkingFileKind::Missing {
        report.issue(
            IssueKind::ExplicitlyMissing,
            state,
            Some(&file.path),
            "path was explicitly observed missing in this working state",
        );
        return None;
    }
    if !matches!(
        file.working_kind,
        WorkingFileKind::File | WorkingFileKind::Unknown
    ) {
        report.issue(IssueKind::NonRegularFile, state, Some(&file.path), "only retained regular working files are compared; ignored files, links, and directories are not followed");
        return None;
    }
    let Some(content) = file.working_content.as_deref() else {
        report.issue(
            IssueKind::ContentNotRetained,
            state,
            Some(&file.path),
            "working text was not retained; hash-only observations do not establish moved code",
        );
        return None;
    };
    let Some(raw_hash) = &file.working_hash else {
        report.issue(IssueKind::RawHashUnavailable, state, Some(&file.path), "raw working SHA-256 is absent; retained text cannot be assumed to be unchanged unmasked source");
        return None;
    };
    if hash(content.as_bytes()) != *raw_hash {
        report.issue(IssueKind::MaskedOrChangedContent, state, Some(&file.path), "retained text differs from raw working SHA-256, including possible masking; do not infer original-code equality or fetch raw content");
        return None;
    }
    if content.len() > MAX_FILE_BYTES
        || content.lines().take(MAX_LINES.saturating_add(1)).count() > MAX_LINES
    {
        report.issue(
            IssueKind::LimitReached,
            state,
            Some(&file.path),
            "per-file comparison limit is 1 MiB or 32768 lines; narrow the retained comparison",
        );
        return None;
    }
    Some(content)
}

#[allow(clippy::too_many_arguments)]
fn candidate(
    _from: &CodeState,
    to: &CodeState,
    source: &FileState,
    destination: &FileState,
    original_range: &TextRange,
    destination_range: &TextRange,
    method: MappingMethod,
    correspondence: Correspondence,
) -> MappingCandidate {
    MappingCandidate {
        destination: CodeRef {
            state_id: to.id.clone(),
            path: destination.path.clone(),
            range: Some(destination_range.clone()),
        },
        method,
        method_version: VERSION.into(),
        correspondence,
        original_support: original_range.clone(),
        destination_support: destination_range.clone(),
        original_symbol: None,
        destination_symbol: None,
        original_working_sha256: source.working_hash.clone().unwrap_or_default(),
        destination_working_sha256: destination.working_hash.clone().unwrap_or_default(),
        shared_statements: None,
        limitations: Vec::new(),
    }
}
