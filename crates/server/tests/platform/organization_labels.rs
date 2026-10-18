//! Independent synthetic label/path regression; no extractor quality claim.
use super::*;

async fn page(
    h: &Harness,
    owner: &Login,
    base: &str,
    scope: &Value,
    center: Value,
    cursor: Value,
    limit: usize,
) -> Value {
    ok(
        h,
        "POST",
        &format!("{base}/organization/view"),
        owner,
        json!({"scope":scope,"center":center,"cursor":cursor,"limit":limit}),
    )
    .await
}
async fn sources(
    h: &Harness,
    owner: &Login,
    base: &str,
    scope: &Value,
    topic: Option<Uuid>,
) -> Value {
    ok(
        h,
        "POST",
        &format!("{base}/organization/sources/view"),
        owner,
        json!({"scope":scope,"topic_id":topic}),
    )
    .await
}
fn card(view: &Value, topic: Uuid) -> &Value {
    view["topics"]
        .as_array()
        .unwrap()
        .iter()
        .chain(
            view.get("context_cards")
                .and_then(Value::as_array)
                .into_iter()
                .flatten(),
        )
        .find(|c| c["id"] == json!(topic))
        .unwrap_or_else(|| panic!("Missing topic {topic}: {view}"))
}
async fn parity(
    h: &Harness,
    owner: &Login,
    base: &str,
    scope: &Value,
    topic: Uuid,
    label: &str,
) -> (Value, Value) {
    let main = page(
        h,
        owner,
        base,
        scope,
        json!({"kind":"topic","id":topic}),
        Value::Null,
        50,
    )
    .await;
    let source = sources(h, owner, base, scope, Some(topic)).await;
    assert_eq!(card(&main, topic)["label"], label, "{main}");
    assert_eq!(card(&source, topic), card(&main, topic));
    (main, source)
}

