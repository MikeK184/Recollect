import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import uuid

spec=importlib.util.spec_from_file_location("real_topic_score",Path(__file__).parents[1]/"score-real-topics.py")
score=importlib.util.module_from_spec(spec);spec.loader.exec_module(score)

def uid():return str(uuid.uuid4())

def fixture():
    brain, repo, env=uid(),uid(),uid()
    docs=[{"id":f"d{n}","partition":"development","language":"en","scope":"alpha/infra/PROD","text":text,"topics":topics} for n,text,topics in [(1,"Rust runs here.",["systems"]),(2,"Rust works here.",["systems"]),(3,"Redis runs elsewhere.",["data"])]]
    corpus={"documents":docs,"topic_pairs":[]}
    sources=[]
    for doc in docs:
        sources.append({"id":doc["id"],"brain_id":brain,"source_id":uid(),"source_version_id":uid(),"title":f"Synthetic source {doc['id']}","text_sha256":hashlib.sha256(doc["text"].encode()).hexdigest(),"canonical_selection":{"repository_ids":[repo],"area_ids":[],"environment_id":env},"extraction":{"request_id":uid(),"state":"succeeded","mapping_state":"validated","prompt_label":"source-reconciliation-3","schema_label":"source-reconciliation-3"},"vector_entries":[]})
    groups=[{"id":"ner:rust","kind":"entity_facet","label":"Rust","label_provenance":{"kind":"exact_entity_mention","source_version_id":sources[0]["source_version_id"],"byte_start":0,"byte_end":4,"quote":"Rust","field_kind":None,"field_id":None},"source_versions":[s["source_version_id"] for s in sources[:2]]}]
    report={"report_id":uid(),"brain_id":brain,"mode":"native_hybrid","parameters":{"neighbors":5,"minimum_cosine":.75,"vector_scale":1},"gds_version":"controlled-unit-native-version","vector_profile_id":None,"vector_entry_ids":[],"groups":groups,"sources":[{"source_version_id":s["source_version_id"],"request_id":s["extraction"]["request_id"],"candidate_id":uid(),"state":"assigned" if n<2 else "unassigned","groups":["ner:rust"] if n<2 else [],"ner_feature_available":True,"vector_feature_available":False} for n,s in enumerate(sources)]}
    receipt={"schema":score.SCHEMA,"input_mode":score.MODE,"phase":"development_grid","recipe_sha256":"a"*64,"corpus_sha256":"b"*64,"parameters":report["parameters"],"arm_definitions":score.ARM_DEFINITIONS,"sources":sources,"baselines":[{"name":"native_hybrid","state":"completed","reports":[report]}]}
    return corpus,receipt

