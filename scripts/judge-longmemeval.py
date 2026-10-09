#!/usr/bin/env python3
"""Run the hash-pinned upstream evaluator/metrics unmodified through the audited loopback proxy."""
import hashlib
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT=Path(__file__).resolve().parent.parent
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('run',type=Path)
parser.add_argument('--resume-primary',action='store_true',help='Explicit recovery of a terminal partial judge run; retain completed prefix')
args=parser.parse_args()
run=args.run.resolve()
assert run.is_relative_to(ROOT/'.cache'), 'Only owned benchmark reports'
report=json.loads((run/'report.json').read_text())
assert report['complete'] and len(report['rows'])==50
inputs=ROOT/'.cache/longmemeval-input'
manifest=json.loads((inputs/'manifest.json').read_text())
for name in ['evaluate_qa.py','print_qa_metrics.py','subset-50.json']:
    assert hashlib.sha256((inputs/name).read_bytes()).hexdigest()==manifest['sha256'][name]
env=os.environ.copy()
env.update({'OPENAI_API_KEY':'benchmark-proxy','OPENAI_BASE_URL':'http://127.0.0.1:8797/v1','PYTHONDONTWRITEBYTECODE':'1'})
env.pop('OPENAI_ORGANIZATION',None)
assert len({r['id'] for r in report['rows']})==50
assert {r['id'] for r in report['rows']}==set(manifest['question_ids'])
by_id={r['id']:r for r in report['rows']}
rows=[{'question_id':qid,'hypothesis':by_id[qid]['hypothesis']} for qid in manifest['question_ids']]
for label,selected in [('primary',rows),('audit-first-25',rows[:25])]:
    hyp=run/f'{label}.jsonl'
    hyp.write_text(''.join(json.dumps(r)+'\n' for r in selected))
    result=Path(str(hyp)+'.eval-results-gpt-4o')
    prefix=[]
    if result.exists():
        assert args.resume_primary and label=='primary', f'Existing judgment attempt: {result}'
        prefix=[json.loads(line) for line in result.read_text().splitlines()]
        assert 0<len(prefix)<len(selected)
        assert [r['question_id'] for r in prefix]==[r['question_id'] for r in selected[:len(prefix)]]
        assert all(r['hypothesis']==h['hypothesis'] and r['autoeval_label']['model']=='gpt-4o-2024-08-06' for r,h in zip(prefix,selected))
        backup=run/f'primary-partial-{hashlib.sha256(result.read_bytes()).hexdigest()[:16]}.jsonl'
        assert not backup.exists(), 'Identical partial attempt already preserved; reconcile before another recovery'
        backup.write_bytes(result.read_bytes())
        hyp=run/f'primary-recovery-{len(selected)-len(prefix)}.jsonl'
        assert not Path(str(hyp)+'.eval-results-gpt-4o').exists()
        hyp.write_text(''.join(json.dumps(r)+'\n' for r in selected[len(prefix):]))
    with (run/f'{label}-evaluator.log').open('w') as out:
        subprocess.run([sys.executable,str(inputs/'evaluate_qa.py'),'gpt-4o',str(hyp),str(inputs/'subset-50.json')],env=env,stdout=out,stderr=subprocess.STDOUT,check=True,timeout=900)
    if prefix:
        suffix=[json.loads(l) for l in Path(str(hyp)+'.eval-results-gpt-4o').read_text().splitlines()]
        assert len(suffix)+len(prefix)==len(selected)
        result.write_text(''.join(json.dumps(r)+'\n' for r in prefix+suffix))
    judged=[json.loads(l) for l in result.read_text().splitlines()]
    assert len(judged)==len(selected)
    with (run/f'{label}-metrics.txt').open('w') as out:
        subprocess.run([sys.executable,str(inputs/'print_qa_metrics.py'),str(result),str(inputs/'subset-50.json')],env=env,stdout=out,stderr=subprocess.STDOUT,check=True,timeout=30)
primary=[json.loads(l) for l in (run/'primary.jsonl.eval-results-gpt-4o').read_text().splitlines()]
audit=[json.loads(l) for l in (run/'audit-first-25.jsonl.eval-results-gpt-4o').read_text().splitlines()]
disagreements=sum(a['autoeval_label']['label']!=b['autoeval_label']['label'] for a,b in zip(primary[:25],audit))
failed_ids={r['id'] for r in report['rows'] if r.get('pipeline_state',r.get('answer_response',{}).get('state'))=='failed'}
failed_judgments=[{'question_id':r['question_id'],'official_label':r['autoeval_label']['label']} for r in primary if r['question_id'] in failed_ids]
(run/'judge-audit.json').write_text(json.dumps({'judge':'gpt-4o-2024-08-06','transport':'OpenRouter exact dated model; unmodified upstream prompt and yes substring grading','questions':50,'audit_questions':25,'disagreements':disagreements,'pipeline_states':report['pipeline_states'],'failed_pipeline_judgments':failed_judgments,'upstream_commit':manifest['upstream_commit'],'evaluator_sha256':manifest['sha256']['evaluate_qa.py']},indent=2)+'\n')
print(json.dumps({'run':str(run),'audit_disagreements':disagreements}))
