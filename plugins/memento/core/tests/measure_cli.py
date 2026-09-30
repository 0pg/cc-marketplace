#!/usr/bin/env python3
"""Reproducible local latency sample; synthetic history, no user files or LLM."""
import hashlib
import json
from pathlib import Path
import platform
import statistics
import subprocess
import sys
import tempfile
import time

binary = str(Path(sys.argv[1] if len(sys.argv) > 1 else 'target/debug/work-context').resolve())
with tempfile.TemporaryDirectory(prefix='work-context-measure-') as directory:
    store = str(Path(directory) / 'context.sqlite')
    def call(command, *options, value=None):
        start = time.perf_counter()
        result = subprocess.run([binary, command, '--store', store, *options], input=json.dumps(value) if value is not None else None, text=True, capture_output=True, check=True)
        response = json.loads(result.stdout)
        if 'error' in response:
            raise RuntimeError(response['error'])
        return response, (time.perf_counter()-start)*1000
    call('init', '--project', 'measure', '--work', 'W1', '--session', 'S1', '--title', 'Synthetic performance sample', '--goal', 'Measure bounded retrieval')
    call('note', '--project', 'measure', '--work', 'W1', '--session', 'S1', value={'id':'seed', 'kind':'finding', 'body':'sample'})
    seed, _ = call('query', value={'operation':'read', 'scope':{'project_id':'measure'}, 'target':{'kind':'record','id':'seed'}})
    template = seed['items'][0]['entity']
    records = []
    for number in range(1000):
        record = json.loads(json.dumps(template))
        body = f'Experiment {number}: normal requests completed; execution reference X{number}; recorded conditions and next action.\n' * 8
        record['data'].update(id=f'event-{number}', body=body, revision=hashlib.sha256(body.encode()).hexdigest())
        records.append(record)
    long_record = json.loads(json.dumps(template))
    before = 'ordinary output line\n' * 500_000
    body = before + 'HTTP 429: authorization probe failed; password=synthetic-secret\n' + before
    long_record['data'].update(id='long-output', kind='tool_result', body=body, revision=hashlib.sha256(body.encode()).hexdigest())
    records.append(long_record)
    _, ingest_ms = call('record', value=records)
    cases = {
        'list_work': {'operation':'list_work'},
        'search': {'operation':'search','query':{'text':'HTTP 429','mode':'literal'}},
        'read_window': {'operation':'read','target':{'kind':'record','id':'long-output'},'range':{'start_line':500000,'end_line':500003},'context_lines':2},
        'brief_evidence_package': {'operation':'brief','purpose':'resume'},
    }
    measures = {}
    for name, request in cases.items():
        request.update(scope={'project_id':'measure'}, budget_bytes=32768, limit=20)
        times = []
        for _ in range(3):
            response, elapsed = call('query', value=request)
            assert len(json.dumps(response, separators=(',',':')).encode()) <= 32768
            assert 'synthetic-secret' not in json.dumps(response)
            times.append(elapsed)
        measures[name] = {'median_ms':round(statistics.median(times),2), 'max_ms':round(max(times),2), 'returned_items':len(response['items']), 'truncated':response['truncated']}
    print(json.dumps({'platform':platform.platform(),'binary':binary,'records':1002,'long_output_bytes':len(body.encode()),'samples_per_query':3,'ingest_ms':round(ingest_ms,2),'includes':'process startup + SQLite load + query + serialization','measurements':measures,'generative_brief':'not used: CLI returns evidence package; separate prose-generation latency not measured'}, indent=2))
