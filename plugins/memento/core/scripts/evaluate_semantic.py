#!/usr/bin/env python3
"""Reproducible local candidate comparison on a predeclared synthetic split.

No work history is read. Raw-query literal, token-AND, and declared expert query
rewrites are baselines. Same 256-character/32-overlap field chunks as Rust.
This isolates retrieval/model costs; evaluate_semantic_cli.py checks full CLI.
"""
import argparse
import json
from pathlib import Path
import platform
import resource
import sys
import time

sys.dont_write_bytecode = True
from local_embeddings import load_model


def chunks(text):
    start = 0
    while start < len(text):
        end = min(start + 256, len(text))
        yield text[start:end]
        if end == len(text):
            return
        start = end - 32


def baseline(documents, question, rewrites=None):
    needles = rewrites if rewrites is not None else [question]
    ranked = []
    for doc in documents:
        text = (doc['title'] + '\n' + doc['body']).lower()
        score = sum(text.count(needle.lower()) for needle in needles)
        if score:
            ranked.append((doc['id'], score))
    return sorted(ranked, key=lambda item: (-item[1], item[0]))


def evaluate_model(directory, fixture, fixed_threshold=None):
    import numpy as np
    started = time.perf_counter()
    manifest, model = load_model(directory)
    loaded = time.perf_counter()
    documents = fixture['documents']
    passages, owners = [], []
    for doc in documents:
        compact_title = len(doc['title']) <= 96
        for field in ('title', 'body'):
            if field == 'title' and compact_title and doc['body']:
                continue
            for chunk in chunks(doc[field]):
                passages.append((doc['title'] + '\n' if field != 'title' and compact_title and doc['title'] else '') + chunk)
                owners.append(doc['id'])
    e5 = manifest['model_id'].startswith('intfloat/multilingual-e5-')
    encoded_passages = ['passage: ' + p for p in passages] if e5 else passages
    tokenized = model.tokenizer(encoded_passages, truncation=False, add_special_tokens=True)
    if any(len(ids) > model.max_seq_length for ids in tokenized['input_ids']):
        raise ValueError('fixture exceeds tokenizer window')
    indexed_start = time.perf_counter()
    passage_vectors = model.encode(encoded_passages, normalize_embeddings=True, show_progress_bar=False)
    indexed_end = time.perf_counter()
    rows = []
    for question in fixture['queries']:
        start = time.perf_counter()
        query = ('query: ' if e5 else '') + question['question']
        query_vector = model.encode([query], normalize_embeddings=True, show_progress_bar=False)[0]
        scores = passage_vectors @ query_vector
        by_id = {}
        for owner, score in zip(owners, scores):
            by_id[owner] = max(by_id.get(owner, -1), float(score))
        ranking = sorted(by_id.items(), key=lambda item: (-item[1], item[0]))
        rows.append({**question, 'ranking': ranking, 'query_ms': (time.perf_counter()-start)*1000})
    # Fix a threshold from development data only; held-out answers never tune it.
    positive = [score for row in rows if row['split']=='development' and row['required']
                for identity, score in row['ranking'] if identity in row['required']]
    negative = [row['ranking'][0][1] for row in rows if row['split']=='development' and not row['required']]
    threshold = fixed_threshold if fixed_threshold is not None else (min(positive) + max(negative)) / 2
    for row in rows:
        row['selected'] = [(identity, score) for identity, score in row['ranking'] if score >= threshold][:fixture['k']]
        ids = [identity for identity, _ in row['selected']]
        row['required_recall'] = sum(identity in ids for identity in row['required']) / len(row['required']) if row['required'] else None
        row['first_relevant_rank'] = next((rank for rank, (identity, _) in enumerate(row['ranking'],1) if identity in row['required']), None)
        row['forbidden_top1'] = bool(ids and ids[0] in row.get('forbidden_top1', []))
        row['unanswerable_candidates'] = len(ids) if not row['required'] else None
        for name, ranking in [('literal',baseline(documents,row['question'])),
                              ('tokens',[(identity,score) for identity,score in baseline(documents,row['question'],row['question'].split()) if all(term.lower() in (next(d for d in documents if d['id']==identity)['title']+' '+next(d for d in documents if d['id']==identity)['body']).lower() for term in row['question'].split())]),
                              ('rewrite',baseline(documents,row['question'],row['rewrite']))]:
            chosen = [identity for identity,_ in ranking[:fixture['k']]]
            row[name] = {'selected':chosen, 'required_recall':sum(identity in chosen for identity in row['required'])/len(row['required']) if row['required'] else None,
                         'queries':len(row['rewrite']) if name=='rewrite' else 1,
                         'query_bytes':len((' '.join(row['rewrite']) if name=='rewrite' else row['question']).encode())}
    heldout = [row for row in rows if row['split']=='evaluation' and row['required']]
    recall = sum(row['required_recall'] for row in heldout)/len(heldout)
    return {'model_id':manifest['model_id'], 'model_revision':manifest['model_revision'],
            'installed_bytes':sum((directory/p).stat().st_size for p in manifest['files']),
            'load_ms':(loaded-started)*1000,'index_ms':(indexed_end-indexed_start)*1000,
            'input_bytes':sum(len(p.encode()) for p in passages),'chunks':len(passages),
            'threshold_from_development_only':threshold,
            'evaluation_recall_at_3':recall,
            'baseline_recall_at_3':{name:sum(row[name]['required_recall'] for row in heldout)/len(heldout) for name in ['literal','tokens','rewrite']},
            'rows':rows}


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--fixture',type=Path,default=Path(__file__).parents[1]/'tests/fixtures/semantic/evaluation.json')
    parser.add_argument('--model-dir',action='append',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--threshold',type=float,help='Previously fixed threshold for a genuinely held-out fixture')
    args=parser.parse_args()
    fixture=json.loads(args.fixture.read_text())
    report={'environment':platform.platform(),'fixture':str(args.fixture),'targets':fixture['targets'],
            'scope':'synthetic candidate/model comparison; full CLI lifecycle is separately tested',
            'chunk_version':'field-title-context-characters-v2',
            'threshold_source':'frozen prior development' if args.threshold is not None else 'development split',
            'models':[evaluate_model(path.resolve(),fixture,args.threshold) for path in args.model_dir],
            'peak_rss_bytes':resource.getrusage(resource.RUSAGE_SELF).ru_maxrss*(1 if sys.platform=='darwin' else 1024)}
    args.output.write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps({m['model_id']:{key:m[key] for key in ['evaluation_recall_at_3','baseline_recall_at_3','threshold_from_development_only','load_ms','index_ms','installed_bytes']} for m in report['models']},indent=2))


if __name__=='__main__':main()
