use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use memento::{
    Error, Result, Store, adapters::ImportFormat, compaction, ingest, model::*, query, runtime,
    security::RedactionPolicy, semantic,
};
use serde_json::{Value, json};

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();
    match execute().await {
        Ok(value) => match write_json(&value) {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::FAILURE,
        },
        Err(error) => {
            let output = match error {
                Error::Query(error) => json!({"error": error}),
                error @ Error::Capacity { .. } => {
                    json!({"error": {"code": "storage_capacity_exceeded", "message": error.to_string()}})
                }
                other => json!({"error": other.to_string()}),
            };
            let _ = write_json(&output);
            ExitCode::FAILURE
        }
    }
}

fn write_json(value: &Value) -> Result<()> {
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, value)?;
    writeln!(stdout)?;
    Ok(())
}

struct Args {
    command: String,
    options: BTreeMap<String, Vec<String>>,
    positional: Vec<String>,
    trailing: Vec<String>,
}
impl Args {
    fn parse() -> Result<Self> {
        let mut args = std::env::args().skip(1);
        let command = args.next().unwrap_or_else(|| "help".into());
        let mut result = Self {
            command,
            options: BTreeMap::new(),
            positional: Vec::new(),
            trailing: Vec::new(),
        };
        while let Some(arg) = args.next() {
            if arg == "--" {
                result.trailing.extend(args);
                break;
            }
            if let Some(key) = arg.strip_prefix("--") {
                let value = args
                    .next()
                    .ok_or_else(|| Error::Invalid(format!("{arg} requires a value")))?;
                result.options.entry(key.into()).or_default().push(value);
            } else {
                result.positional.push(arg);
            }
        }
        let known = [
            "store",
            "policy",
            "project",
            "source",
            "work",
            "session",
            "id",
            "input",
            "file",
            "format",
            "repository",
            "ref",
            "limit",
            "path",
            "allow",
            "title",
            "goal",
            "completeness",
            "semantic-config",
            "compaction-policy",
            "apply",
        ];
        if let Some(key) = result.options.keys().find(|k| !known.contains(&k.as_str())) {
            return Err(Error::Invalid(format!("unknown option --{key}")));
        }
        Ok(result)
    }
    fn get(&self, key: &str) -> Option<&str> {
        self.options
            .get(key)
            .and_then(|v| v.first())
            .map(String::as_str)
    }
    fn required(&self, key: &str) -> Result<&str> {
        self.get(key)
            .ok_or_else(|| Error::Invalid(format!("--{key} is required")))
    }
    fn many(&self, key: &str) -> Vec<String> {
        self.options.get(key).cloned().unwrap_or_default()
    }
    fn input(&self) -> Result<String> {
        match self.get("input") {
            Some(path) if path != "-" => Ok(std::fs::read_to_string(path)?),
            _ => {
                let mut s = String::new();
                std::io::stdin().read_to_string(&mut s)?;
                Ok(s)
            }
        }
    }
}

