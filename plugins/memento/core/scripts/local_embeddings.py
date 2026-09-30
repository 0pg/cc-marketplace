#!/usr/bin/env python3
"""Offline JSON embedding worker. Install models separately with setup_embeddings.py.

Runtime protocol v1 accepts already-masked query/documents on stdin, emits vectors
on stdout, and performs no networking. This process never caches input text/vectors.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import resource
import sys
import time

os.environ['HF_HUB_OFFLINE'] = '1'
os.environ['TRANSFORMERS_OFFLINE'] = '1'
os.environ['HF_HUB_DISABLE_TELEMETRY'] = '1'
os.environ['TOKENIZERS_PARALLELISM'] = 'false'


def load_model(directory):
    manifest = json.loads((directory / 'work-context-model.json').read_text())
    for relative, digest in manifest['files'].items():
        path = directory / relative
        if not path.is_relative_to(directory) or '..' in Path(relative).parts:
            raise ValueError('invalid model manifest path')
        with path.open('rb') as stream:
            actual = hashlib.file_digest(stream, 'sha256').hexdigest()
        if actual != digest:
            raise ValueError('model files changed; install the pinned revision again')
    from sentence_transformers import SentenceTransformer
    import torch
    torch.set_num_threads(min(4, os.cpu_count() or 1))
    model = SentenceTransformer(str(directory), device='cpu', local_files_only=True,
                                trust_remote_code=False, model_kwargs={'use_safetensors': True})
    return manifest, model


def embed(model, model_id, query, documents):
    queries = ['query: ' + query] if model_id.startswith('intfloat/multilingual-e5-') else [query]
    passages = ['passage: ' + text for text in documents] if model_id.startswith('intfloat/multilingual-e5-') else documents
    texts = queries + passages
    # Refuse hidden truncation; Rust owns exact source-range chunking.
    for offset in range(0, len(texts), 64):
        encoded = model.tokenizer(texts[offset:offset + 64], truncation=False, add_special_tokens=True)
        if any(len(ids) > model.max_seq_length for ids in encoded['input_ids']):
            raise ValueError('input exceeds model tokens; reduce chunk_chars or query length')
    vectors = model.encode(texts, batch_size=32, normalize_embeddings=True,
                           show_progress_bar=False, convert_to_numpy=True)
    return vectors[0].tolist(), vectors[1:].tolist()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--model-dir', type=Path, required=True)
    args = parser.parse_args()
    raw = sys.stdin.buffer.read(64 * 1024 * 1024 + 1)
    if len(raw) > 64 * 1024 * 1024:
        raise ValueError('request byte limit exceeded')
    request = json.loads(raw)
    if request.get('protocol') != 1 or not isinstance(request.get('query'), str):
        raise ValueError('unsupported embedding protocol')
    documents = request.get('documents')
    if not isinstance(documents, list) or len(documents) > 50_000 or any(not isinstance(v, str) for v in documents):
        raise ValueError('invalid document list')
    start = time.perf_counter()
    manifest, model = load_model(args.model_dir.resolve())
    loaded = time.perf_counter()
    if request.get('model_id') != manifest['model_id'] or request.get('model_revision') != manifest['model_revision']:
        raise ValueError('requested model differs from installed manifest')
    query, vectors = embed(model, manifest['model_id'], request['query'], documents)
    finished = time.perf_counter()
    json.dump({'protocol': 1, 'model_id': manifest['model_id'],
               'model_revision': manifest['model_revision'], 'query': query,
               'documents': vectors, 'metrics': {'model_load_ms': (loaded-start)*1000,
               'embedding_ms': (finished-loaded)*1000,
               'peak_rss_bytes': resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * (1 if sys.platform == 'darwin' else 1024)}}, sys.stdout, separators=(',', ':'))


if __name__ == '__main__':
    try:
        main()
    except Exception as exc:
        # Error text contains setup/shape information; never echo user inputs.
        print(f'local embedding worker failed: {type(exc).__name__}: {exc}', file=sys.stderr)
        sys.exit(1)
