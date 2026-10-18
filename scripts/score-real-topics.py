#!/usr/bin/env python3
"""Offline real-candidate comparison. Never accepts gold-upper-bound receipts.

Rust owns canonical admission/native execution. This checks their scalar receipts,
complete pair/coverage denominators and exact label provenance; it never reads a
vector value, invokes a model or promotes a producer/recipe. Fresh held-out work
requires a separately frozen development selection before its corpus is opened.
"""
import argparse
from collections import Counter
import hashlib
from itertools import combinations, product
import json
import os
from pathlib import Path
import uuid

MODE = "actual_unpublished_candidates_with_retained_vectors"
SCHEMA = "recollect-real-organization-evaluation-1"
RECONCILIATION_PROMPT = "source-reconciliation-3"
ARM_DEFINITIONS = {
    "native_hybrid": "Shared hybrid native weighted Leiden plus exact eligible term facets",
    "native_weighted": "Entity IDF native weighted Leiden plus the same exact eligible term facets",
    "entity_overlap": "Exact eligible term facets only",
    "metadata": "Overlapping actual canonical repository, environment and area groups only",
    "eligible_vector": "Retained vector only native weighted Leiden; no entity facets; selected hybrid neighbors/cosine, uniform scale and no-entity median1",
}
GRID = tuple(product((3, 5, 8), (.65, .75, .85), (.25, .5, 1.0)))
OUTCOMES = {"assigned", "unassigned", "ineligible", "failed", "unsupported"}


def once(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError("Duplicate JSON field")
        value[key] = item
    return value


def read(path):
    return json.loads(path.read_bytes(), object_pairs_hook=once)


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()).hexdigest()


def identifier(value):
    try:
        return str(uuid.UUID(value))
    except (ValueError, TypeError, AttributeError) as error:
        raise ValueError("Missing or invalid native UUID") from error


def parameter_tuple(value):
    result = (value.get("neighbors"), value.get("minimum_cosine"), value.get("vector_scale"))
    if result not in GRID:
        raise ValueError("Parameters outside the predeclared27-grid")
    return result


def ratio(a, b):
    return a / b if b else None


def truth(a, b, annotations):
    if a["scope"].split("/")[0] != b["scope"].split("/")[0]:
        return False, "brain_boundary"
    explicit = annotations.get(tuple(sorted((a["id"], b["id"]))))
    if explicit is not None:
        return explicit, "explicit_annotation"
    if a["topics"] and b["topics"]:
        return bool(set(a["topics"]) & set(b["topics"])), "topic_categories"
    return None, "unannotated_pair"


def annotations(corpus):
    output = {}
    for a, b, value in corpus["topic_pairs"]:
        key = tuple(sorted((a, b)))
        if a == b or key in output:
            raise ValueError("Duplicate or invalid frozen pair")
        if value.startswith("related_"):
            output[key] = True
        elif value in ("unrelated", "unrelated_empty", "cross_Brain_no_membership_or_count"):
            output[key] = False
        else:
            raise ValueError("Unknown pair annotation")
    return output


def chunks(text):
    raw=text.encode(); start=0; result=[]
    while start<len(raw):
        end=min(start+4096,len(raw))
        while end<len(raw) and raw[end]&0xc0==0x80: end-=1
        if end<len(raw):
            newline=raw.rfind(b"\n",start,end)
            if newline-start>=2048: end=newline+1
        result.append((start,end,hashlib.sha256(raw[start:end]).hexdigest()));start=end
    return result


def sha_field(value):
    if not isinstance(value,str) or len(value)!=64 or any(c not in "0123456789abcdef" for c in value):
        raise ValueError("Exact lowercaseSHA256 binding required")
    return value