async fn execute() -> Result<Value> {
    let args = Args::parse()?;
    if matches!(args.command.as_str(), "help" | "--help") {
        return Ok(
            json!({"commands": ["init", "note", "record", "import", "sync", "query", "compact", "observe", "git-sync", "hooks-install", "hooks-status", "hook", "run", "delete-record", "source-access"], "usage": "memento COMMAND --store /absolute/context.sqlite [options]; note/query/record read JSON from --input FILE or stdin; compact previews, --apply true applies", "notes": "Explicitly select files and repositories. Query never executes historical commands. See skills/memento/references."}),
        );
    }
    let store_path = PathBuf::from(args.required("store")?);
    let policy: RedactionPolicy = match args.get("policy") {
        Some(path) => serde_json::from_str(&std::fs::read_to_string(path)?)?,
        None => RedactionPolicy::default(),
    };
    let mut store = Store::open(&store_path, policy.clone()).await?;
    match args.command.as_str() {
        "compact" => {
            let retention: Option<compaction::Policy> = args
                .get("compaction-policy")
                .map(|path| -> Result<_> {
                    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
                })
                .transpose()?;
            let apply = args
                .get("apply")
                .unwrap_or("false")
                .parse::<bool>()
                .map_err(|_| Error::Invalid("--apply must be true or false".into()))?;
            let report = store.compact(retention, apply).await?;
            Ok(json!({"applied": apply, "report": report}))
        }
        "init" => {
            let project = args.required("project")?;
            let source_id = args.get("source").unwrap_or("journal");
            let mut receipts = vec![
                store
                    .append(Entity::Source(ingest::source(
                        source_id,
                        project,
                        SourceKind::Journal,
                    )))
                    .await?,
            ];
            if let Some(id) = args.get("work") {
                receipts.push(
                    store
                        .append(Entity::Work(Work {
                            id: id.into(),
                            project_id: project.into(),
                            source_id: source_id.into(),
                            title: args.required("title")?.into(),
                            goal: args.required("goal")?.into(),
                            status: WorkStatus::Active,
                            observed_at: Some(chrono::Utc::now().to_rfc3339()),
                            evidence: Vec::new(),
                            completion_conditions: Vec::new(),
                        }))
                        .await?,
                );
            }
            if let Some(id) = args.get("session") {
                receipts.push(
                    store
                        .append(Entity::Session(Session {
                            id: id.into(),
                            project_id: project.into(),
                            source_id: source_id.into(),
                            work_ids: args.get("work").map(str::to_owned).into_iter().collect(),
                            status: SessionStatus::Active,
                            started_at: Some(chrono::Utc::now().to_rfc3339()),
                            ended_at: None,
                            worktree_id: None,
                            parent_id: None,
                            working_directory: None,
                        }))
                        .await?,
                );
            }
            Ok(
                json!({"receipts": receipts, "store": store_path, "capture": "Explicit records at decisions, failures, corrections, verification and handoff; no background history collection"}),
            )
        }
        "note" => {
            let input: Value = serde_json::from_str(&args.input()?)?;
            let input = input
                .as_object()
                .ok_or_else(|| Error::Invalid("note requires a JSON object".into()))?;
            let id = input
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| Error::Invalid("note id is required".into()))?;
            let body = input
                .get("body")
                .and_then(Value::as_str)
                .ok_or_else(|| Error::Invalid("note body is required".into()))?;
            let kind: RecordKind = serde_json::from_value(
                input
                    .get("kind")
                    .cloned()
                    .ok_or_else(|| Error::Invalid("note kind is required".into()))?,
            )?;
            let project = args.required("project")?;
            let source = args.get("source").unwrap_or("journal");
            let mut record = Record::new(id, project, source, kind, body);
            record.nature = Nature::Reported;
            record.work_ids = args.get("work").map(str::to_owned).into_iter().collect();
            record.association = if record.work_ids.is_empty() {
                Association::Unassigned
            } else {
                Association::Explicit
            };
            record.session_id = args.get("session").map(str::to_owned);
            let mut value = serde_json::to_value(record)?;
            let object = value
                .as_object_mut()
                .ok_or_else(|| Error::Invalid("invalid note".into()))?;
            for (key, item) in input {
                object.insert(key.clone(), item.clone());
            }
            let mut record: Record = serde_json::from_value(value)?;
            if !input.contains_key("revision") {
                record.revision.clear();
                record.revision = memento::security::hash(&serde_json::to_vec(&record)?);
            }
            if record.project_id != project || record.source_id != source {
                return Err(Error::Invalid(
                    "note project/source must match command scope".into(),
                ));
            }
            Ok(json!({"receipt": store.append(Entity::Record(record)).await?}))
        }
        "record" => {
            let value: Value = serde_json::from_str(&args.input()?)?;
            let entities: Vec<Entity> = if value.is_array() {
                serde_json::from_value(value)?
            } else {
                vec![serde_json::from_value(value)?]
            };
            Ok(json!({"receipts": store.append_all(entities).await?}))
        }
        "import" => {
            let format = match args.required("format")? {
                "codex" => ImportFormat::CodexJsonl,
                "document" => ImportFormat::Document,
                "journal" => ImportFormat::JournalJsonl,
                _ => {
                    return Err(Error::Invalid(
                        "format must be codex, document or journal".into(),
                    ));
                }
            };
            let completeness = match args.get("completeness").unwrap_or("full_snapshot") {
                "full_snapshot" => ImportCompleteness::FullSnapshot,
                "partial" => ImportCompleteness::Partial,
                "delta" => ImportCompleteness::Delta,
                _ => {
                    return Err(Error::Invalid(
                        "completeness must be full_snapshot, partial or delta".into(),
                    ));
                }
            };
            let receipts = ingest::import_with_completeness(
                &mut store,
                args.required("project")?,
                args.required("source")?,
                Path::new(args.required("file")?),
                format,
                args.get("work"),
                completeness,
            )
            .await?;
            Ok(json!({"receipts": receipts}))
        }
        "sync" => {
            let project = args.required("project")?;
            let sources = store.latest().await?;
            let mut results = Vec::new();
            for entity in sources {
                let Entity::Source(src) = entity else {
                    continue;
                };
                if src.project_id != project
                    || !src.authorized
                    || args.get("source").is_some_and(|id| id != src.id)
                {
                    continue;
                }
                let Some(path) = src.location else {
                    continue;
                };
                let format = match src.kind {
                    SourceKind::Codex => ImportFormat::CodexJsonl,
                    SourceKind::Document => ImportFormat::Document,
                    SourceKind::Journal => ImportFormat::JournalJsonl,
                    SourceKind::Git => continue,
                };
                match ingest::import_with_completeness(
                    &mut store,
                    project,
                    &src.id,
                    Path::new(&path),
                    format,
                    args.get("work"),
                    src.import_completeness,
                )
                .await
                {
                    Ok(receipts) => results.push(json!({"source": src.id, "receipts": receipts})),
                    Err(error) => {
                        results.push(json!({"source": src.id, "error": error.to_string()}))
                    }
                }
            }
            ingest::revalidate(&mut store).await?;
            Ok(json!({"sources": results}))
        }
        "query" => {
            let query: Query = serde_json::from_str(&args.input()?)?;
            ingest::revalidate(&mut store).await?;
            let corpus = store.load().await?;
            let response = if query
                .query
                .as_ref()
                .is_some_and(|text| text.mode == SearchMode::Semantic)
            {
                let config_path = args.get("semantic-config").ok_or_else(|| Error::Query(
                    query::QueryError::SemanticUnavailable("pass --semantic-config for an explicitly configured local model; literal/tokens remain available".into())))?;
                let config_text = std::fs::read_to_string(config_path).map_err(|error| {
                    Error::Query(query::QueryError::SemanticUnavailable(format!(
                        "cannot read local model configuration: {error}"
                    )))
                })?;
                let config: semantic::SemanticConfig =
                    serde_json::from_str(&config_text).map_err(|error| {
                        Error::Query(query::QueryError::SemanticUnavailable(format!(
                            "invalid local model configuration: {error}"
                        )))
                    })?;
                let records = query::prepare_semantic_records(&corpus, &query)?;
                let text = query
                    .query
                    .as_ref()
                    .map(|text| text.text.as_str())
                    .ok_or_else(|| Error::Invalid("semantic query missing".into()))?;
                let prepared = semantic::prepare(&records, text, &config, &policy)
                    .and_then(|prepared| {
                        prepared.with_compaction_generation(corpus.compaction.generation)
                    })
                    .map_err(|error| {
                        Error::Query(query::QueryError::SemanticUnavailable(error.to_string()))
                    })?;
                drop(records);
                drop(corpus);
                // A local model may take time. Re-check files, revocations and
                // revisions before returning anything from its old input snapshot.
                ingest::revalidate(&mut store).await?;
                let current = store.load().await?;
                query::execute_semantic(&current, &query, &prepared, &policy)?
            } else {
                query::execute(&corpus, &query)?
            };
            Ok(serde_json::to_value(response)?)
        }
        "observe" => {
            let paths: Vec<_> = args.many("path").into_iter().map(PathBuf::from).collect();
            Ok(serde_json::to_value(
                runtime::observe(
                    &mut store,
                    args.required("project")?,
                    Path::new(args.required("repository")?),
                    &paths,
                )
                .await?,
            )?)
        }
        "git-sync" => {
            let refs = if args.many("ref").is_empty() {
                vec!["HEAD".into()]
            } else {
                args.many("ref")
            };
            let limit = args
                .get("limit")
                .unwrap_or("20")
                .parse()
                .map_err(|_| Error::Invalid("limit must be an integer".into()))?;
            runtime::git_sync(
                &mut store,
                args.required("project")?,
                Path::new(args.required("repository")?),
                &refs,
                limit,
            )
            .await
        }
        "hooks-install" => Ok(serde_json::to_value(
            memento::git::install_hooks_with_policy(
                Path::new(args.required("repository")?),
                &std::env::current_exe()?,
                &store_path.canonicalize()?,
                args.required("project")?,
                args.get("policy").map(Path::new),
            )
            .map_err(|e| Error::Invalid(e.to_string()))?,
        )?),
        "hooks-status" => Ok(serde_json::to_value(
            memento::git::hook_status(Path::new(args.required("repository")?))
                .map_err(|e| Error::Invalid(e.to_string()))?,
        )?),
        "hook" => {
            let kind = args
                .positional
                .first()
                .ok_or_else(|| Error::Invalid("hook kind is required".into()))?;
            let input = if kind == "post-rewrite" {
                args.input()?
            } else {
                String::new()
            };
            Ok(
                json!({"receipts": runtime::hook(&mut store, args.required("project")?, Path::new(args.required("repository")?), kind, args.positional.get(1).map(String::as_str), &input).await?}),
            )
        }
        "run" => {
            let paths: Vec<_> = args.many("path").into_iter().map(PathBuf::from).collect();
            let receipts = runtime::capture_run(
                &mut store,
                runtime::RunRequest {
                    project: args.required("project")?,
                    source: args.get("source").unwrap_or("journal"),
                    work: args.required("work")?,
                    session: args.required("session")?,
                    execution: args.required("id")?,
                    repository: Path::new(args.required("repository")?),
                    paths: &paths,
                    command: &args.trailing,
                },
            )
            .await?;
            Ok(json!({"receipts": receipts}))
        }
        "source-access" => {
            let project = args.required("project")?;
            let id = args.required("source")?;
            let allow: bool = args
                .required("allow")?
                .parse()
                .map_err(|_| Error::Invalid("allow must be true or false".into()))?;
            let source = store
                .latest()
                .await?
                .into_iter()
                .find_map(|e| match e {
                    Entity::Source(s) if s.id == id && s.project_id == project => Some(s),
                    _ => None,
                })
                .ok_or_else(|| Error::Invalid("source not found".into()))?;
            let mut source = source;
            source.authorized = allow;
            source.last_captured_at = Some(chrono::Utc::now().to_rfc3339());
            Ok(json!({"receipt": store.append(Entity::Source(source)).await?}))
        }
        "delete-record" => {
            let id = args.required("id")?;
            let project = args.required("project")?;
            let record = store
                .latest()
                .await?
                .into_iter()
                .find_map(|e| match e {
                    Entity::Record(r) if r.id == id && r.project_id == project => Some(r),
                    _ => None,
                })
                .ok_or_else(|| Error::Invalid("record not found".into()))?;
            let mut record = record;
            record.availability = Availability::Deleted;
            record.body.clear();
            record.title.clear();
            record.revision = format!("deleted:{}", record.revision);
            Ok(json!({"receipt": store.append(Entity::Record(record)).await?}))
        }
        _ => Err(Error::Invalid("unknown command; run help".into())),
    }
}
