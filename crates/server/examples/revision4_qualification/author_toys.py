"""Independent source-only campaign toys; never opens a prior corpus or outcome."""
import hashlib, itertools, json, os, sys
from pathlib import Path
root=Path(sys.argv[1]); folder=Path(sys.argv[2]); spec=Path(sys.argv[3])
sys.path.insert(0,str(root/'scripts'))
import frozen_source_corpus as loader

def encoded(v):return (json.dumps(v,ensure_ascii=False,sort_keys=True,separators=(',',':'))+'\n').encode()
def sha(b):return hashlib.sha256(b).hexdigest()
def put(path,v):
 b=encoded(v)
 with path.open('xb') as f:os.fchmod(f.fileno(),0o600);f.write(b);f.flush();os.fsync(f.fileno())
 return sha(b)
def span(text,quote,n=1):
 p=-1
 for _ in range(n):p=text.index(quote,p+1)
 return [len(text[:p].encode()),len(text[:p].encode())+len(quote.encode()),quote]
fixtures=json.loads(spec.read_bytes()); result={}
for cohort,sources in fixtures.items():
 docs=[]
 for source in sources:
  t=source['text'];d={'id':source['document_id'],'title':source['title'],'partition':'held_out','language':source['language'],'scope':source['scope'],'text':t,'mentions':[],'aliases':[],'relations':[],'topics':[],'metadata':{'topic_annotation_state':'unknown' if source['document_id'].endswith('_missing') else 'complete'}}
  if 'Orion' in t:
   d['mentions'].append(['o','service',*span(t,'Orion')]);d['topics']=['orion_literal']
  docs.append(d)
 corpus={'schema':'independent-revision4-campaign-toys-1','metadata':{'purpose':'native plumbing only'},'documents':docs,'identity_controls':[],'topic_pairs':[],'topic_pair_metadata':[]}
 for a,b in itertools.combinations(docs,2):
  sa,sb=a['scope'].split('/'),b['scope'].split('/'); shared=sorted(set(a['topics'])&set(b['topics']))
  disp='cross_Brain_no_membership_or_count' if sa[0]!=sb[0] else 'unrelated_empty' if not a['text'] or not b['text'] else 'unknown_topic_coverage' if 'unknown' in [a['metadata']['topic_annotation_state'],b['metadata']['topic_annotation_state']] else 'related_independent_literal' if shared else 'unrelated'
  unknown=disp=='unknown_topic_coverage';corpus['topic_pairs'].append([a['id'],b['id'],disp]);corpus['topic_pair_metadata'].append({'left':a['id'],'right':b['id'],'disposition':disp,'reason':'Independent literal toy annotation','shared_topics':shared,'cross_brain_boundary':sa[0]!=sb[0],'same_repository_scope_token':sa[1]==sb[1],'same_environment_scope_token':sa[2]==sb[2],'label_usefulness':'NOT_MEASURED','unsupported_language_in_pair':a['language']!='en' or b['language']!='en','lifecycle_scenario_note':None,'topic_truth_status':'unknown' if unknown else 'known','scored_topic_positive_or_negative':not unknown,'topic_coverage_reason':'Explicit incomplete toy annotation' if unknown else None})
 interface={'schema':loader.INTERFACE_SCHEMA,'corpus_schema_version':corpus['schema'],'top_level_required_keys':loader.TOP_LEVEL,'Document':loader.DOCUMENT_DECLARATIONS,**loader.RECORD_FIELDS,'enums':{k:sorted(v)for k,v in [('Kind',loader.KINDS),('AliasDisposition',loader.ALIAS_DISPOSITIONS),('RelationDisposition',loader.RELATION_DISPOSITIONS),('RelationKind',loader.RELATIONS),('TopicPairDisposition',loader.TOPIC_DISPOSITIONS),('IdentityDisposition',loader.IDENTITY_DISPOSITIONS)]},'source_hash_manifest':{'schema':'string','documents':'map document ID -> lowercase SHA256 hex of exact UTF-8 text'},'topic_coverage_state':{'location':'documents[].metadata.topic_annotation_state','enum':['complete','unknown']},'unknown_pair_semantics':{'disposition':'unknown_topic_coverage','truth':None}}
 manifest={'schema':loader.MANIFEST_SCHEMA,'corpus_schema':corpus['schema']}
 for key,v in {'corpus':corpus,'guide':{'rule':'Literal authored toys; not a producer quality evaluation.'},'source_hashes':{'schema':'independent-campaign-toy-hashes-1','documents':{d['id']:sha(d['text'].encode())for d in docs}},'interface':interface}.items():
  p=folder/f'{cohort}-{key}.json';manifest[key+'_path']=str(p.relative_to(root));manifest[key+'_sha256']=put(p,v)
 p=folder/f'{cohort}-manifest.json';h=put(p,manifest);loaded=loader.load_frozen_corpus(p,manifest_sha256=h);result[cohort]={'manifest':str(p.relative_to(root)),'sha256':h,'corpus_bindings':loaded.file_sha256,'readiness':loaded.summary()['complete_topic_gold_readiness']}
put(folder/'toy-bindings.json',result)
