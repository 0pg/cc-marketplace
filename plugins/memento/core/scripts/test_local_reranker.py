"""Boundary checks; actual model parity is recorded separately in the evaluation."""
import contextlib
import hashlib
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import local_reranker as worker


def request(**changes):
    value = {'protocol': 1, 'model_id': worker.MODEL_ID, 'model_revision': worker.MODEL_REVISION,
             'query': 'private query', 'documents': ['private document']}
    value.update(changes)
    return io.BytesIO(json.dumps(value).encode())


class WorkerBoundaryTests(unittest.TestCase):
    def test_invalid_shapes_fail_before_model_loading(self):
        cases = [request(protocol=True), request(protocol=2), request(query=[]),
                 request(documents=['ok', 42]), request(documents=['x'] * 151),
                 request(model_revision='unapproved'), request(extra='unexpected'),
                 io.BytesIO(b'[]'), io.BytesIO(b'{private invalid json')]
        for value in cases:
            with self.subTest(value=value), self.assertRaises(worker.WorkerError):
                worker.read_request(value)

    def test_request_byte_limit(self):
        with mock.patch.object(worker, 'REQUEST_BYTES', 8):
            with self.assertRaisesRegex(worker.WorkerError, 'request_byte_limit'):
                worker.read_request(io.BytesIO(b'x' * 9))

    def test_manifest_checks_pin_and_file_bytes(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary).resolve()
            data = b'model fixture'
            (directory / 'weights.safetensors').write_bytes(data)
            manifest = {'model_id': worker.MODEL_ID, 'model_revision': worker.MODEL_REVISION,
                        'files': {'weights.safetensors': hashlib.sha256(data).hexdigest()}}
            path = directory / 'work-context-model.json'
            path.write_text(json.dumps(manifest))
            self.assertEqual(worker.verify_manifest(directory), manifest)
            (directory / 'weights.safetensors').write_bytes(b'changed')
            with self.assertRaisesRegex(worker.WorkerError, 'model_checksum_mismatch'):
                worker.verify_manifest(directory)
            manifest['model_revision'] = 'changed'
            path.write_text(json.dumps(manifest))
            with self.assertRaisesRegex(worker.WorkerError, 'model_manifest_mismatch'):
                worker.verify_manifest(directory)

    def test_manifest_cannot_escape_model_directory(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary).resolve()
            for relative in ['../outside', '/outside']:
                manifest = {'model_id': worker.MODEL_ID, 'model_revision': worker.MODEL_REVISION,
                            'files': {relative: '0' * 64}}
                (directory / 'work-context-model.json').write_text(json.dumps(manifest))
                with self.assertRaisesRegex(worker.WorkerError, 'invalid_manifest_path'):
                    worker.verify_manifest(directory)

    def test_missing_model_is_explicit(self):
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaisesRegex(worker.WorkerError, 'model_unavailable'):
                worker.verify_manifest(Path(temporary).resolve())

    def test_unexpected_error_does_not_echo_text(self):
        stderr = io.StringIO()
        with mock.patch.object(worker, 'main', side_effect=ValueError('PRIVATE_INPUT_SENTINEL')):
            with contextlib.redirect_stderr(stderr):
                self.assertEqual(worker.run(), 1)
        self.assertEqual(stderr.getvalue(), 'local reranker worker failed: runtime_failure\n')

    def test_token_overflow_fails_before_forward(self):
        class Tokenizer:
            def encode(self, text, **options):
                return [0]

            def convert_tokens_to_ids(self, text):
                return 1 if text == 'no' else 2

            def __call__(self, pairs, **options):
                return {'input_ids': [[0] * worker.PAIR_TOKEN_LIMIT for _ in pairs]}

            def pad(self, encoded, **options):
                import torch
                return {'input_ids': torch.tensor(encoded['input_ids'])}

        model = mock.Mock()
        with self.assertRaisesRegex(worker.WorkerError, 'input_token_limit'):
            worker.score(Tokenizer(), model, 'private query', ['private document'])
        model.assert_not_called()


if __name__ == '__main__':
    unittest.main()
