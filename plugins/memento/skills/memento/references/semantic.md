# Local semantic search

Use semantic search to retrieve candidate records when literal/token searches and a few concrete query rewrites miss paraphrased evidence. It supports only `operation: search`. It does not generate an answer or add causal links. Read candidates and follow explicit relationships before answering. The optional reranker uses one retrieved passage as unverified context; even a high final score can belong to a different incident. Keep literal/token rewrites and bounded follow-up reads available; do not rely on the first semantic page for completeness.

## Explicit setup

Install memento from the plugin root (requires Rust/Cargo, Python 3.9+, and `uv`). The default embedding model is `e5-small`:

```sh
python3 scripts/install_runtime.py
# Or select the other supplied embedding model:
python3 scripts/install_runtime.py --embedding-model minilm
```

| Selection | Pinned model |
| --- | --- |
| `e5-small` (default) | `intfloat/multilingual-e5-small` |
| `minilm` | `sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2` |

The installer prepares Python 3.12, installs the pinned dependencies and model weights, and checks the worker with a short public test query before replacing the plugin skill. It writes `<plugin-root>/skills/memento/semantic-config.json`. Runtime, weights, and a worker identified by its content hash are cached in `<plugin-parent>/.memento-semantic` by model and pinned setup, outside the replaceable plugin package. The configuration uses those absolute paths; moving the plugin cache on the same machine works while that external runtime remains available. Reinstalling the same setup reuses them and repeats the inference check. Changing the selected model prepares a separate runtime; existing query cursors must be restarted.

The skill's `scripts/memento.py` launcher automatically supplies that configuration for `query`. Use `mode: semantic` in the search JSON as shown below; no manual path editing is needed. An explicit `--semantic-config` overrides the installed selection. Running the Rust executable directly requires `--semantic-config <plugin-root>/skills/memento/semantic-config.json`. Omitting `--embedding-model` selects `e5-small` on both install and update. Use `--embedding-model none` to skip model setup and preserve any existing configuration. The installer sets up embeddings alone; configure the optional reranker manually below.

## Manual setup and optional reranking

The Rust CLI runs an explicitly configured local executable. The supplied worker uses CPU inference, an installed model directory, offline library settings, and no work-text/vector cache. Installation downloads public model weights and Python packages; ordinary search never downloads a model or sends the query to a model service. Use the supplied worker for this behavior; an arbitrary custom command has its own behavior.

Run these from the plugin root, choosing absolute runtime/model paths outside the repository:

```sh
uv venv --python 3.12 /absolute/context-venv
uv pip install --python /absolute/context-venv/bin/python -r core/scripts/semantic-requirements.txt
/absolute/context-venv/bin/python core/scripts/setup_embeddings.py --model e5-small --output /absolute/context-model
/absolute/context-venv/bin/python core/scripts/setup_embeddings.py --model qwen-reranker-0.6b --output /absolute/context-reranker
```

The setup script downloads the fixed revisions below and writes `work-context-model.json` with file SHA-256 hashes. Search checks those hashes, loads local safetensors, disables remote model code, and rejects token overflow rather than silently discarding part of a chunk. Together the weights occupy approximately 1.7 GB; Python/runtime memory and dependencies are additional. Installation is an explicit setup step, not something to repeat during retrieval.

Create `/absolute/semantic-config.json`, replacing all executable/script/model paths:

```json
{
  "command": ["/absolute/context-venv/bin/python", "/absolute/repository/core/scripts/local_embeddings.py", "--model-dir", "/absolute/context-model"],
  "model_id": "intfloat/multilingual-e5-small",
  "model_revision": "614241f622f53c4eeff9890bdc4f31cfecc418b3",
  "min_score": 0,
  "rerank": {
    "command": ["/absolute/context-venv/bin/python", "/absolute/repository/core/scripts/local_reranker.py", "--model-dir", "/absolute/context-reranker"],
    "model_id": "Qwen/Qwen3-Reranker-0.6B",
    "model_revision": "e61197ed45024b0ed8a2d74b80b4d909f1255473"
  }
}
```

