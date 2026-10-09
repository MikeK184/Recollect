#!/usr/bin/env python3
"""Matched public HotpotQA corpus on an isolated Cognee installation, not vendor scores."""
import asyncio
import hashlib
import json
import os
from pathlib import Path
import time

ROOT=Path(__file__).resolve().parent.parent
OUT=ROOT/".cache/cognee-bench"
RUNTIME=OUT/"runtime"
for name,value in {
    "COGNEE_LOG_FILE":"false", "COGNEE_LOGS_DIR":str(RUNTIME/"logs"),
    "COGNEE_REPOS_DIR":str(RUNTIME/"repos"), "TELEMETRY_DISABLED":"1",
    "SYSTEM_ROOT_DIRECTORY":str(RUNTIME/"system"), "DATA_ROOT_DIRECTORY":str(RUNTIME/"data"),
    "CACHE_ROOT_DIRECTORY":str(RUNTIME/"cache"), "ENABLE_BACKEND_ACCESS_CONTROL":"false",
    "CACHING":"false", "LLM_PROVIDER":"openai", "LLM_MODEL":"openai/z-ai/glm-5.3-flash",
    "LLM_ENDPOINT":"http://127.0.0.1:8799/v1", "LLM_API_KEY":"benchmark-proxy",
    "OPENAI_API_KEY":"benchmark-proxy", "OPENAI_BASE_URL":"http://127.0.0.1:8799/v1",
    "LLM_MAX_COMPLETION_TOKENS":"4096", "LLM_RATE_LIMIT_ENABLED":"true",
    "LLM_RATE_LIMIT_REQUESTS":"20", "LLM_RATE_LIMIT_INTERVAL":"60",
    "EMBEDDING_PROVIDER":"openai_compatible", "EMBEDDING_MODEL":"qwen/qwen3-embedding-8b",
    "EMBEDDING_DIMENSIONS":"1024", "EMBEDDING_ENDPOINT":"http://127.0.0.1:8799/v1",
    "EMBEDDING_API_KEY":"benchmark-proxy", "LOG_LEVEL":"ERROR",
}.items(): os.environ[name]=value

import cognee
from cognee.api.v1.search import SearchType

async def main():
    raw=(ROOT/".cache/public-benchmark-input/hotpotqa-validation-50.json").read_bytes()
    digest=hashlib.sha256(raw).hexdigest()
    assert digest=="32dd92947d6c50194bc2e76588bc78ad6ad08805a4b05728336ac70cd7e19b96"
    rows=[r['row'] for r in json.loads(raw)['rows']]
    docs={}
    for row in rows:
        for title,sentences in zip(row['context']['title'],row['context']['sentences']):
            content=title+'\n'+''.join(sentences)
            docs[title+'\0'+content]={'title':title,'content':content}
    report={'complete':False,'library':'cognee','version':'1.5.4','upstream_commit':(OUT/'upstream-commit.txt').read_text().strip(),'dataset_sha256':digest,'questions':50,'documents':len(docs),'retrieval':'SearchType.CHUNKS','top_k':10,'embedding_model':'qwen/qwen3-embedding-8b','dimensions':1024,'graph_model':'z-ai/glm-5.3-flash','chunk_size':2048,'data_per_batch':1,'results':[],'started_at':time.time()}
    def save(): (OUT/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    save()
    corpus=OUT/'corpus';corpus.mkdir(exist_ok=True)
    paths=[]
    for key,doc in docs.items():
        path=corpus/(hashlib.sha256(key.encode()).hexdigest()+'.txt')
        path.write_text(doc['content']);paths.append(str(path))
    dataset='frozen-hotpotqa-50'
    await cognee.add(paths,dataset_name=dataset)
    report['ingested_at']=time.time();save()
    # Normal graph enrichment; CHUNKS subsequently makes no answer/judge calls.
    await cognee.cognify(datasets=[dataset],chunk_size=2048,data_per_batch=1,chunks_per_batch=1,raise_on_error=True)
    report['indexed_at']=time.time();save()
    titles={d['title'] for d in docs.values()}
    for row in rows:
        start=time.perf_counter()
        result=await cognee.search(query_text=row['question'],query_type=SearchType.CHUNKS,datasets=[dataset],top_k=10)
        elapsed=(time.perf_counter()-start)*1000
        retrieved=set();unknown=0
        for item in result:
            text=item.get('text','') if isinstance(item,dict) else ''
            title=text.split('\n',1)[0]
            if title in titles: retrieved.add(title)
            else: unknown+=1
        gold=set(row['supporting_facts']['title']);hits=len(gold&retrieved)
        report['results'].append({'id':row['id'],'relevant':sorted(gold),'retrieved':sorted(retrieved),'recall':hits/len(gold),'complete_support':hits==len(gold),'unattributed_chunks':unknown,'elapsed_ms':elapsed})
        save()
    report['metrics']={'supporting_document_recall_at_10':sum(r['recall'] for r in report['results'])/50,'complete_support_rate':sum(r['complete_support'] for r in report['results'])/50,'unattributed_chunks':sum(r['unattributed_chunks'] for r in report['results'])}
    report['complete']=True;save()
    print(json.dumps({'report':str(OUT/'report.json'),'metrics':report['metrics']}))

asyncio.run(main())
