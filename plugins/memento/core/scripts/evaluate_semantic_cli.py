#!/usr/bin/env python3
"""Run the frozen semantic fixture through real SQLite/CLI/model/read/trace.

Requires a prebuilt work-context and explicitly installed offline model. Synthetic
records only; temporary databases are removed. Output retains query responses.
"""
import argparse
import contextlib
import json
from pathlib import Path
import subprocess
import tempfile
import time


def run(binary, store, command, args=(), payload=None):
    started=time.perf_counter()
    completed=subprocess.run([str(binary),command,'--store',str(store),*map(str,args)],
                             input=json.dumps(payload,ensure_ascii=False) if payload is not None else '',
                             text=True,capture_output=True,timeout=180)
    value=json.loads(completed.stdout)
    if completed.returncode:
        raise RuntimeError(f'{command} failed: {value}')
    return value,(time.perf_counter()-started)*1000,len(completed.stdout.encode())


def request(text,mode='semantic',limit=3,work='evaluation'):
    return {'operation':'search','scope':{'project_id':'semantic-evaluation','source_ids':['journal'],'work_ids':[work]},
            'query':{'text':text,'mode':mode},'limit':limit,'budget_bytes':65536}


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--binary',type=Path,required=True)
    parser.add_argument('--config',type=Path,required=True)
    parser.add_argument('--fixture',type=Path,default=Path(__file__).parents[1]/'tests/fixtures/semantic/evaluation.json')
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--long-output',action='store_true')
    parser.add_argument('--store',type=Path,help='Retain a new synthetic store for independent forward testing')
    args=parser.parse_args()
    fixture=json.loads(args.fixture.read_text())
    k=fixture.get('k',3)
    if args.store is not None and args.store.exists():
        raise ValueError('evaluation store already exists; select a new path')
    with (contextlib.nullcontext(None) if args.store is not None else tempfile.TemporaryDirectory(prefix='work-context-semantic-cli-')) as temporary:
        store=args.store if args.store is not None else Path(temporary)/'context.sqlite'
        run(args.binary,store,'init',['--project','semantic-evaluation','--source','journal','--work','evaluation','--title','Synthetic retrieval evaluation','--goal','Recover original evidence'])
        ingest_start=time.perf_counter()
        for doc in fixture['documents']:
            run(args.binary,store,'note',['--project','semantic-evaluation','--source','journal','--work','evaluation'],
                {**doc,'kind':doc.get('kind','tool_result'),'revision':doc['revision'],'nature':'observed'})
        # Explicit fixture relationships; semantic similarity does not create these.
        for question in fixture['queries']:
            for required in question['required'][:1]:
                for followup in question.get('followup',[]):
                    run(args.binary,store,'record',payload={'entity':'relation','data':{
                        'id':required+'-supports-'+followup,'project_id':'semantic-evaluation','source_id':'journal',
                        'from':{'kind':'record','id':required},'to':{'kind':'record','id':followup},'kind':'supports','nature':'observed'}})
        ingest_ms=(time.perf_counter()-ingest_start)*1000
        rows=[]
        for question in fixture['queries']:
            if question['split']!='evaluation':continue
            queries=[]
            for mode in ['literal','tokens','semantic']:
                query=request(question['question'],mode,k)
                response,ms,size=run(args.binary,store,'query',['--semantic-config',args.config],query)
                ids=[item['entity']['data']['id'] for item in response['items']]
                queries.append({'mode':mode,'ms':ms,'response_bytes':size,'required_recall':sum(i in ids for i in question['required'])/len(question['required']) if question['required'] else None,'query':query,'response':response})
            rewrite=[]
            for expression in question.get('rewrite',[]):
                response,ms,size=run(args.binary,store,'query',payload=request(expression,'tokens',k))
                rewrite.append({'expression':expression,'ms':ms,'response_bytes':size,'response':response})
            # Inspect retrieved records; no expected ID is injected into search.
            follow=[]
            for item in queries[-1]['response']['items']:
                record=item['entity']['data']
                q={'operation':'read','scope':{'project_id':'semantic-evaluation','source_ids':['journal']},
                   'target':{'kind':'artifact','record_id':record['id'],'revision':record['revision'],'range':None},'budget_bytes':65536}
                response,ms,size=run(args.binary,store,'query',payload=q)
                follow.append({'operation':'read','record':record['id'],'ms':ms,'response_bytes':size,'response':response})
                q={'operation':'trace','scope':{'project_id':'semantic-evaluation','source_ids':['journal']},
                   'target':{'kind':'record','id':record['id']},'max_depth':2,'budget_bytes':65536}
                response,ms,size=run(args.binary,store,'query',payload=q)
                follow.append({'operation':'trace','record':record['id'],'ms':ms,'response_bytes':size,'response':response})
            rows.append({**question,'queries':queries,'rewrite':rewrite,'followup_reads':follow})
        long_result=None
        if args.long_output:
            repeated='ordinary output line\n'*524288
            body=repeated+'Database connection acquisition waited 1800 milliseconds because all connections were held by long transactions.\n'+repeated
            run(args.binary,store,'note',['--project','semantic-evaluation','--source','journal','--work','long'],{'id':'long-output','kind':'tool_result','revision':'long-v1','body':body,'nature':'observed'})
            q=request('데이터베이스 연결을 오래 기다려 느려진 원인',work='long')
            response,ms,size=run(args.binary,store,'query',['--semantic-config',args.config],q)
            item=response['items'][0]
            chunks=item['semantic'].get('rerank',{}).get('chunks',item['semantic']['chunks'])
            chunk=next(chunk for chunk in chunks if chunk['field']=='body')
            read={'operation':'read','scope':q['scope'],'target':{'kind':'artifact','record_id':'long-output','revision':'long-v1','range':chunk['range']},'budget_bytes':32768}
            original,read_ms,read_size=run(args.binary,store,'query',payload=read)
            long_result={'bytes':len(body.encode()),'needle_line':524289,'search_ms':ms,'read_ms':read_ms,'response_bytes':size+read_size,'query':q,'response':response,'read':original,
                         'range_covers_needle':chunk['range']['start_line']<=524289<=chunk['range']['end_line'],
                         'original_contains_evidence':any('1800 milliseconds' in i['entity']['data'].get('body','') for i in original['items'])}
        report={'fixture':str(args.fixture),'ingest_ms':ingest_ms,'config':json.loads(args.config.read_text()),'rows':rows,'long_output':long_result}
        args.output.write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
        print(json.dumps({'evaluation':[{r['id']:{q['mode']:q['required_recall'] for q in r['queries']}} for r in rows],
                          'long_output':None if long_result is None else {k:long_result[k] for k in ['bytes','search_ms','read_ms','response_bytes','range_covers_needle','original_contains_evidence']}},indent=2))


if __name__=='__main__':main()