def sources_for(corpus, receipt):
    docs = {d["id"]: d for d in corpus["documents"]}
    if len(docs)!=len(corpus["documents"]): raise ValueError("Repeated frozen document")
    records, versions, brains, requests, entries_seen, source_ids, scopes = {}, {}, {}, set(), set(), set(), {}
    for source in receipt["sources"]:
        key = source["id"]
        if key not in docs or key in records:
            raise ValueError("Foreign or repeated canonical source")
        doc = docs[key]
        brain = identifier(source["brain_id"])
        version = identifier(source["source_version_id"])
        source_id=identifier(source["source_id"])
        if source_id in source_ids: raise ValueError("Distinct documents cannot reuse canonical source identity")
        source_ids.add(source_id)
        if source["title"]!=doc.get("title",f"Synthetic source {key}"): raise ValueError("Canonical title differs from declared setup")
        if version in versions or source["text_sha256"] != hashlib.sha256(doc["text"].encode()).hexdigest():
            raise ValueError("Duplicate version or changed immutable source text")
        symbolic_brain = doc["scope"].split("/")[0]
        if symbolic_brain in brains and brains[symbolic_brain] != brain:
            raise ValueError("Declared Brain was split into unrelated canonical contexts")
        if symbolic_brain not in brains and brain in brains.values():
            raise ValueError("Distinct declared Brains cannot share canonical identity")
        brains[symbolic_brain] = brain
        selection=source["canonical_selection"]
        repository_ids,area_ids=selection["repository_ids"],selection["area_ids"]
        if len(repository_ids)!=len(set(repository_ids)) or len(area_ids)!=len(set(area_ids)): raise ValueError("Repeated canonical scope identity")
        for reference in repository_ids+area_ids: identifier(reference)
        if selection["environment_id"] is not None: identifier(selection["environment_id"])
        parts=doc["scope"].split("/")
        if len(parts)!=3: raise ValueError("Frozen scope must declare Brain/repository/environment")
        for kind,symbol,actual in (("repository",parts[1],repository_ids),("environment",parts[2],[selection["environment_id"]] if selection["environment_id"] else [])):
            declared=(brain,kind,symbol)
            if symbol in ("unknown","none"):
                if actual: raise ValueError("Unknown scope cannot gain a canonical reference")
            else:
                if len(actual)!=1: raise ValueError("Declared scope requires one exact canonical reference")
                if declared in scopes and scopes[declared]!=actual[0]: raise ValueError("Declared scope was split across canonical identities")
                if any(k[:2]==declared[:2] and k!=declared and v==actual[0] for k,v in scopes.items()): raise ValueError("Distinct declared scopes cannot share canonical identity")
                scopes[declared]=actual[0]
        if area_ids: raise ValueError("This frozen campaign declares no area references")
        extraction = source["extraction"]
        request = extraction.get("request_id")
        if request is not None:
            request = identifier(request)
            if request in requests:
                raise ValueError("Distinct source attempts cannot reuse extraction request")
            requests.add(request)
            if extraction["prompt_label"] != RECONCILIATION_PROMPT or extraction["schema_label"] != RECONCILIATION_PROMPT:
                raise ValueError("A source-facts or judge request cannot qualify reconciliation features")
        expected_chunks=chunks(doc["text"])
        ordinals=set()
        for entry in source["vector_entries"]:
            if entry["entry_id"] in entries_seen or entry["ordinal"] in ordinals:
                raise ValueError("Duplicate vector entry/chunk provenance")
            entries_seen.add(entry["entry_id"]);ordinals.add(entry["ordinal"])
            n=entry["ordinal"]
            if type(n) is not int or not 0<=n<len(expected_chunks) or (entry["byte_start"],entry["byte_end"],entry["chunk_sha256"])!=expected_chunks[n] or entry["source_version_id"]!=version:
                raise ValueError("Vector entry differs from exact complete canonical chunk")
            if entry.get("embedding_sha256") is not None: sha_field(entry["embedding_sha256"])
            for field in ("entry_id", "profile_id", "source_chunk_id"):
                identifier(entry[field])
            if entry.get("request_id") is not None:
                identifier(entry["request_id"])
            if "embedding" in entry or "vector" in entry:
                raise ValueError("Vector values must remain in canonical PostgreSQL")
        records[key], versions[version] = source, key
    return docs, records, versions