class RealTopicTests(unittest.TestCase):
    def test_legacy_or_mixed_producer_labels_cannot_qualify_corrected_campaign(self):
        for prompt, schema in (("source-reconciliation-2", "source-reconciliation-2"),
                               ("source-reconciliation-3", "source-reconciliation-2"),
                               ("source-facts-3", "source-facts-3")):
            corpus, receipt = fixture()
            receipt["sources"][0]["extraction"].update(prompt_label=prompt, schema_label=schema)
            with self.subTest(prompt=prompt, schema=schema), self.assertRaises(ValueError):
                score.compare(corpus, receipt)
    def test_bound_real_reports_keep_all_pairs_and_missing_failure_denominators(self):
        corpus,receipt=fixture(); result=score.compare(corpus,receipt)["baselines"][0]
        self.assertEqual(result["pair_metrics"]["pairs"],3)
        self.assertEqual(result["pair_metrics"]["tp"],1)
        self.assertEqual(result["pair_metrics"]["fp"],0)
        self.assertEqual(result["assignment"]["coverage"],2/3)
        self.assertFalse(result["quality_qualified"])
        receipt["baselines"][0]["reports"]=[];receipt["baselines"][0]["state"]="failed"
        result=score.compare(corpus,receipt)["baselines"][0]
        self.assertEqual(result["documents"],3)
        self.assertEqual(result["pair_metrics"]["fn"],1)
        self.assertEqual(result["assignment"]["outcomes"],{"failed":3})

    def test_gold_mode_unknown_arm_and_foreign_source_are_rejected(self):
        corpus,receipt=fixture();receipt["input_mode"]="gold_upper_bound"
        with self.assertRaises(ValueError):score.compare(corpus,receipt)
        corpus,receipt=fixture();receipt["baselines"][0]["name"]="optimistic"
        with self.assertRaises(ValueError):score.compare(corpus,receipt)
        corpus,receipt=fixture();receipt["baselines"][0]["reports"][0]["sources"][0]["source_version_id"]=uid()
        with self.assertRaises(ValueError):score.compare(corpus,receipt)

    def test_unoffered_failed_ner_and_raw_vector_second_store_are_rejected(self):
        corpus,receipt=fixture();receipt["sources"][0]["extraction"]["state"]="failed"
        with self.assertRaises(ValueError):score.compare(corpus,receipt)
        corpus,receipt=fixture();receipt["sources"][0]["vector_entries"]=[{"embedding":[.1],"entry_id":uid(),"ordinal":0,"byte_start":0,"byte_end":len(corpus["documents"][0]["text"]),"chunk_sha256":receipt["sources"][0]["text_sha256"],"source_version_id":receipt["sources"][0]["source_version_id"],"profile_id":uid(),"source_chunk_id":uid()}]
        with self.assertRaisesRegex(ValueError,"Vector values"):score.compare(corpus,receipt)

    def test_missing_second_chunk_cannot_be_called_available(self):
        corpus,receipt=fixture();text="Rust "+"x"*5000
        corpus["documents"][0]["text"]=text
        source=receipt["sources"][0];source["text_sha256"]=hashlib.sha256(text.encode()).hexdigest()
        profile,entry,request=uid(),uid(),uid();a,b,h=score.chunks(text)[0]
        source["vector_entries"]=[{"entry_id":entry,"profile_id":profile,"source_chunk_id":uid(),"source_version_id":source["source_version_id"],"request_id":request,"ordinal":0,"byte_start":a,"byte_end":b,"chunk_sha256":h,"state":"ready","truncated":False,"compatible":True,"embedding_sha256":"c"*64}]
        report=receipt["baselines"][0]["reports"][0];report["sources"][0]["vector_feature_available"]=True;report["vector_profile_id"]=profile;report["vector_entry_ids"]=[entry]
        with self.assertRaisesRegex(ValueError,"partial"):score.compare(corpus,receipt)

    def test_forged_memberships_without_the_actual_arm_feature_fail_closed(self):
        corpus,receipt=fixture()
        receipt["baselines"][0]["reports"][0]["sources"][0]["ner_feature_available"]=False
        with self.assertRaisesRegex(ValueError,"NER feature"):score.compare(corpus,receipt)
        corpus,receipt=fixture()
        for arm in receipt["baselines"]:
            arm["name"]="eligible_vector"
            arm["reports"][0]["mode"]="eligible_vector"
            arm["reports"][0]["groups"][0]["kind"]="native_community"
        docs=score.sources_for(corpus,receipt)
        with self.assertRaisesRegex(ValueError,"retained vector feature"):score.score_arm(corpus,docs,receipt["baselines"][0])

    def test_scope_title_type_and_arm_tampering_cannot_change_the_frozen_setup(self):
        for change in ("title","scope","boolean","arm","duplicate"):
            corpus,receipt=fixture()
            if change=="title":receipt["sources"][0]["title"]="Fabricated useful group"
            if change=="scope":receipt["sources"][1]["canonical_selection"]["environment_id"]=uid()
            if change=="boolean":receipt["baselines"][0]["reports"][0]["sources"][0]["ner_feature_available"]="false"
            if change=="arm":receipt["baselines"][0]["reports"][0]["groups"][0]["kind"]="canonical_scope"
            if change=="duplicate":receipt["baselines"][0]["reports"][0]["groups"][0]["source_versions"]*=2
            with self.subTest(change=change),self.assertRaises(ValueError):score.compare(corpus,receipt)

    def test_native_title_and_field_labels_do_not_fabricate_utf8_spans(self):
        corpus,receipt=fixture();group=receipt["baselines"][0]["reports"][0]["groups"][0]
        group["label"]=receipt["sources"][0]["title"];group["label_provenance"]={"kind":"canonical_title","source_version_id":receipt["sources"][0]["source_version_id"],"byte_start":None,"byte_end":None,"quote":None}
        self.assertTrue(score.compare(corpus,receipt)["baselines"][0]["labels"][0]["supported"])
        group["label_provenance"]["byte_start"]=0
        self.assertFalse(score.compare(corpus,receipt)["baselines"][0]["labels"][0]["supported"])
        group["label_provenance"]={"kind":"canonical_scope","source_version_id":receipt["sources"][0]["source_version_id"],"field_kind":"repository","field_id":uid()}
        self.assertFalse(score.compare(corpus,receipt)["baselines"][0]["labels"][0]["supported"])
        group["label_provenance"]["field_id"]=receipt["sources"][0]["canonical_selection"]["repository_ids"][0]
        label=score.compare(corpus,receipt)["baselines"][0]["labels"][0]
        self.assertTrue(label["origin_verified"])
        self.assertIsNone(label["supported"])
        self.assertIsNone(label["label_text_independently_verified"])

    def test_complete_grid_selects_only_precision_floor_and_fixed_tie_rule(self):
        corpus,receipt=fixture();results=[]
        for k,c,s in score.GRID:
            r=copy.deepcopy(receipt);r["parameters"]={"neighbors":k,"minimum_cosine":c,"vector_scale":s};r["baselines"][0]["reports"][0]["parameters"]=r["parameters"]
            results.append(score.compare(corpus,r))
        chosen=score.select_development(results)
        self.assertEqual(chosen["parameters"],{"neighbors":3,"minimum_cosine":.65,"vector_scale":.25})
        results[0]["baselines"][0]["pair_metrics"]["precision"]=.94
        self.assertEqual(score.select_development(results)["parameters"]["vector_scale"],.5)
        for result in results:result["baselines"][0]["pair_metrics"]["precision"]=.94
        self.assertFalse(score.select_development(results)["selected"])
        with self.assertRaises(ValueError):score.select_development(results[:-1])

    def test_grid_cannot_swap_canonical_cohort_or_open_heldout(self):
        corpus,receipt=fixture();corpus["documents"][0]["partition"]="held_out"
        with self.assertRaisesRegex(ValueError,"Held-out"):score.compare(corpus,receipt)
        corpus,receipt=fixture();results=[]
        for k,c,s in score.GRID:
            r=copy.deepcopy(receipt);r["parameters"]={"neighbors":k,"minimum_cosine":c,"vector_scale":s};r["baselines"][0]["reports"][0]["parameters"]=r["parameters"]
            results.append(score.compare(corpus,r))
        results[-1]["source_cohort_sha256"]="d"*64
        with self.assertRaisesRegex(ValueError,"changed canonical"):score.select_development(results)
        with tempfile.TemporaryDirectory() as temp:
            p=Path(temp);(p/"receipt.json").write_text(json.dumps({"phase":"fresh12"}))
            result=subprocess.run([sys.executable,str(Path(score.__file__)),"--corpus",str(p/"must-not-be-opened.json"),"--receipt",str(p/"receipt.json"),"--expected-corpus-sha256","unused","--output",str(p/"out.json")],capture_output=True,text=True)
            self.assertEqual(result.returncode,2)
            self.assertIn("before opening",result.stderr)
            self.assertNotIn("FileNotFoundError",result.stderr)

    def test_selection_cli_is_complete_private_and_cannot_overwrite_a_frozen_choice(self):
        corpus,receipt=fixture()
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);paths=[]
            for n,(k,c,s) in enumerate(score.GRID):
                row=copy.deepcopy(receipt);row["parameters"]={"neighbors":k,"minimum_cosine":c,"vector_scale":s};row["baselines"][0]["reports"][0]["parameters"]=row["parameters"]
                path=root/f"trial-{n}.json";path.write_text(json.dumps(score.compare(corpus,row)));paths.append(str(path))
            output=root/"selection.json"
            command=[sys.executable,str(Path(score.__file__)),"--select-grid",*paths,"--output",str(output)]
            result=subprocess.run(command,capture_output=True,text=True)
            self.assertEqual(result.returncode,0,result.stderr)
            frozen=output.read_bytes();selection=json.loads(frozen)
            self.assertTrue(selection["selected"]);self.assertEqual(len(selection["development_result_sha256"]),27)
            self.assertEqual(output.stat().st_mode&0o777,0o600)
            self.assertNotEqual(subprocess.run(command,capture_output=True,text=True).returncode,0)
            self.assertEqual(output.read_bytes(),frozen)

if __name__=="__main__":unittest.main()
