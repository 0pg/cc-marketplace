#!/usr/bin/env python3
"""Offline, bounded Qwen relevance scores; source selection/PRF belongs to Rust."""
import argparse
import contextlib
import hashlib
import json
import math
import os
from pathlib import Path
import resource
import sys
import time

os.environ['HF_HUB_OFFLINE'] = '1'
os.environ['TRANSFORMERS_OFFLINE'] = '1'
os.environ['HF_HUB_DISABLE_TELEMETRY'] = '1'
os.environ['TOKENIZERS_PARALLELISM'] = 'false'

MODEL_ID = 'Qwen/Qwen3-Reranker-0.6B'
MODEL_REVISION = 'e61197ed45024b0ed8a2d74b80b4d909f1255473'
REQUEST_BYTES = 64 * 1024 * 1024
MAX_DOCUMENTS = 150
PAIR_TOKEN_LIMIT = 1024
BATCH_SIZE = 8
INSTRUCTION = 'Given a software work-history question, retrieve records that provide evidence for any requested aspect, including attempts, decisions or verification. Distinguish unrelated incidents.'
PREFIX = '<|im_start|>system\nJudge whether the Document meets the requirements based on the Query and the Instruct provided. Note that the answer can only be "yes" or "no".<|im_end|>\n<|im_start|>user\n'
SUFFIX = '<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n'


class WorkerError(Exception):
    """Only static error codes may cross the worker's stderr boundary."""


def read_request(stream):
    raw = stream.read(REQUEST_BYTES + 1)
    if len(raw) > REQUEST_BYTES:
        raise WorkerError('request_byte_limit')
    try:
        request = json.loads(raw)
    except (ValueError, UnicodeDecodeError):
        raise WorkerError('invalid_json') from None
    fields = {'protocol', 'model_id', 'model_revision', 'query', 'documents'}
    if not isinstance(request, dict) or set(request) != fields:
        raise WorkerError('invalid_request_shape')
    if type(request['protocol']) is not int or request['protocol'] != 1:
        raise WorkerError('unsupported_protocol')
    if not isinstance(request['query'], str):
        raise WorkerError('invalid_query')
    documents = request['documents']
    if not isinstance(documents, list) or len(documents) > MAX_DOCUMENTS or any(not isinstance(value, str) for value in documents):
        raise WorkerError('invalid_documents')
    if request['model_id'] != MODEL_ID or request['model_revision'] != MODEL_REVISION:
        raise WorkerError('unsupported_model')
    return request


def verify_manifest(directory):
    try:
        manifest = json.loads((directory / 'work-context-model.json').read_text())
    except FileNotFoundError:
        raise WorkerError('model_unavailable') from None
    except (ValueError, OSError):
        raise WorkerError('invalid_manifest') from None
    if not isinstance(manifest, dict) or manifest.get('model_id') != MODEL_ID or manifest.get('model_revision') != MODEL_REVISION:
        raise WorkerError('model_manifest_mismatch')
    files = manifest.get('files')
    if not isinstance(files, dict) or not files:
        raise WorkerError('invalid_manifest')
    for relative, digest in files.items():
        if not isinstance(relative, str) or not isinstance(digest, str) or len(digest) != 64:
            raise WorkerError('invalid_manifest')
        path = directory / relative
        if Path(relative).is_absolute() or '..' in Path(relative).parts or not path.resolve().is_relative_to(directory):
            raise WorkerError('invalid_manifest_path')
        try:
            with path.open('rb') as stream:
                actual = hashlib.file_digest(stream, 'sha256').hexdigest()
        except OSError:
            raise WorkerError('model_file_unavailable') from None
        if actual != digest:
            raise WorkerError('model_checksum_mismatch')
    return manifest


def load_model(directory):
    manifest = verify_manifest(directory)
    from transformers import AutoModelForCausalLM, AutoTokenizer
    import torch
    torch.set_num_threads(min(4, os.cpu_count() or 1))
    tokenizer = AutoTokenizer.from_pretrained(str(directory), local_files_only=True,
                                              trust_remote_code=False, padding_side='left')
    model = AutoModelForCausalLM.from_pretrained(str(directory), local_files_only=True,
                                                trust_remote_code=False, use_safetensors=True,
                                                dtype=torch.float32).to('cpu').eval()
    return manifest, tokenizer, model


def score(tokenizer, model, query, documents):
    import torch
    prefix_tokens = tokenizer.encode(PREFIX, add_special_tokens=False)
    suffix_tokens = tokenizer.encode(SUFFIX, add_special_tokens=False)
    no_id = tokenizer.convert_tokens_to_ids('no')
    yes_id = tokenizer.convert_tokens_to_ids('yes')
    if not isinstance(no_id, int) or not isinstance(yes_id, int) or min(no_id, yes_id) < 0 or no_id == yes_id:
        raise WorkerError('invalid_score_tokens')
    scores = []
    max_tokens = 0
    for offset in range(0, len(documents), BATCH_SIZE):
        pairs = [f'<Instruct>: {INSTRUCTION}\n<Query>: {query}\n<Document>: {document}'
                 for document in documents[offset:offset + BATCH_SIZE]]
        encoded = tokenizer(pairs, add_special_tokens=False, padding=False,
                            truncation=False, return_attention_mask=False)
        encoded['input_ids'] = [prefix_tokens + tokens + suffix_tokens for tokens in encoded['input_ids']]
        batch = tokenizer.pad(encoded, padding=True, return_tensors='pt')
        max_tokens = max(max_tokens, int(batch['input_ids'].shape[1]))
        if max_tokens > PAIR_TOKEN_LIMIT:
            raise WorkerError('input_token_limit')
        with torch.inference_mode():
            output = model(**batch, return_dict=True, use_cache=False, logits_to_keep=1).logits[:, -1, :]
            values = torch.sigmoid((output[:, yes_id] - output[:, no_id]).float()).tolist()
        if len(values) != len(pairs) or any(not math.isfinite(value) or not 0 <= value <= 1 for value in values):
            raise WorkerError('invalid_scores')
        scores.extend(values)
    return scores, max_tokens


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--model-dir', type=Path, required=True)
    args = parser.parse_args()
    request = read_request(sys.stdin.buffer)
    started = time.perf_counter()
    # Third-party diagnostics must never echo query/document content. Only our
    # static error codes are emitted after these redirects have been restored.
    with open(os.devnull, 'w') as quiet, contextlib.redirect_stdout(quiet), contextlib.redirect_stderr(quiet):
        manifest, tokenizer, model = load_model(args.model_dir.resolve())
        loaded = time.perf_counter()
        scores, max_tokens = score(tokenizer, model, request['query'], request['documents'])
        finished = time.perf_counter()
    response = {'protocol': 1, 'model_id': manifest['model_id'], 'model_revision': manifest['model_revision'],
                'scores': scores, 'metrics': {'model_load_ms': (loaded - started) * 1000,
                'reranking_ms': (finished - loaded) * 1000, 'document_count': len(scores),
                'max_pair_tokens': max_tokens,
                'peak_rss_bytes': resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * (1 if sys.platform == 'darwin' else 1024)}}
    json.dump(response, sys.stdout, separators=(',', ':'), allow_nan=False)


def run():
    try:
        main()
        return 0
    except WorkerError as error:
        print(f'local reranker worker failed: {error}', file=sys.stderr)
    except Exception:
        print('local reranker worker failed: runtime_failure', file=sys.stderr)
    return 1


if __name__ == '__main__':
    sys.exit(run())