def label_check(group, members, docs, records, versions):
    label, p = group["label"], group["label_provenance"]
    version = identifier(p["source_version_id"])
    key = versions.get(version)
    if not label or len(label.encode()) > 1024 or key not in members:
        return False
    kind = p["kind"]
    if kind in ("source_span", "exact_entity_mention"):
        a, b = p.get("byte_start"), p.get("byte_end")
        if type(a) is not int or type(b) is not int or not 0 <= a < b <= len(docs[key]["text"].encode()):
            return False
        try:
            return docs[key]["text"].encode()[a:b].decode() == p.get("quote") == label
        except UnicodeDecodeError:
            return False
    if p.get("byte_start") is not None or p.get("byte_end") is not None or p.get("quote") is not None:
        return False
    if kind == "canonical_title":
        return label == records[key]["title"]
    if kind == "canonical_scope":
        field, reference = p.get("field_kind"), p.get("field_id")
        scope = records[key]["canonical_selection"]
        ids = {"repository": scope["repository_ids"], "area": scope["area_ids"], "environment": [scope["environment_id"]]}
        return field in ids and reference in ids[field] and reference is not None
    return False


def score_arm(corpus, sources, arm, parameters=None):
    docs, records, versions = sources
    name = arm["name"]
    if name not in ARM_DEFINITIONS or arm["state"] not in ("completed", "failed", "not_measured"):
        raise ValueError("Unknown baseline or state")
    members, groups, reports = {}, {}, set()
    for result in arm.get("reports", []):
        report = identifier(result["report_id"])
        if report in reports or result["mode"] != name:
            raise ValueError("Duplicate native report or baseline mode changed")
        reports.add(report)
        brain = identifier(result["brain_id"])
        if parameters is not None:
            actual=parameter_tuple(result["parameters"])
            expected=parameter_tuple(parameters)
            if actual[:2]!=expected[:2] or (name!="eligible_vector" and actual[2]!=expected[2]):
                raise ValueError("Baseline parameters differ from frozen development selection")
        if any(g["kind"]=="native_community" for g in result["groups"]) and not result.get("gds_version"):
            raise ValueError("Native baseline requires actual GDS identity")
        vector_ids=set(result["vector_entry_ids"])
        if len(vector_ids)!=len(result["vector_entry_ids"]): raise ValueError("Repeated retained vector dependency")
        available_entries={entry["entry_id"]:entry for record in records.values() if record["brain_id"]==brain for entry in record["vector_entries"]}
        if any(identifier(entry) not in available_entries or available_entries[entry]["profile_id"]!=result.get("vector_profile_id") for entry in vector_ids): raise ValueError("Foreign retained vector dependency")
        local = {}
        for group in result["groups"]:
            if group["id"] in groups or not isinstance(group["id"], str) or not group["id"]:
                raise ValueError("Duplicate group identity")
            allowed={"native_hybrid":{"native_community","entity_facet"},"native_weighted":{"native_community","entity_facet"},"entity_overlap":{"entity_facet"},"metadata":{"canonical_scope"},"eligible_vector":{"native_community"}}[name]
            if group["kind"] not in allowed: raise ValueError("Group kind differs from declared comparison arm")
            if len(group["source_versions"])!=len(set(group["source_versions"])): raise ValueError("Repeated native group contributor")
            if name=="eligible_vector" and len(group["source_versions"])<2: raise ValueError("Vector isolates cannot become singleton topics")
            local[group["id"]] = group
            groups[group["id"]] = group
        for source in result["sources"]:
            version = identifier(source["source_version_id"])
            key = versions.get(version)
            if key is None or key in members or records[key]["brain_id"] != brain:
                raise ValueError("Native assignment not bound to exact canonical source/Brain")
            if source["state"] not in OUTCOMES or len(source["groups"]) != len(set(source["groups"])):
                raise ValueError("Invalid or repeated native assignment")
            if type(source["ner_feature_available"]) is not bool or type(source["vector_feature_available"]) is not bool: raise ValueError("Feature availability must be explicit booleans")
            if bool(source["groups"]) != (source["state"] == "assigned"):
                raise ValueError("Unavailable/unassigned source cannot retain groups")
            if source["ner_feature_available"]:
                identifier(source["candidate_id"])
                extraction = records[key]["extraction"]
                if extraction["state"] != "succeeded" or extraction["mapping_state"] != "validated" or source["request_id"] != extraction["request_id"]:
                    raise ValueError("Unbound or unsuccessful NER feature")
            if source["vector_feature_available"]:
                entries = records[key]["vector_entries"]
                if len(entries)!=len(chunks(docs[key]["text"])) or any(e["state"] != "ready" or e["truncated"] or not e["compatible"] or not e.get("request_id") or not e.get("embedding_sha256") or e["entry_id"] not in vector_ids or e["profile_id"]!=result.get("vector_profile_id") for e in entries):
                    raise ValueError("Missing/partial/incompatible vectors cannot become usable features")
            for gid in source["groups"]:
                if gid not in local or version not in local[gid]["source_versions"]:
                    raise ValueError("Assignment differs from native report membership")
                kind=local[gid]["kind"]
                if (kind=="entity_facet" or name=="native_weighted") and not source["ner_feature_available"]: raise ValueError("Entity grouping requires an actual NER feature")
                if name=="eligible_vector" and not source["vector_feature_available"]: raise ValueError("Vector grouping requires an actual retained vector feature")
                if name=="native_hybrid" and not (source["ner_feature_available"] or source["vector_feature_available"]): raise ValueError("Hybrid grouping requires an actual eligible feature")
            members[key] = source
        for group in local.values():
            declared = set(group["source_versions"])
            actual = {v for v, key in versions.items() if key in members and group["id"] in members[key]["groups"]}
            if not declared or declared != actual:
                raise ValueError("Group member set differs from complete native assignment")
    missing = sorted(set(docs) - set(members))
    for key in missing:
        members[key] = {"state": "failed", "groups": [], "ner_feature_available": False, "vector_feature_available": False}
    pairs, annotation = [], annotations(corpus)
    for a, b in combinations(sorted(docs), 2):
        expected, reason = truth(docs[a], docs[b], annotation)
        predicted = bool(set(members[a]["groups"]) & set(members[b]["groups"]))
        pairs.append({"left": a, "right": b, "expected_related": expected, "truth_source": reason, "predicted_related": predicted})
    measured = [p for p in pairs if p["expected_related"] is not None]
    tp = sum(p["predicted_related"] and p["expected_related"] for p in measured)
    fp = sum(p["predicted_related"] and not p["expected_related"] for p in measured)
    fn = sum(not p["predicted_related"] and p["expected_related"] for p in measured)
    annotated = [key for key, doc in docs.items() if doc["topics"]]
    assigned = sum(members[key]["state"] == "assigned" for key in annotated)
    multi_source=sum(any(len(groups[gid]["source_versions"])>=2 for gid in members[key]["groups"]) for key in annotated)
    label_rows = []
    for gid, group in groups.items():
        keys = {key for key, source in members.items() if gid in source["groups"]}
        kind=group["label_provenance"]["kind"]
        verified=label_check(group, keys, docs, records, versions)
        label_rows.append({"id": gid, "label": group["label"], "supported": None if kind=="canonical_scope" and verified else verified, "kind":kind,"verification_scope":"canonical_reference_origin_only" if kind=="canonical_scope" else "exact_label_text","origin_verified":verified if kind=="canonical_scope" else None,"label_text_independently_verified":None if kind=="canonical_scope" else verified})
    return {"name": name, "state": arm["state"], "documents": len(docs), "missing_document_outcomes": missing,
            "pair_metrics": {"pairs": len(pairs), "scored_pairs": len(measured), "tp": tp, "fp": fp, "fn": fn, "precision": ratio(tp, tp+fp), "recall": ratio(tp, tp+fn), "f1": ratio(2*tp, 2*tp+fp+fn), "cross_brain_shared_groups": sum(p["predicted_related"] and p["truth_source"] == "brain_boundary" for p in pairs)},
            "assignment": {"annotated_sources": len(annotated), "assigned_annotated": assigned, "coverage": ratio(assigned, len(annotated)), "multi_source_annotated":multi_source,"multi_source_coverage":ratio(multi_source,len(annotated)),"outcomes": dict(Counter(m["state"] for m in members.values())), "ner_available": sum(m["ner_feature_available"] for m in members.values()), "vector_available": sum(m["vector_feature_available"] for m in members.values())},
            "source_outcomes":[{"id":key,"state":members[key]["state"],"groups":members[key]["groups"],"ner_feature_available":members[key]["ner_feature_available"],"vector_feature_available":members[key]["vector_feature_available"],"declared_language":docs[key]["language"]} for key in sorted(docs)],"unsupported_language_assigned":[key for key in sorted(docs) if docs[key]["language"]!="en" and members[key]["state"]=="assigned"],
            "pairs": pairs, "labels": label_rows, "blind_semantic_usefulness": "manual_review_required", "quality_qualified": False}


