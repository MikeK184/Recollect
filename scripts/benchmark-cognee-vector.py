#!/usr/bin/env python3
"""Cognee's real LanceDB/Qwen retrieval lane on the frozen corpus, without graph extraction."""
import asyncio
import hashlib
import json
import os
from pathlib import Path
import time

ROOT=Path(__file__).resolve().parent.parent
OUT=ROOT/'.cache/cognee-bench/vector'
OUT.mkdir(parents=True,exist_ok=True)
for name,value in {
    'COGNEE_LOG_FILE':'false','COGNEE_LOGS_DIR':str(OUT/'logs'),
    'COGNEE_REPOS_DIR':str(OUT/'repos'),'TELEMETRY_DISABLED':'1',
    'SYSTEM_ROOT_DIRECTORY':str(OUT/'system'),'DATA_ROOT_DIRECTORY':str(OUT/'data'),
    'CACHE_ROOT_DIRECTORY':str(OUT/'cache'),'ENABLE_BACKEND_ACCESS_CONTROL':'false',
    'CACHING':'false','VECTOR_DB_SUBPROCESS_ENABLED':'false','EMBEDDING_PROVIDER':'openai_compatible',
    'EMBEDDING_MODEL':'qwen/qwen3-embedding-8b','EMBEDDING_DIMENSIONS':'1024',
    'EMBEDDING_ENDPOINT':'http://127.0.0.1:8798/v1','EMBEDDING_API_KEY':'benchmark-proxy',
    'OPENAI_API_KEY':'benchmark-proxy','LOG_LEVEL':'ERROR',
}.items():os.environ[name]=value

from cognee.infrastructure.engine import DataPoint
from cognee.infrastructure.databases.vector import get_vector_engine_async

class Passage(DataPoint):
    text: str
    document_name: str
    metadata: dict = {'index_fields':['text']}

async def main():
    raw=(ROOT/'.cache/public-benchmark-input/hotpotqa-validation-50.json').read_bytes()
    digest=hashlib.sha256(raw).hexdigest()
    assert digest=='32dd92947d6c50194bc2e76588bc78ad6ad08805a4b05728336ac70cd7e19b96'
    rows=[r['row'] for r in json.loads(raw)['rows']]
    docs={}
    for row in rows:
        for title,sentences in zip(row['context']['title'],row['context']['sentences']):
            content=title+'\n'+''.join(sentences)
            docs[title+'\0'+content]={'title':title,'content':content}
    report={'complete':False,'library':'cognee','version':'1.5.4','upstream_commit':(OUT.parent/'upstream-commit.txt').read_text().strip(),'dataset_sha256':digest,'questions':50,'documents':len(docs),'lane':'Cognee native vector adapter; raw passages; graph extraction disabled','top_k':10,'embedding_model':'qwen/qwen3-embedding-8b','dimensions':1024,'query_instruction':'Given an engineering question, retrieve relevant passages that answer the question','representation':'Whole title + passage, one DataPoint per deduplicated document','transport_shim':'Send configured 1024 dimensions omitted by compatible engine; explicitly precreate native collection to avoid nested initial creation lock; no model fallback','started_at':time.time(),'results':[]}
    def save(): (OUT/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    save()
    engine=await get_vector_engine_async()
    print("Cognee native vector adapter ready",flush=True)
    await engine.create_collection("HotpotQA_passages",Passage)
    points=[Passage(text=d['content'],document_name=d['title']) for d in docs.values()]
    for start in range(0,len(points),8):
        print(f'Indexing passages {start}..{min(start+8,len(points))}',flush=True)
        await asyncio.wait_for(engine.create_data_points('HotpotQA_passages',points[start:start+8]),timeout=75)
        report['indexed_documents']=min(start+8,len(points));save()
    for row in rows:
        start=time.perf_counter()
        query='Instruct: '+report['query_instruction']+'\nQuery:'+row['question']
        found=await engine.search('HotpotQA_passages',query_text=query,limit=10,include_payload=True)
        retrieved={r.payload['document_name'] for r in found}
        gold=set(row['supporting_facts']['title']);hits=len(gold&retrieved)
        report['results'].append({'id':row['id'],'relevant':sorted(gold),'retrieved':sorted(retrieved),'recall':hits/len(gold),'complete_support':hits==len(gold),'elapsed_ms':(time.perf_counter()-start)*1000})
        save()
    report['metrics']={'supporting_document_recall_at_10':sum(r['recall'] for r in report['results'])/50,'complete_support_rate':sum(r['complete_support'] for r in report['results'])/50}
    report['complete']=True;save()
    print(json.dumps({'report':str(OUT/'report.json'),'metrics':report['metrics']}))

if __name__=='__main__':asyncio.run(main())