async fn split_fixture(
    h: &Harness,
    brain: Uuid,
    docs: &[Value],
    inputs: &[Uuid],
    topic: Uuid,
) -> Uuid {
    let change = Uuid::new_v4();
    sqlx::query("INSERT INTO knowledge_topics(id,brain_id,kind,change_id,recipe) VALUES($1,$2,'split',$3,'human-split-1')").bind(topic).bind(brain).bind(change).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_changes(id,brain_id,topic_id,actor_id,action) SELECT $1,$2,$3,owner_id,'create' FROM brains WHERE id=$2").bind(change).bind(brain).bind(topic).execute(&h.admin).await.unwrap();
    let versions: Vec<Uuid> = docs
        .iter()
        .map(|d| d["version"]["id"].as_str().unwrap().parse().unwrap())
        .collect();
    sqlx::query("INSERT INTO knowledge_topic_sources(brain_id,topic_id,input_id,reason,weight) SELECT brain_id,$2,id,'explicit_source_split',1 FROM knowledge_source_inputs WHERE brain_id=$1 AND source_version_id=ANY($3)").bind(brain).bind(topic).bind(versions).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_memberships(brain_id,topic_id,entity_id,input_id,reason,weight) SELECT DISTINCT brain_id,$2,entity_id,input_id,'explicit_split',1 FROM knowledge_mentions WHERE brain_id=$1 AND input_id=ANY($3)").bind(brain).bind(topic).bind(inputs).execute(&h.admin).await.unwrap();
    topic
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL/Neo4j/GDS and controlled loopback; synthetic label conformance only"]
async fn organization_labels_qualified_native_paths_scope_overrides_and_bounds() {
    let (h, owner, base, p, server) = setup().await;
    let brain: Uuid = base.rsplit('/').next().unwrap().parse().unwrap();
    allow(&h, &owner, &base, |policy| {
        policy["purposes"] = json!(["extraction", "embedding"]);
        policy["automatic_embedding"] = json!(false);
        policy["automatic_learning"] = json!(false);
    })
    .await;
    let settings = ok(
        &h,
        "GET",
        &format!("{base}/models/policy"),
        &owner,
        Value::Null,
    )
    .await;
    let mut policy = settings["current"]["policy"].clone();
    policy["embedding_dimensions"] = json!(8);
    ok(&h,"PUT",&format!("{base}/models/policy"),&owner,json!({"base_change":settings["current"]["change_id"],"policy":policy,"rebuild_embeddings":true})).await;
    *p.candidates.lock().unwrap() = json!({"claims":[]});
    let area = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"area","name":"Synthetic label scope"}),
    )
    .await;
    let other_area = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"area","name":"Other synthetic label scope"}),
    )
    .await;
    let mut docs = vec![];
    let mut inputs = vec![];
    let mut profile = None;
    for (i, text) in [
        "AlphaIncidental AlphaIncidental AlphaIncidental ZetaCommon BetaShared ZuluCenter ScopeName.",
        "ZetaCommon BetaShared ZuluCenter ScopeName.",
        "ZetaCommon ZuluCenter ScopeName.",
        "Source only fallback document A.",
        "Source only fallback document B.",
    ]
    .iter()
    .enumerate()
    {
        let doc=ok(&h,"POST",&format!("{base}/sources"),&owner,json!({"title":format!("Synthetic canonical title {i}"),"media_type":"text/plain","retain_content":true,"content":text})).await;
        while worker::run_once(&h.state, "capture").await.unwrap() {}
        if i < 3 {
            ok(
                &h,
                "PUT",
                &format!("{base}/sources/{}/groups", doc["id"].as_str().unwrap()),
                &owner,
                json!({"group_ids":[if i==0 { &area["id"] } else { &other_area["id"] }]}),
            )
            .await;
        }
        if i < 3 {
            let mut mentions = vec![];
            for term in ["AlphaIncidental", "ZetaCommon", "BetaShared", "ZuluCenter"] {
                for (start, _) in text.match_indices(term) {
                    mentions.push(json!({"key":format!("m{}",mentions.len()),"kind":"technology","span":{"byte_start":start,"byte_end":start+term.len(),"quote":term},"confidence":1.0}));
                }
            }
            let start = text.find("ScopeName").unwrap();
            mentions.push(json!({"key":"scoped","kind":"service","span":{"byte_start":start,"byte_end":start+9,"quote":"ScopeName"},"confidence":1.0}));
            let payload = json!({"schema":"source-mentions-1","adapter_revision":"synthetic-organization-plumbing-1","source_version_id":doc["version"]["id"],"language":"en","mentions":mentions,"relations":[],"aliases":[]});
            let input = organization::staged(&h, &owner, &base, &doc, &payload).await;
            organization::synthetic_publish(&h, &owner, &base, &doc, input, &mut profile).await;
            inputs.push(input);
        }
        docs.push(doc);
    }
    // Deliberately put incidental UUIDs before the repeated winner. UUID order
    // is unrelated to semantic support and must not select the label.
    let mentions: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id,quote FROM knowledge_mentions WHERE brain_id=$1 ORDER BY input_id,local_key",
    )
    .bind(brain)
    .fetch_all(&h.admin)
    .await
    .unwrap();
    for (n, (id, quote)) in mentions.iter().enumerate() {
        let replacement = Uuid::from_u128(if quote == "AlphaIncidental" {
            n as u128 + 1
        } else {
            u128::MAX - n as u128
        });
        sqlx::query("UPDATE knowledge_mentions SET id=$2 WHERE id=$1")
            .bind(id)
            .bind(replacement)
            .execute(&h.admin)
            .await
            .unwrap();
    }
    let vectors = organization_hybrid::retained_profile(&h, brain).await;
    for (i, doc) in docs.iter().enumerate() {
        let version = doc["version"]["id"].as_str().unwrap().parse().unwrap();
        organization_hybrid::retained(
            &h,
            brain,
            vectors,
            version,
            if i < 3 {
                &[1., 0., 0., 0., 0., 0., 0., 0.]
            } else {
                &[0., 1., 0., 0., 0., 0., 0., 0.]
            },
        )
        .await;
    }
    organization_hybrid::select(&h, brain).await;
    let report = organization_hybrid::calculate(&h, &h.state, brain).await;
    let full = json!({"kind":"knowledge"});
    // A fixed explicit split supplies the exact three-source selector cohort;
    // the unchanged native calculation determines its own separate groups.
    let topic = split_fixture(&h, brain, &docs[..3], &inputs, Uuid::from_u128(1)).await;
    let (main, source) = parity(&h, &owner, &base, &full, topic, "ZetaCommon").await;
    assert_eq!(card(&main, topic)["source_count"], 3);
    assert_eq!(
        source["representative_sources"][0]["provenance_kind"],
        "eligible_source_term"
    );
    let winner = docs[..3]
        .iter()
        .map(|d| {
            d["version"]["id"]
                .as_str()
                .unwrap()
                .parse::<Uuid>()
                .unwrap()
        })
        .min()
        .unwrap();
    assert_eq!(
        source["representative_sources"][0]["source_key"],
        format!("source_version:{winner}")
    );
    assert!(
        main["representative_mentions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["quote"] == "ZetaCommon"
                && m["source_key"] == format!("source_version:{winner}"))
    );
    let center_entity: Uuid = sqlx::query_scalar(
        "SELECT entity_id FROM knowledge_mentions WHERE brain_id=$1 AND quote='ZuluCenter' LIMIT 1",
    )
    .bind(brain)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    let centered = page(
        &h,
        &owner,
        &base,
        &full,
        json!({"kind":"entity","id":center_entity}),
        Value::Null,
        50,
    )
    .await;
    assert_eq!(card(&centered, topic), card(&main, topic));
    let narrow = json!({"kind":"knowledge","selection":{"area_ids":[area["id"]]}});
    parity(&h, &owner, &base, &narrow, topic, "ScopeName").await;

    // Keep the actual native result unchanged. Whichever real native group
    // contains document C has ZetaCommon/ZuluCenter in every contributor;
    // incidental terms occur on fewer members, and scoped services remain
    // ineligible community labels. No prechosen partition is required.
    let native:Uuid=sqlx::query_scalar("SELECT t.id FROM knowledge_topics t WHERE t.brain_id=$1 AND t.kind='community' AND EXISTS(SELECT 1 FROM knowledge_topic_sources m JOIN knowledge_source_inputs i ON i.brain_id=m.brain_id AND i.id=m.input_id WHERE m.topic_id=t.id AND i.source_version_id=$2)")
        .bind(brain).bind(docs[2]["version"]["id"].as_str().unwrap().parse::<Uuid>().unwrap()).fetch_one(&h.admin).await.unwrap();
    let (native_main, native_sources) =
        parity(&h, &owner, &base, &full, native, "ZetaCommon").await;
    assert_eq!(
        native_sources["representative_sources"][0]["provenance_kind"],
        "eligible_source_term"
    );
    let native_versions:Vec<Uuid>=sqlx::query_scalar("SELECT i.source_version_id FROM knowledge_topic_sources m JOIN knowledge_source_inputs i ON i.brain_id=m.brain_id AND i.id=m.input_id WHERE m.topic_id=$1").bind(native).fetch_all(&h.admin).await.unwrap();
    let native_winner = format!(
        "source_version:{}",
        native_versions.into_iter().min().unwrap()
    );
    assert_eq!(
        native_sources["representative_sources"][0]["source_key"],
        native_winner
    );
    assert!(
        native_main["representative_mentions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["quote"] == "ZetaCommon" && m["source_key"] == native_winner)
    );

    // A facet keeps its anchor even when another term has wider support.
    let facet:Uuid=sqlx::query_scalar("SELECT t.id FROM knowledge_topics t JOIN knowledge_entities e ON e.id=t.anchor_entity_id WHERE t.brain_id=$1 AND e.normalized_label='alphaincidental' AND t.kind='facet'")
        .bind(brain).fetch_one(&h.admin).await.unwrap();
    let anchored = page(
        &h,
        &owner,
        &base,
        &full,
        json!({"kind":"topic","id":facet}),
        Value::Null,
        50,
    )
    .await;
    assert_eq!(card(&anchored, facet)["label"], "AlphaIncidental");
    assert!(
        sources(&h, &owner, &base, &full, Some(facet)).await["topics"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    // Same-surface services retain three occurrence-scoped identities. A
    // reducer pooling them would tie at three sources and incorrectly beat
    // the technology term by kind. Split compatibility still admits services.
    let scoped_count:i64=sqlx::query_scalar("SELECT count(DISTINCT entity_id) FROM knowledge_mentions WHERE brain_id=$1 AND quote='ScopeName'").bind(brain).fetch_one(&h.admin).await.unwrap();
    assert_eq!(scoped_count, 3);
    let split = split_fixture(&h, brain, &docs[..3], &inputs, Uuid::new_v4()).await;
    parity(&h, &owner, &base, &full, split, "ZetaCommon").await;
    sqlx::query("DELETE FROM knowledge_topic_memberships m USING knowledge_entities e WHERE m.topic_id=$1 AND e.id=m.entity_id AND e.kind<>'service'").bind(split).execute(&h.admin).await.unwrap();
    parity(&h, &owner, &base, &full, split, "ScopeName").await;
    // Source-only native group uses the minimum qualified version's field,
    // independent of canonical source-input UUID order.
    let fallback_versions: Vec<Uuid> = docs[3..]
        .iter()
        .map(|d| d["version"]["id"].as_str().unwrap().parse().unwrap())
        .collect();
    let fallback:Uuid=sqlx::query_scalar("SELECT t.id FROM knowledge_topics t JOIN knowledge_topic_sources m ON m.brain_id=t.brain_id AND m.topic_id=t.id JOIN knowledge_source_inputs i ON i.brain_id=m.brain_id AND i.id=m.input_id WHERE t.brain_id=$1 AND t.kind='community' GROUP BY t.id HAVING count(*)=2 AND bool_and(i.source_version_id=ANY($2))")
        .bind(brain).bind(&fallback_versions).fetch_one(&h.admin).await.unwrap();
    let fallback_doc = docs[3..]
        .iter()
        .min_by_key(|d| {
            d["version"]["id"]
                .as_str()
                .unwrap()
                .parse::<Uuid>()
                .unwrap()
        })
        .unwrap();
    let (_, fallback_sources) = parity(
        &h,
        &owner,
        &base,
        &full,
        fallback,
        fallback_doc["version"]["title"].as_str().unwrap(),
    )
    .await;
    assert_eq!(
        fallback_sources["representative_sources"][0]["provenance_kind"],
        "canonical_source_title"
    );

    // A legacy mapping name and then an authenticated source name use their
    // genuine current bases; narrower scopes cannot borrow either basis.
    sqlx::query("INSERT INTO knowledge_topic_labels(brain_id,topic_id,label,basis_input,actor_id) SELECT $1,$2,'Reviewed mapping name',$3,owner_id FROM brains WHERE id=$1")
        .bind(brain).bind(topic).bind(inputs[1]).execute(&h.admin).await.unwrap();
    let (renamed, renamed_sources) =
        parity(&h, &owner, &base, &full, topic, "Reviewed mapping name").await;
    assert_eq!(
        renamed_sources["representative_sources"][0]["provenance_kind"],
        "authenticated_mapping_label"
    );
    parity(&h, &owner, &base, &narrow, topic, "ScopeName").await;
    let centered = page(
        &h,
        &owner,
        &base,
        &full,
        json!({"kind":"entity","id":center_entity}),
        Value::Null,
        50,
    )
    .await;
    assert_eq!(card(&centered, topic)["label"], "Reviewed mapping name");
    let source_basis: Uuid = sqlx::query_scalar(
        "SELECT id FROM knowledge_source_inputs WHERE brain_id=$1 AND source_version_id=$2",
    )
    .bind(brain)
    .bind(
        docs[0]["version"]["id"]
            .as_str()
            .unwrap()
            .parse::<Uuid>()
            .unwrap(),
    )
    .fetch_one(&h.admin)
    .await
    .unwrap();
    let changed=h.keyed("PUT",&format!("{base}/organization/topics/{topic}"),Some(&owner),json!({"base_change_id":card(&renamed,topic)["change_id"],"change":{"action":"rename_source","label":"Reviewed source name","basis_input":source_basis}}),Some("label-source-rename")).await;
    assert_eq!(changed.0, StatusCode::OK, "{}", changed.1);
    let (renamed, _) = parity(&h, &owner, &base, &full, topic, "Reviewed source name").await;
    let centered = page(
        &h,
        &owner,
        &base,
        &full,
        json!({"kind":"entity","id":center_entity}),
        Value::Null,
        50,
    )
    .await;
    assert_eq!(card(&centered, topic), card(&renamed, topic));
    assert!(
        centered["source_memberships"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(centered["source_next_cursor"].is_null());
    assert!(
        centered["representative_sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["topic_id"] == json!(topic)
                && r["provenance_kind"] == "authenticated_source_label"
                && r["input_id"] == json!(source_basis)
                && r["source_key"]
                    == format!(
                        "source_version:{}",
                        docs[0]["version"]["id"].as_str().unwrap()
                    ))
    );
    // A source-human context card keeps its canonical label basis when all
    // 80 detail slots are consumed by other typed rows. These cloned, exact
    // source spans are only a bounded pagination load fixture.
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM knowledge_mentions WHERE brain_id=$1")
            .bind(brain)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    sqlx::query("INSERT INTO knowledge_mentions(id,brain_id,input_id,entity_id,local_key,byte_start,byte_end,quote,confidence) SELECT gen_random_uuid(),v.brain_id,v.input_id,v.entity_id,'context_fixture_'||n,v.byte_start,v.byte_end,v.quote,1 FROM (SELECT * FROM knowledge_mentions WHERE input_id=$1 AND quote='AlphaIncidental' LIMIT 1)v CROSS JOIN generate_series(1,$2::int)n").bind(inputs[0]).bind(79-count).execute(&h.admin).await.unwrap();
    sqlx::query("UPDATE knowledge_topics t SET pinned=true FROM knowledge_entities e WHERE t.brain_id=$1 AND t.anchor_entity_id=e.id AND t.kind='facet' AND e.normalized_label='zetacommon'").bind(brain).execute(&h.admin).await.unwrap();
    let contextual = page(&h, &owner, &base, &full, Value::Null, Value::Null, 1).await;
    assert!(
        contextual["topics"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["id"] != json!(topic))
    );
    assert!(
        contextual["context_cards"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == json!(topic) && c["label"] == "Reviewed source name")
    );
    assert_eq!(
        contextual["mentions"].as_array().unwrap().len()
            + contextual["memberships"].as_array().unwrap().len()
            + contextual["connections"].as_array().unwrap().len(),
        80
    );
    assert!(
        contextual["source_memberships"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        contextual["representative_sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["topic_id"] == json!(topic)
                && r["input_id"] == json!(source_basis)
                && r["provenance_kind"] == "authenticated_source_label")
    );
    let excluded=h.keyed("PUT",&format!("{base}/organization/topics/{topic}"),Some(&owner),json!({"base_change_id":card(&renamed,topic)["change_id"],"change":{"action":"exclude_sources","input_ids":[source_basis],"excluded":true}}),Some("label-source-exclude")).await;
    assert_eq!(excluded.0, StatusCode::OK, "{}", excluded.1);
    parity(&h, &owner, &base, &full, topic, "ZetaCommon").await;
    let centered = page(
        &h,
        &owner,
        &base,
        &full,
        json!({"kind":"entity","id":center_entity}),
        Value::Null,
        50,
    )
    .await;
    assert_eq!(card(&centered, topic)["label"], "ZetaCommon");
    assert!(
        centered["representative_sources"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["topic_id"] != json!(topic) || r["input_id"] != json!(source_basis))
    );
    assert!(
        sources(&h, &owner, &base, &narrow, Some(topic)).await["topics"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    // Small card pages must not change a context card's full-window label.
    let mut cursor = Value::Null;
    let mut seen_context = false;
    for _ in 0..20 {
        let v = page(&h, &owner, &base, &full, Value::Null, cursor, 1).await;
        for c in v["context_cards"].as_array().unwrap() {
            if c["id"] == json!(topic) {
                assert_eq!(c["label"], "ZetaCommon");
                seen_context = true;
            }
        }
        let rows = v["mentions"].as_array().unwrap().len()
            + v["memberships"].as_array().unwrap().len()
            + v["connections"].as_array().unwrap().len();
        assert!(rows <= 80);
        if v["next_cursor"].is_null() {
            break;
        }
        cursor = v["next_cursor"].clone();
    }
    assert!(seen_context, "The real context path must be exercised");
    // Preserve the earliest report deadline, including nonrepresentative
    // groups, and remove expired communities from both independent APIs.
    let expiry = chrono::Utc::now() + chrono::Duration::seconds(30);
    sqlx::query("UPDATE analytics_reports SET expires_at=$2 WHERE id=$1")
        .bind(report)
        .bind(expiry)
        .execute(&h.admin)
        .await
        .unwrap();
    let (timed, timed_sources) = parity(
        &h,
        &owner,
        &base,
        &full,
        fallback,
        fallback_doc["version"]["title"].as_str().unwrap(),
    )
    .await;
    assert_eq!(timed["expires_at"], timed_sources["expires_at"]);
    sqlx::query(
        "UPDATE analytics_reports SET expires_at=clock_timestamp()-interval '1 second' WHERE id=$1",
    )
    .bind(report)
    .execute(&h.admin)
    .await
    .unwrap();
    assert!(
        sources(&h, &owner, &base, &full, Some(fallback)).await["topics"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let expired = page(
        &h,
        &owner,
        &base,
        &full,
        json!({"kind":"topic","id":fallback}),
        Value::Null,
        50,
    )
    .await;
    assert!(expired["topics"].as_array().unwrap().is_empty());
    sqlx::query("UPDATE analytics_reports SET expires_at=NULL WHERE id=$1")
        .bind(report)
        .execute(&h.admin)
        .await
        .unwrap();

    // Explicitly synthetic load rows exercise the complete stream's +1
    // rejection. They are not producer output or a quality fixture.
    let entity: Uuid = sqlx::query_scalar(
        "SELECT entity_id FROM knowledge_mentions WHERE input_id=$1 AND quote='ZetaCommon' LIMIT 1",
    )
    .bind(inputs[0])
    .fetch_one(&h.admin)
    .await
    .unwrap();
    sqlx::query("INSERT INTO knowledge_mentions(id,brain_id,input_id,entity_id,local_key,byte_start,byte_end,quote,confidence) SELECT gen_random_uuid(),$1,$2,$3,'load_'||n,v.byte_start,v.byte_end,v.quote,1 FROM (SELECT byte_start,byte_end,quote FROM knowledge_mentions WHERE input_id=$2 AND entity_id=$3 LIMIT 1)v CROSS JOIN generate_series(1,50001)n")
        .bind(brain).bind(inputs[0]).bind(entity).execute(&h.admin).await.unwrap();
    for endpoint in ["organization/view", "organization/sources/view"] {
        let response = h
            .call(
                "POST",
                &format!("{base}/{endpoint}"),
                Some(&owner),
                if endpoint == "organization/view" {
                    json!({"scope":full,"limit":50})
                } else {
                    json!({"scope":full})
                },
            )
            .await;
        assert_eq!(
            response.0,
            StatusCode::BAD_REQUEST,
            "{endpoint}: {}",
            response.1
        );
        assert_eq!(
            response.1["message"],
            "The complete topic label window exceeds its bounded envelope."
        );
    }
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        3,
        "Only three controlled local synthetic publication requests"
    );
    server.abort();
    h.finish().await;
}