def compare(corpus, receipt):
    if receipt.get("schema") != SCHEMA or receipt.get("input_mode") != MODE or receipt.get("arm_definitions") != ARM_DEFINITIONS:
        raise ValueError("Real-candidate schema/arm definitions required; gold receipts are incompatible")
    names = [arm["name"] for arm in receipt["baselines"]]
    expected = {"native_hybrid"} if receipt["phase"] == "development_grid" else set(ARM_DEFINITIONS)
    if set(names) != expected or len(names) != len(expected):
        raise ValueError("Every declared arm requires one outcome, including failed/unmeasured arms")
    if receipt["phase"] == "development_grid" and any(d["partition"] != "development" for d in corpus["documents"]):
        raise ValueError("Held-out inputs cannot enter development selection")
    if receipt["phase"] not in ("development_grid","diagnostic20","fresh12"): raise ValueError("Unknown frozen evaluation phase")
    recipe=sha_field(receipt["recipe_sha256"]); corpus_hash=sha_field(receipt["corpus_sha256"])
    parameter_tuple(receipt["parameters"])
    sources = sources_for(corpus, receipt)
    return {"input_mode": MODE, "phase": receipt["phase"], "source_cohort_sha256": digest(receipt["sources"]), "parameters": receipt["parameters"], "recipe_sha256":recipe,"corpus_sha256":corpus_hash,"scorer_sha256":hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),"baselines": [score_arm(corpus, sources, arm, receipt["parameters"]) for arm in receipt["baselines"]], "production_recipe_qualified": False, "native_authorization_proven_by_python": False}


