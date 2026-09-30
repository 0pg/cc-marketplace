use super::*;

pub(super) fn attach(
    view: &View<'_>,
    query: &Query,
    location: &mut Option<LocationResolution>,
) -> Result<(), QueryError> {
    let Some(request) = &query.code_mapping else {
        return Ok(());
    };
    if !matches!(query.operation, Operation::Read | Operation::Trace) {
        return Err(QueryError::InvalidQuery(
            "code_mapping requires a read or trace of the original code target".into(),
        ));
    }
    let Some(Target::Code {
        state_id,
        path,
        range,
    }) = &query.target
    else {
        return Err(QueryError::InvalidQuery(
            "code_mapping requires a code target".into(),
        ));
    };
    if state_id.is_empty() || request.target_state_id.is_empty() || request.paths.is_empty() {
        return Err(QueryError::InvalidQuery(
            "code_mapping requires explicit original and target states and destination paths"
                .into(),
        ));
    }
    if request
        .target_source_id
        .as_ref()
        .is_some_and(|source| !view.authorized(source))
    {
        return Err(QueryError::InvalidScope(
            "mapping target source is outside the accessible scope".into(),
        ));
    }
    let states: Vec<_> = view
        .entities()
        .filter(|entity| filters::scoped(view, entity, query))
        .filter_map(|entity| {
            if let Entity::CodeState(state) = entity {
                Some(state)
            } else {
                None
            }
        })
        .collect();
    let original: Vec<_> = states
        .iter()
        .copied()
        .filter(|state| &state.id == state_id)
        .collect();
    let destination: Vec<_> = states
        .iter()
        .copied()
        .filter(|state| {
            state.id == request.target_state_id
                && request
                    .target_source_id
                    .as_ref()
                    .is_none_or(|source| &state.source_id == source)
        })
        .collect();
    if original.len() > 1 || destination.len() > 1 {
        return Err(QueryError::InvalidScope(
            "mapping state is ambiguous; narrow the source scope".into(),
        ));
    }
    let Some(location) = location else {
        return Err(QueryError::InvalidQuery(
            "code mapping has no original location".into(),
        ));
    };
    let (Some(original), Some(destination)) = (original.first(), destination.first()) else {
        location.mapping_status = Some(LocationStatus::Unavailable);
        location.notes.push("code mapping unavailable: an explicitly requested state is not retained in the accessible scope; historical read/trace remains available".into());
        // Original address resolution is unchanged; absence of mapping cannot erase history.
        return Ok(());
    };
    let reference = CodeRef {
        state_id: state_id.clone(),
        path: path.clone(),
        range: range.clone(),
    };
    let mapping = crate::mapping::map(&reference, original, destination, &request.paths);
    location.mapping_status = Some(mapping.status);
    location.mapping = Some(mapping);
    Ok(())
}