With reranking enabled, `min_score` must be 0: the embedding pool and final ranking have no relevance cutoff. The API returns bounded candidates even for questions without an answer in the store. The seed threshold only controls an extra ranking pass, not whether an event is established. To use embeddings alone, omit `rerank` and the second model installation; the earlier independent E5-only test recovered 47.6% of required originals at k=3. The `minilm` setup option reproduces an earlier comparison. See the [upstream evaluation](https://github.com/0pg/0pg-mcp/blob/8d151993/docs/agent-work-context/p1-semantic-evaluation.md) for measured quality and limitations.

## Query and read

```sh
memento query --store /absolute/context.sqlite --semantic-config /absolute/semantic-config.json --input /absolute/query.json
```

```json
{"operation":"search","scope":{"project_id":"demo","work_ids":["W1"]},"query":{"text":"동시 요청 때문에 업로드가 막혔던 이유","mode":"semantic"},"limit":3,"budget_bytes":32768}
```

Project/source/work/session and typed filters apply before embedding and ranking. A failed-attempt filter still excludes a tool result without that typed status. Search the output separately, then trace to the attempt; semantic similarity does not relax filters.

Each item includes `semantic.score`, source record revision, projection revision, and at most three nonoverlapping `semantic.chunks`. Chunks identify the original field, one-based inclusive lines, byte span, and optional `context_fields` (such as the title supplied alongside a body chunk). These are evidence locations, not literal `match_ranges`. Read a body chunk with an artifact target containing the returned record ID, source `revision`, and chunk range. For title or execution fields, read the record and inspect that field; do not pass its line numbers as a body range. Keep `fidelity`, `source_truncated`, and warnings in the answer.

With reranking, those original cosine scores/chunks remain unchanged. `semantic.rerank` adds `initial_score`, final `score`, and expanded `chunks`; use these expanded ranges for the excerpt and original read. Expanded contexts may overlap, and do not count as separate evidence. The response's `semantic.reranking` identifies the model, method, stage count, and optional `seed`. Its `scored_context` identifies the initially scored passage; `context` identifies the actual shortened feedback range, with its source revision, projection hash and text hash. A title-only feedback fragment is identified as a title range.

Check whether the original actually describes the requested incident, conditions, action and verification. A nearby incident can become a high-scoring seed and reinforce itself. Reject that interpretation when the originals disagree; use actual returned terms for another bounded query if needed. If no original supports the question, report the searched scope and missing evidence even when three high-scoring candidates were returned. Reusing a source as feedback is not independent corroboration.

`semantic_unavailable` means a missing/invalid configuration, worker failure, or processing budget failure. Continue with literal/tokens search where useful. `no_matches` describes candidate absence within the selected scope and configuration; it does not establish absence of the underlying event. Neither cosine nor reranking scores prove a root cause or verification success.

## Bounds and freshness

The first implementation rebuilds a temporary in-memory index for each request. There is no persistent vector server, index migration, or background refresh. The fingerprint binds the query, masking, entire searchable projection (title/body/command/tool name/input), model revision, chunk version/configuration, candidate scores, and source snapshot. Identical input chunks share one embedding while their source ranges stay distinct.

Defaults: chunks of 256 characters with 32-character overlap; 10,000 records, 32 MiB projected text, 200,000 total chunks, 10,000 unique chunks, 4 KiB query, 100 candidate records, and 120 seconds. Explicit config fields can adjust these within bounded validation limits. Limits that leave parts uninspected appear in `omitted`; a processing failure returns an error. These are processing limits separate from `budget_bytes`, which caps the full serialized response including candidate metadata. Narrow scope or use exact search for large corpora. The long-output evaluation uses repeated output; it does not establish performance on equally large unique text.

Reranking defaults to the top 50 embedding records, up to three selected contexts per record, each expanded to 768 characters in its original field. It deduplicates identical input strings while preserving source locations. A first-stage score of at least `seed_min_score` (default 0.013634464528877288) permits one feedback pass using at most 768 characters including title context. `pool_limit`, `context_chars`, and `feedback_chars` can be reduced; contexts must still cover their selected embedding chunks. The supplied worker caps each query/context/document pair at 1024 model tokens, 150 documents, CPU 4 threads and batch 8. Both calls share the original request deadline; errors never silently fall back to earlier scores.

Every CLI query reloads and revalidates current source access and revisions after model inference. Confirmed deletion/revocation invalidates old candidates. Keep query parameters and config unchanged while using `next_cursor`; the original snapshot is pinned, and appended records are left for a fresh query. Projection changes, model/chunk/config changes, or changed candidate scores invalidate the cursor with `stale_cursor`. Restart the bounded search; never reuse old output after confirmed revocation.

This also applies to the feedback seed even when it is absent from the final page. Its model, method, original projection and used range participate in the fingerprint; processing times do not.

Models are loaded for each CLI request. Each reranking pass uses a separate bounded process, so a feedback search loads the reranker twice; pages also pay that cost. The previous exact-search latency figures do not apply to semantic search.