def select_development(results):
    if len(results) != len(GRID):
        raise ValueError("Complete27-grid required, including failed parameter outcomes")
    parameters, cohort = set(), None
    eligible = []
    for result in results:
        if result["input_mode"] != MODE or result["phase"] != "development_grid":
            raise ValueError("Only real development outcomes may select parameters")
        p = parameter_tuple(result["parameters"])
        if p in parameters:
            raise ValueError("Repeated parameter trial")
        parameters.add(p)
        binding=(result["source_cohort_sha256"],result["recipe_sha256"],result["corpus_sha256"],result["scorer_sha256"])
        if cohort is not None and cohort != binding:
            raise ValueError("Development attempts changed canonical cohort/provenance")
        cohort = binding
        arm = result["baselines"][0]
        metric = arm["pair_metrics"]
        if arm["state"] == "completed" and metric["precision"] is not None and metric["precision"] >= .95:
            eligible.append((-(metric["f1"] or 0), -(arm["assignment"]["coverage"] or 0), p))
    chosen = sorted(eligible)[0][2] if eligible else None
    return {"schema": "recollect-real-development-selection-1", "input_mode": MODE, "development_source_cohort_sha256":cohort[0],"recipe_sha256":cohort[1],"development_corpus_sha256":cohort[2],"scorer_sha256":cohort[3], "trials": len(results), "selected": chosen is not None, "parameters": dict(zip(("neighbors", "minimum_cosine", "vector_scale"), chosen)) if chosen else None, "rule": "precision>=0.95; maximizeF1,coverage; lexicographically smallest tuple", "production_recipe_qualified": False}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus", type=Path)
    parser.add_argument("--receipt", type=Path)
    parser.add_argument("--expected-corpus-sha256")
    parser.add_argument("--select-grid", type=Path, nargs="+")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--selection", type=Path)
    args=parser.parse_args()
    def save(value):
        descriptor=os.open(args.output,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
        with os.fdopen(descriptor,"w") as file:
            json.dump(value,file,indent=2,ensure_ascii=False);file.write("\n");file.flush();os.fsync(file.fileno())
        directory=os.open(args.output.parent,os.O_RDONLY)
        try: os.fsync(directory)
        finally: os.close(directory)
    if args.select_grid:
        if args.corpus or args.receipt or args.selection or args.expected_corpus_sha256: parser.error("Development selection accepts only the complete scored development grid")
        results=[read(path) for path in args.select_grid]
        selection=select_development(results)
        selection["development_result_sha256"]=[hashlib.sha256(path.read_bytes()).hexdigest() for path in args.select_grid]
        save(selection)
        print(json.dumps({"development_trials":len(results),"selected":selection["selected"],"production_recipe_qualified":False}))
        return
    if not args.corpus or not args.receipt or not args.expected_corpus_sha256: parser.error("Corpus, receipt and exact corpus hash required")
    receipt=read(args.receipt)
    if receipt.get("phase") != "development_grid":
        if not args.selection: parser.error("Frozen development selection required before opening diagnostic or untouched held-out corpus")
        selection=read(args.selection)
        if not selection.get("selected") or selection.get("parameters") != receipt.get("parameters") or selection.get("recipe_sha256") != receipt.get("recipe_sha256") or selection.get("scorer_sha256") != hashlib.sha256(Path(__file__).read_bytes()).hexdigest() or receipt.get("development_selection_sha256") != hashlib.sha256(args.selection.read_bytes()).hexdigest():
            parser.error("Held-out receipt differs from frozen selected development parameters")
    expected="a01e61e95454d02aec7d4446530a13d16900dbe50550495c7be052f89bea3d67" if receipt.get("phase")=="fresh12" else "a0ecfe7dd692c2517f00035c0be7205841e27e7b17551f7a21580edc6d1c916c"
    if args.expected_corpus_sha256!=expected: parser.error("This phase requires its exact independently frozen corpus")
    raw=args.corpus.read_bytes()
    if hashlib.sha256(raw).hexdigest()!=args.expected_corpus_sha256 or receipt.get("corpus_sha256")!=args.expected_corpus_sha256:
        parser.error("Frozen corpus changed")
    corpus=json.loads(raw,object_pairs_hook=once)
    if receipt.get("phase")=="development_grid":
        corpus={**corpus,"documents":[d for d in corpus["documents"] if d["partition"]=="development"]}
    output=compare(corpus,receipt)
    output.update(receipt_sha256=hashlib.sha256(args.receipt.read_bytes()).hexdigest(),corpus_sha256=args.expected_corpus_sha256,scorer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
    save(output)
    print(json.dumps({"phase":output["phase"],"arms":len(output["baselines"]),"production_recipe_qualified":False}))


if __name__=="__main__": main()
