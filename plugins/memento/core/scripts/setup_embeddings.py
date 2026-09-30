#!/usr/bin/env python3
"""Explicit download of fixed public model artifacts, no work context is uploaded."""
import argparse
import hashlib
import json
import os
from pathlib import Path

MODELS = {
    'e5-small': ('intfloat/multilingual-e5-small', '614241f622f53c4eeff9890bdc4f31cfecc418b3'),
    'minilm': ('sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2', 'e8f8c211226b894fcb81acc59f3b34ba3efd5f42'),
    'qwen-reranker-0.6b': ('Qwen/Qwen3-Reranker-0.6B', 'e61197ed45024b0ed8a2d74b80b4d909f1255473'),
}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--model', choices=MODELS, default='e5-small')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    from huggingface_hub import snapshot_download
    name, revision = MODELS[args.model]
    directory = args.output.resolve()
    directory.mkdir(parents=True, exist_ok=True)
    snapshot_download(name, revision=revision, local_dir=directory,
                      allow_patterns=['*.json', '*.safetensors', '*.model', '*.txt', '*.jinja', '1_Pooling/*'],
                      ignore_patterns=['onnx/*', 'openvino/*', 'tf_*', '.eval_results/*'])
    files = {}
    for path in sorted(directory.rglob('*')):
        if path.is_file() and '.cache' not in path.parts and path.name != 'work-context-model.json':
            with path.open('rb') as stream:
                files[path.relative_to(directory).as_posix()] = hashlib.file_digest(stream, 'sha256').hexdigest()
    manifest = {'model_id': name, 'model_revision': revision, 'files': files}
    (directory / 'work-context-model.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps({'model_dir': str(directory), 'model_id': name, 'model_revision': revision,
                      'installed_bytes': sum((directory / p).stat().st_size for p in files)}, indent=2))


if __name__ == '__main__':
    main()
