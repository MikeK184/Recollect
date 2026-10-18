//! Explicit unpublished evaluation. No HTTP admission, automatic experiment,
//! producer qualification, learning success or canonical semantic publication.
use super::*;
use crate::{
    knowledge_mapping::{
        self as mapping,
        affinity::{self, Association, Projection, Vertex},
        source_features::{self, HybridParameters, RetainedFeatures},
    },
    model_gateway::{self as gateway, Context},
    model_policy,
};
use sqlx::FromRow;
mod groups;
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    NativeHybrid,
    NativeWeighted,
    EntityOverlap,
    Metadata,
    EligibleVector,
}
impl Mode {
    fn vectors(self) -> bool {
        matches!(self, Self::NativeHybrid | Self::EligibleVector)
    }
    fn native(self) -> bool {
        matches!(
            self,
            Self::NativeHybrid | Self::NativeWeighted | Self::EligibleVector
        )
    }
    fn terms(self) -> bool {
        !matches!(self, Self::Metadata | Self::EligibleVector)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ShadowSource {
    pub source_version_id: Uuid,
    pub candidate_id: Option<Uuid>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LabelProvenance {
    pub kind: String,
    pub source_version_id: Uuid,
    pub byte_start: Option<usize>,
    pub byte_end: Option<usize>,
    pub quote: Option<String>,
    pub field_kind: Option<String>,
    pub field_id: Option<Uuid>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Group {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub label_provenance: LabelProvenance,
    pub source_versions: Vec<Uuid>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SourceOutcome {
    pub source_version_id: Uuid,
    pub candidate_id: Option<Uuid>,
    pub request_id: Option<Uuid>,
    pub state: String,
    pub ner_feature_available: bool,
    pub vector_feature_available: bool,
    pub groups: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ShadowResult {
    pub report_id: Uuid,
    pub brain_id: Uuid,
    pub mode: Mode,
    pub parameters: HybridParameters,
    pub gds_version: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub sources: Vec<SourceOutcome>,
    pub groups: Vec<Group>,
    pub vector_profile_id: Option<Uuid>,
    pub vector_entry_ids: Vec<Uuid>,
    pub vector_coverage: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct Source {
    pub id: Uuid,
    pub source_id: Uuid,
    pub version: Uuid,
    pub candidate: Option<Uuid>,
    pub context: Value,
    pub label: String,
    pub request: Option<Uuid>,
    #[serde(skip, default)]
    pub payload: Option<mapping::SourceCandidates>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct Prepared {
    epoch: i64,
    policy: Uuid,
    sources: Vec<Source>,
    vectors: RetainedFeatures,
    projection: Projection,
    mode: Mode,
    parameters: HybridParameters,
    gds_version: String,
    metadata_groups: Vec<Group>,
    deadline: DateTime<Utc>,
}
fn invalid() -> Error {
    failure(
        "shadow_input_invalid",
        "The exact unpublished candidate or retained source provenance is unavailable.",
    )
}
async fn bounds(tx: &mut Tx<'_>, cx: Context) -> Result<DateTime<Utc>> {
    let deadline:DateTime<Utc>=sqlx::query_scalar("SELECT least(clock_timestamp()+interval '7 days',CASE WHEN a.auth_kind<>'local' THEN a.membership_until END,d.expires_at) FROM accounts a LEFT JOIN devices d ON d.id=$2 AND d.account_id=a.id WHERE a.id=$1 AND a.enabled").bind(cx.actor).bind(cx.device).fetch_one(&mut **tx).await?;
    Ok(deadline)
}
async fn settings(tx: &mut Tx<'_>) -> Result<()> {
    sqlx::query("SET LOCAL statement_timeout='1500ms'")
        .execute(&mut **tx)
        .await?;
    sqlx::query("SET LOCAL jit=off").execute(&mut **tx).await?;
    sqlx::query("SET LOCAL recollect.bulk_qualification=on")
        .execute(&mut **tx)
        .await?;
    Ok(())
}
async fn canonical(
    state: &AppState,
    tx: &mut Tx<'_>,
    evaluator: &Auth,
    cx: Context,
    versions: &[Uuid],
) -> Result<(BTreeMap<Uuid, (Uuid, String, Value)>, Option<DateTime<Utc>>)> {
    let mut scope = GraphSelection {
        kind: "knowledge".into(),
        ..Default::default()
    };
    crate::retrieval::graph::authorize(tx, evaluator, cx.brain, &scope).await?;
    let rows: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT id,source_id FROM source_versions WHERE brain_id=$1 AND id=ANY($2) ORDER BY id",
    )
    .bind(cx.brain)
    .bind(versions)
    .fetch_all(&mut **tx)
    .await?;
    if rows.len() != versions.len() {
        return Err(invalid());
    }
    let entities: Vec<_> = rows
        .iter()
        .map(|(v, s)| Entity {
            kind: "source_version".into(),
            id: *s,
            revision_id: *v,
        })
        .collect();
    let qualified = crate::retrieval::graph::qualify(
        state,
        tx,
        evaluator,
        cx.brain,
        &mut scope,
        &entities,
        (250, None),
    )
    .await?;
    let mut out = BTreeMap::new();
    for (v, s) in rows {
        let node = qualified
            .nodes
            .get(&key("source_version", v))
            .ok_or_else(invalid)?;
        source_features::canonical_gate(tx, cx.brain, v).await?;
        let context = source_features::canonical_context(tx, cx.brain, v, s).await?;
        out.insert(v, (s, node.evidence.label.clone(), context));
    }
    Ok((out, qualified.deadline))
}
#[derive(FromRow)]
struct Request {
    operation_id: Uuid,
    policy_id: Uuid,
    actor_id: Uuid,
    device_id: Option<Uuid>,
    provider: String,
    model: String,
    returned_model: Option<String>,
    prompt_label: String,
    schema_label: String,
    state: String,
    suppressed: bool,
    detail_expired: bool,
    mapping_output_sha256: Option<String>,
    mapping_source_version_id: Option<Uuid>,
    deadline: Option<DateTime<Utc>>,
    offered: Json<Value>,
}
async fn request(
    state: &AppState,
    tx: &mut Tx<'_>,
    cx: Context,
    id: Uuid,
    version: Uuid,
    qualify_offered: bool,
) -> Result<Request> {
    let q:Request=sqlx::query_as("SELECT q.operation_id,q.policy_id,q.actor_id,q.device_id,q.provider,q.model,q.returned_model,q.prompt_label,q.schema_label,q.state,q.suppressed,q.detail_expired,q.mapping_output_sha256,q.mapping_source_version_id,recollect_retention_deadline(q.brain_id,'audit',q.created_at) deadline,(SELECT coalesce(jsonb_agg(jsonb_build_object('kind',i.kind,'id',i.input_id) ORDER BY i.kind,i.input_id),'[]'::jsonb) FROM model_request_inputs i WHERE i.brain_id=q.brain_id AND i.request_id=q.id) offered FROM model_requests q WHERE q.brain_id=$1 AND q.id=$2 AND q.purpose='extraction'")
  .bind(cx.brain).bind(id).fetch_optional(&mut **tx).await?.ok_or_else(invalid)?;
    let current:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM accounts a WHERE a.id=$1 AND a.enabled AND (a.auth_kind='local' OR a.membership_until>clock_timestamp())) AND recollect_account_role($2,$1) IN('writer','admin') AND($3::uuid IS NULL OR EXISTS(SELECT 1 FROM devices d WHERE d.id=$3 AND d.account_id=$1 AND d.claimed AND d.revoked_at IS NULL AND d.expires_at>clock_timestamp()))").bind(cx.actor).bind(cx.brain).bind(cx.device).fetch_one(&mut **tx).await?;
    if !current {
        return Err(invalid());
    }
    if q.actor_id != cx.actor
        || q.device_id != cx.device
        || q.state != "succeeded"
        || q.suppressed
        || q.detail_expired
        || q.mapping_source_version_id != Some(version)
        || q.mapping_output_sha256.is_none()
        || q.deadline.is_some_and(|d| d <= Utc::now())
    {
        return Err(invalid());
    }
    let policy:Option<Json<recollect_protocol::ModelPolicy>>=sqlx::query_scalar("SELECT p.policy FROM model_policies p JOIN model_policy_heads h ON h.brain_id=p.brain_id AND h.policy_id=p.id WHERE p.brain_id=$1 AND p.id=$2").bind(cx.brain).bind(q.policy_id).fetch_optional(&mut **tx).await?;
    let Some(Json(policy)) = policy else {
        return Err(invalid());
    };
    if !policy.enabled
        || !policy.purposes.iter().any(|p| p == "extraction")
        || q.provider != policy.provider
        || q.model != policy.text_model
        || !model_policy::catalogue::returned_model_matches(
            &q.provider,
            &q.model,
            q.returned_model.as_deref().unwrap_or_default(),
            false,
        )
    {
        return Err(invalid());
    }
    if mapping::production::mapping_adapter(&q.prompt_label, &q.schema_label).is_none() {
        return Err(invalid());
    }
    let refs: Vec<gateway::InputRef> =
        serde_json::from_value(q.offered.0.clone()).map_err(|_| invalid())?;
    let claims: Vec<_> = refs
        .iter()
        .filter(|r| r.kind == "claim_revision")
        .map(|r| r.id)
        .collect();
    if q.prompt_label.starts_with("source-facts-") && !claims.is_empty() {
        return Err(invalid());
    }
    if qualify_offered
        && !claims.is_empty()
        && !crate::autonomous::shadow_offered_current(
            state,
            tx,
            cx.brain,
            cx.actor,
            cx.device,
            q.operation_id,
            version,
            &claims,
        )
        .await?
    {
        return Err(invalid());
    }
    let producer = bounds(tx, cx).await?;
    let mut q = q;
    q.deadline = [q.deadline, Some(producer)].into_iter().flatten().min();
    Ok(q)
}

// Short publication locks protect exact producer/request authority across audit.
async fn lock_producer(tx: &mut Tx<'_>, cx: Context, request: Uuid) -> Result<()> {
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR SHARE")
        .bind(cx.actor)
        .execute(&mut **tx)
        .await?;
    if let Some(device) = cx.device {
        sqlx::query("SELECT id FROM devices WHERE id=$1 FOR SHARE")
            .bind(device)
            .execute(&mut **tx)
            .await?;
    }
    sqlx::query("SELECT id FROM model_requests WHERE brain_id=$1 AND id=$2 FOR SHARE")
        .bind(cx.brain)
        .bind(request)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn fresh_offered(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
    offered: &Value,
) -> Result<Option<DateTime<Utc>>> {
    let refs: Vec<gateway::InputRef> =
        serde_json::from_value(offered.clone()).map_err(|_| invalid())?;
    if refs.is_empty() || refs.len() > 1000 {
        return Err(invalid());
    }
    for r in refs {
        match r.kind.as_str() {
            "source_version" => {
                source_features::canonical_gate(tx, brain, r.id).await?;
                gateway::source(state, tx, brain, r.id, crate::artifacts::MAX_TEXT).await?;
            }
            "claim_revision" => {
                let revision:Option<Json<recollect_protocol::ClaimRevision>>=sqlx::query_scalar("SELECT r.revision FROM claims c JOIN claim_revisions r ON r.brain_id=c.brain_id AND r.id=c.current_revision WHERE c.brain_id=$1 AND r.id=$2 AND recollect_content_state(r.brain_id,'claim',r.privacy_state,r.recorded_at)='active' AND NOT EXISTS(SELECT 1 FROM model_claim_fences f WHERE f.brain_id=r.brain_id AND f.revision_id=r.id)").bind(brain).bind(r.id).fetch_optional(&mut **tx).await?;
                let Some(Json(revision)) = revision else {
                    return Err(invalid());
                };
                let view = crate::memory::view(state, tx, revision, Utc::now(), None).await?;
                if !view.eligibility.investigation
                    || !view.eligibility.rule_ids.is_empty()
                    || view.contributions.iter().any(|c| c.revision.is_none())
                {
                    return Err(invalid());
                }
            }
            "repository_fact" => {
                let row = crate::memory_evidence::row(tx, brain, "repository_fact", r.id).await?;
                if row.privacy() != "active" {
                    return Err(invalid());
                }
            }
            _ => return Err(invalid()),
        }
    }
    // Use the same exact memory dependency deadline as canonical claim reads;
    // primary-source qualification alone does not bound earlier offered memory.
    let until: Option<DateTime<Utc>> = sqlx::query_scalar("WITH refs AS MATERIALIZED(SELECT * FROM jsonb_to_recordset($2) x(kind text,id uuid)), deadlines AS(SELECT recollect_retention_deadline(v.brain_id,v.retention_class,v.created_at) d FROM refs r JOIN source_versions v ON v.brain_id=$1 AND v.id=r.id WHERE r.kind='source_version' UNION ALL SELECT recollect_memory_deadline(c.brain_id,c.id) FROM refs r JOIN claim_revisions c ON c.brain_id=$1 AND c.id=r.id WHERE r.kind='claim_revision' UNION ALL SELECT recollect_retention_deadline(p.brain_id,'repository',p.created_at) FROM refs r JOIN repository_facts f ON f.brain_id=$1 AND f.id=r.id JOIN repository_snapshots p ON p.brain_id=f.brain_id AND p.id=f.snapshot_id WHERE r.kind='repository_fact') SELECT min(d) FROM deadlines").bind(brain).bind(Json(offered)).fetch_one(&mut **tx).await?;
    Ok(until)
}

/// Admits a real returned mapping using the gateway's immutable input0 binding
/// and PostgreSQL JSONB digest. Missing/invalid NER uses queue's candidate=None.
pub async fn admit(
    state: &AppState,
    evaluator: &Auth,
    cx: Context,
    request_id: Uuid,
    version: Uuid,
    mapping_value: &Value,
) -> Result<Uuid> {
    tokio::time::timeout(Duration::from_secs(5),async{
  if !mapping_value.is_object()||serde_json::to_vec(mapping_value).map_err(|_|invalid())?.len()>262144{return Err(invalid());}
  evaluator.require_browser()?;
  let mut tx=evaluator.preparation_tx(&state.pool).await?;settings(&mut tx).await?;let epoch=analytics_epoch(&mut tx,cx.brain).await?;
  let q=request(state,&mut tx,cx,request_id,version,true).await?;let offered_deadline=fresh_offered(state,&mut tx,cx.brain,&q.offered.0).await?;let(canonical,qualified_deadline)=canonical(state,&mut tx,evaluator,cx,&[version]).await?;
  let(source,_,context)=canonical.get(&version).cloned().ok_or_else(invalid)?;
  let raw=gateway::source(state,&mut tx,cx.brain,version,crate::artifacts::MAX_TEXT).await?;
  let policy=model_policy::current(state,&mut tx,cx.brain).await?;
  let mut classes=vec![raw.class.clone()];if let Some(c)=raw.provenance["original_content_class"].as_str(){classes.push(c.into());}
  if classes.iter().any(|c|!policy.policy.content_classes.contains(c)){return Err(invalid());}
  let payload=mapping::production::adapt_for_request(version,&raw.text,mapping_value,&q.prompt_label,&q.schema_label).map_err(|_|invalid())?;
  let digest:String=sqlx::query_scalar("SELECT encode(sha256(convert_to($1::jsonb::text,'UTF8')),'hex')").bind(Json(mapping_value)).fetch_one(&mut *tx).await?;
  if q.mapping_output_sha256.as_deref()!=Some(digest.as_str()){return Err(invalid());}
  let evaluator_context=Context{brain:cx.brain,actor:evaluator.user.id,device:evaluator.device_id};let authority=bounds(&mut tx,evaluator_context).await?;let expires=[Some(authority),qualified_deadline,q.deadline,offered_deadline].into_iter().flatten().min().ok_or_else(invalid)?;

  tx.commit().await?;
  tokio::time::timeout(Duration::from_millis(1500),async{
   let mut tx=evaluator.publication_tx(&state.pool).await?;sqlx::query("SET LOCAL statement_timeout='750ms'").execute(&mut *tx).await?;sqlx::query("SET LOCAL lock_timeout='250ms'").execute(&mut *tx).await?;db::require_writer(&mut tx,cx.brain).await?;
   if analytics_epoch(&mut tx,cx.brain).await?!=epoch{return Err(changed());}deadline(&mut tx,Some(expires)).await?;
   lock_producer(&mut tx,cx,request_id).await?;let live=request(state,&mut tx,cx,request_id,version,false).await?;if live.prompt_label!=q.prompt_label||live.schema_label!=q.schema_label||mapping::production::mapping_adapter(&live.prompt_label,&live.schema_label)!=Some(payload.adapter_revision.as_str())||live.mapping_output_sha256.as_deref()!=Some(&digest)||live.offered.0!=q.offered.0{return Err(invalid());}
   let existing:Option<(Uuid,Json<Value>,Json<mapping::SourceCandidates>,String,DateTime<Utc>,String,String)>=sqlx::query_as("SELECT id,context,payload,output_sha256,expires_at,schema_revision,adapter_revision FROM knowledge_shadow_candidates WHERE brain_id=$1 AND request_id=$2").bind(cx.brain).bind(request_id).fetch_optional(&mut *tx).await?;
   if let Some((id,Json(old_context),Json(old_payload),old_digest,old_expiry,old_schema,old_adapter))=existing {if old_schema!=payload.schema||old_adapter!=payload.adapter_revision||old_expiry<=Utc::now()||old_context!=context||old_payload!=payload||old_digest!=digest{return Err(invalid());}deadline(&mut tx,Some(expires.min(old_expiry))).await?;tx.commit().await?;return Ok(id);}
   let count:i64=sqlx::query_scalar("SELECT count(*) FROM knowledge_shadow_candidates WHERE brain_id=$1").bind(cx.brain).fetch_one(&mut *tx).await?;
   if count>=250{return Err(failure("shadow_capacity","This Brain has250 retained experimental candidates; expire owned experiments before admitting more."));}
   let id=Uuid::new_v4();
   sqlx::query("INSERT INTO knowledge_shadow_candidates(id,brain_id,source_id,source_version_id,request_id,policy_id,actor_id,device_id,context,schema_revision,adapter_revision,output_sha256,offered_inputs,payload,expires_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)")
    .bind(id).bind(cx.brain).bind(source).bind(version).bind(request_id).bind(q.policy_id).bind(cx.actor).bind(cx.device).bind(Json(context)).bind(mapping::SOURCE_SCHEMA).bind(payload.adapter_revision.clone()).bind(&digest).bind(q.offered).bind(Json(payload)).bind(expires).execute(&mut *tx).await?;
   db::audit(&mut tx,evaluator.user.id,cx.brain,"organization.shadow.admit",id,"unpublished").await?;let live=request(state,&mut tx,cx,request_id,version,false).await?;if live.prompt_label!=q.prompt_label||live.schema_label!=q.schema_label||analytics_epoch(&mut tx,cx.brain).await?!=epoch||live.mapping_output_sha256.as_deref()!=Some(&digest){return Err(changed());}deadline(&mut tx,Some(expires)).await?;tx.commit().await?;Ok(id)
  }).await.map_err(|_|failure("analytics_publication_timeout","Shadow candidate admission exceeded its bounded writer window."))?
 }).await.map_err(|_|failure("analytics_preparation_timeout","Shadow candidate preparation exceeded its bounded window."))?
}

async fn prepare(
    state: &AppState,
    evaluator: &Auth,
    cx: Context,
    mut sources: Vec<Source>,
    mode: Mode,
    parameters: HybridParameters,
    gds_version: &str,
) -> Result<Prepared> {
    tokio::time::timeout(Duration::from_secs(5),async{
  if sources.is_empty()||sources.len()>250||!parameters.valid()||gds_version.is_empty()||gds_version.len()>64||sources.iter().map(|s|s.version).collect::<BTreeSet<_>>().len()!=sources.len(){return Err(invalid());}
  sources.sort_by_key(|s|s.version);
  let mut tx=db::preparation_tx(&state.pool,cx.actor,cx.device).await?;settings(&mut tx).await?;
  if !matches!(db::require_role(&mut tx,cx.brain,false).await?.as_str(),"writer"|"admin"){return Err(Error::forbidden());}
  let epoch=analytics_epoch(&mut tx,cx.brain).await?;let policy=model_policy::current(state,&mut tx,cx.brain).await?;
  let versions:Vec<_>=sources.iter().map(|s|s.version).collect();let(current,source_deadline)=canonical(state,&mut tx,evaluator,cx,&versions).await?;
  let mut expiry=[Some(bounds(&mut tx,cx).await?),source_deadline].into_iter().flatten().min().ok_or_else(invalid)?;
  for s in &mut sources{
   let(source,label,context)=current.get(&s.version).cloned().ok_or_else(invalid)?;
   if s.source_id!=Uuid::nil()&&(s.source_id!=source||s.context!=context){return Err(changed());}
   s.source_id=source;s.label=label;s.context=context;s.payload=None;s.request=None;
   if let Some(candidate)=s.candidate{
    let row:Option<(Uuid,Uuid,Json<Value>,Json<mapping::SourceCandidates>,String,Json<Value>,DateTime<Utc>,Uuid,Option<Uuid>,String,String)>=sqlx::query_as("SELECT request_id,source_version_id,context,payload,output_sha256,offered_inputs,expires_at,actor_id,device_id,schema_revision,adapter_revision FROM knowledge_shadow_candidates WHERE brain_id=$1 AND id=$2").bind(cx.brain).bind(candidate).fetch_optional(&mut *tx).await?;
    let Some((qid,version,Json(context),Json(payload),hash,Json(offered),until,producer,device,schema,adapter))=row else{return Err(invalid());};
    if version!=s.version||context!=s.context||until<=Utc::now()||schema!=mapping::SOURCE_SCHEMA||schema!=payload.schema||adapter!=payload.adapter_revision{return Err(invalid());}
    let q=request(state,&mut tx,Context{brain:cx.brain,actor:producer,device},qid,version,true).await?;let offered_deadline=fresh_offered(state,&mut tx,cx.brain,&q.offered.0).await?;if mapping::production::mapping_adapter(&q.prompt_label,&q.schema_label)!=Some(adapter.as_str())||q.mapping_output_sha256.as_deref()!=Some(&hash)||q.offered.0!=offered{return Err(invalid());}
    let raw=gateway::source(state,&mut tx,cx.brain,version,crate::artifacts::MAX_TEXT).await?;
    let bytes=serde_json::to_vec(&payload).map_err(|_|invalid())?;
    let checked=mapping::source_candidates(version,&raw.text,&bytes).map_err(|_|invalid())?;
    s.payload=Some(checked);s.request=Some(qid);expiry=[Some(expiry),Some(until),q.deadline,offered_deadline].into_iter().flatten().min().ok_or_else(invalid)?;
   }
  }
  let vectors=if mode.vectors(){source_features::retained_for_versions(state,&mut tx,cx.brain,&versions).await?}else{RetainedFeatures{profile:None,entries:vec![],available:BTreeSet::new(),coverage:"disabled_mode".into(),budget:false}};
  if !vectors.entries.is_empty(){let audit:Option<DateTime<Utc>>=sqlx::query_scalar("SELECT min(recollect_retention_deadline(brain_id,'audit',created_at)) FROM model_requests WHERE brain_id=$1 AND id=ANY($2)").bind(cx.brain).bind(vectors.entries.iter().map(|e|e.request).collect::<Vec<_>>()).fetch_one(&mut *tx).await?;expiry=[Some(expiry),audit].into_iter().flatten().min().ok_or_else(invalid)?;}
  let associations=associations(&sources,mode);
  let inputs:Vec<_>=sources.iter().map(|s|(s.id,s.version)).collect();
  let(zero,edges)=if mode.vectors(){source_features::cosine_edges(&mut tx,cx.brain,&vectors.entries.iter().map(|e|e.entry).collect::<Vec<_>>(),parameters).await?}else{(false,vec![])};
  let mut vectors=vectors;if zero{vectors.available.clear();vectors.coverage="zero_centroid".into();}
  let projection=match mode{Mode::NativeHybrid|Mode::EligibleVector=>source_features::project(&inputs,&associations,&edges,parameters)?,Mode::NativeWeighted=>affinity::prepare(&inputs,&associations).map_err(|_|invalid())?,_=>affinity::prepare(&[],&[]).map_err(|_|invalid())?};
  let metadata_groups=if mode==Mode::Metadata{metadata(&mut tx,cx.brain,&sources).await?}else{vec![]};
  deadline(&mut tx,Some(expiry)).await?;tx.commit().await?;
  Ok(Prepared{epoch,policy:policy.change_id,sources,vectors,projection,mode,parameters,gds_version:gds_version.into(),metadata_groups,deadline:expiry})
 }).await.map_err(|_|failure("analytics_preparation_timeout","Shadow exact-cohort preparation exceeded its bounded window."))?
}
fn term_keys(sources: &[Source]) -> BTreeSet<(String, String)> {
    sources
        .iter()
        .filter_map(|s| s.payload.as_ref())
        .flat_map(|p| p.mentions.iter())
        .filter(|m| {
            matches!(
                m.kind,
                mapping::EntityKind::Technology | mapping::EntityKind::Concept
            )
        })
        .map(|m| {
            (
                m.kind.as_str().into(),
                m.kind.normalized_label(&m.span.quote),
            )
        })
        .collect()
}
fn associations(sources: &[Source], mode: Mode) -> Vec<Association> {
    if !mode.terms() {
        return vec![];
    }
    let terms: BTreeMap<_, _> = term_keys(sources)
        .into_iter()
        .enumerate()
        .map(|(i, k)| (k, Uuid::from_u128(i as u128 + 1)))
        .collect();
    let mut out = vec![];
    for s in sources {
        if let Some(p) = &s.payload {
            for m in &p.mentions {
                let key = (
                    m.kind.as_str().into(),
                    m.kind.normalized_label(&m.span.quote),
                );
                if let Some(id) = terms.get(&key) {
                    out.push(Association {
                        input_id: s.id,
                        source_version_id: s.version,
                        entity_id: *id,
                        kind: m.kind.clone(),
                        boilerplate: false,
                    });
                }
            }
        }
    }
    out
}

async fn metadata(tx: &mut Tx<'_>, brain: Uuid, sources: &[Source]) -> Result<Vec<Group>> {
    let mut refs = BTreeMap::<(String, Uuid), BTreeSet<Uuid>>::new();
    for source in sources {
        for (kind, values) in [
            (
                "repository",
                source
                    .context
                    .pointer("/selection/repository_ids")
                    .and_then(Value::as_array),
            ),
            (
                "environment",
                source.context.get("environments").and_then(Value::as_array),
            ),
            (
                "area",
                source.context.get("areas").and_then(Value::as_array),
            ),
        ] {
            if let Some(values) = values {
                for value in values {
                    let id = value
                        .as_str()
                        .and_then(|v| Uuid::parse_str(v).ok())
                        .ok_or_else(invalid)?;
                    if id.is_nil() {
                        return Err(invalid());
                    }
                    refs.entry((kind.into(), id))
                        .or_default()
                        .insert(source.version);
                }
            }
        }
        if let Some(value) = source
            .context
            .pointer("/selection/environment_id")
            .filter(|v| !v.is_null())
        {
            let id = value
                .as_str()
                .and_then(|v| Uuid::parse_str(v).ok())
                .ok_or_else(invalid)?;
            refs.entry(("environment".into(), id))
                .or_default()
                .insert(source.version);
        }
    }
    let mut groups = vec![];
    for ((kind, id), members) in refs {
        let label: Option<String> = if kind == "repository" {
            sqlx::query_scalar(
                "SELECT canonical_origin FROM repositories WHERE brain_id=$1 AND id=$2",
            )
            .bind(brain)
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?
        } else {
            sqlx::query_scalar(
                "SELECT name FROM evidence_groups WHERE brain_id=$1 AND id=$2 AND kind=$3",
            )
            .bind(brain)
            .bind(id)
            .bind(&kind)
            .fetch_optional(&mut **tx)
            .await?
        };
        let Some(label) = label else {
            return Err(invalid());
        };
        let first = *members.first().ok_or_else(invalid)?;
        groups.push(Group {
            id: format!("field:{brain}:{kind}:{id}"),
            kind: "metadata".into(),
            label,
            label_provenance: LabelProvenance {
                kind: "canonical_scope".into(),
                source_version_id: first,
                byte_start: None,
                byte_end: None,
                quote: None,
                field_kind: Some(kind),
                field_id: Some(id),
            },
            source_versions: members.into_iter().collect(),
        });
    }
    Ok(groups)
}

async fn fresh(tx: &mut Tx<'_>, cx: Context, p: &Prepared) -> Result<()> {
    if analytics_epoch(tx, cx.brain).await? != p.epoch {
        return Err(changed());
    }
    let current: Option<Uuid> =
        sqlx::query_scalar("SELECT policy_id FROM model_policy_heads WHERE brain_id=$1")
            .bind(cx.brain)
            .fetch_optional(&mut **tx)
            .await?;
    if current.unwrap_or(Uuid::nil()) != p.policy {
        return Err(changed());
    }
    deadline(tx, Some(p.deadline)).await?;
    let candidates: Vec<_> = p.sources.iter().filter_map(|s| s.candidate).collect();
    sqlx::query_scalar::<_,Uuid>("SELECT a.id FROM accounts a WHERE a.id IN(SELECT actor_id FROM knowledge_shadow_candidates WHERE brain_id=$1 AND id=ANY($2)) FOR SHARE").bind(cx.brain).bind(&candidates).fetch_all(&mut **tx).await?;
    sqlx::query_scalar::<_,Uuid>("SELECT d.id FROM devices d WHERE d.id IN(SELECT device_id FROM knowledge_shadow_candidates WHERE brain_id=$1 AND id=ANY($2)) FOR SHARE").bind(cx.brain).bind(&candidates).fetch_all(&mut **tx).await?;
    let valid:Vec<(String,String,String,String,String,String)>=sqlx::query_as("SELECT q.prompt_label,q.schema_label,c.schema_revision,c.adapter_revision,c.payload->>'schema',c.payload->>'adapter_revision' FROM knowledge_shadow_candidates c JOIN model_requests q ON q.brain_id=c.brain_id AND q.id=c.request_id JOIN accounts a ON a.id=c.actor_id LEFT JOIN devices d ON d.id=c.device_id AND d.account_id=c.actor_id WHERE c.brain_id=$1 AND c.id=ANY($2) AND c.expires_at>clock_timestamp() AND q.state='succeeded' AND NOT q.suppressed AND NOT q.detail_expired AND q.mapping_output_sha256=c.output_sha256 AND q.mapping_source_version_id=c.source_version_id AND q.policy_id=c.policy_id AND q.actor_id=c.actor_id AND q.device_id IS NOT DISTINCT FROM c.device_id AND a.enabled AND(a.auth_kind='local' OR a.membership_until>clock_timestamp()) AND recollect_account_role(c.brain_id,c.actor_id) IN('writer','admin') AND(c.device_id IS NULL OR(d.claimed AND d.revoked_at IS NULL AND d.expires_at>clock_timestamp()))")
  .bind(cx.brain).bind(&candidates).fetch_all(&mut **tx).await?;
    if valid.len() != candidates.len()
        || valid.iter().any(
            |(prompt, schema, stored_schema, adapter, payload_schema, payload_adapter)| {
                stored_schema != mapping::SOURCE_SCHEMA
                    || stored_schema != payload_schema
                    || adapter != payload_adapter
                    || mapping::production::mapping_adapter(prompt, schema)
                        != Some(adapter.as_str())
            },
        )
    {
        return Err(invalid());
    }
    Ok(())
}

/// Explicit bounded experiment admission. Every source appears in its result;
/// unavailable optional NER/vector features never manufacture success.
pub async fn queue(
    state: &AppState,
    evaluator: &Auth,
    brain: Uuid,
    sources: &[ShadowSource],
    mode: Mode,
    parameters: HybridParameters,
    gds_version: &str,
) -> Result<Uuid> {
    evaluator.require_browser()?;
    let cx = Context {
        brain,
        actor: evaluator.user.id,
        device: evaluator.device_id,
    };
    let sources: Vec<_> = sources
        .iter()
        .map(|s| Source {
            // Computational identity is canonical and stable across grid reports.
            // The per-report dependency row receives its own random SQL PK.
            id: s.source_version_id,
            source_id: Uuid::nil(),
            version: s.source_version_id,
            candidate: s.candidate_id,
            context: Value::Null,
            label: String::new(),
            request: None,
            payload: None,
        })
        .collect();
    let p = prepare(state, evaluator, cx, sources, mode, parameters, gds_version).await?;
    tokio::time::timeout(Duration::from_millis(1500),async{
  let mut tx=evaluator.publication_tx(&state.pool).await?;sqlx::query("SET LOCAL statement_timeout='750ms'").execute(&mut *tx).await?;sqlx::query("SET LOCAL lock_timeout='250ms'").execute(&mut *tx).await?;db::require_writer(&mut tx,brain).await?;sqlx::query("SET LOCAL statement_timeout='750ms'").execute(&mut *tx).await?;
  fresh(&mut tx,cx,&p).await?;
  let(total,pending):(i64,i64)=sqlx::query_as("SELECT count(*),count(*) FILTER(WHERE state IN('queued','running')) FROM analytics_reports WHERE brain_id=$1 AND purpose='organization_shadow'").bind(brain).fetch_one(&mut *tx).await?;
  if total>=100||pending>=20{return Err(failure("shadow_capacity","Retained experimental reports exceed their bounded window."));}
  let id=Uuid::new_v4();let audit=db::audit(&mut tx,cx.actor,brain,"organization.shadow.calculate",id,"queued").await?;
  let job=jobs::enqueue_work(&mut tx,cx.actor,brain,audit,id,"graph.analyze","heavy").await?;
  let sequence=privacy_sequence(&mut tx,brain).await?;
  sqlx::query("INSERT INTO analytics_reports(id,brain_id,actor_id,job_id,purpose,algorithm,direction,descriptor,parameters,analytics_epoch,privacy_sequence,node_count,edge_count,expires_at) VALUES($1,$2,$3,$4,'organization_shadow','leiden','both',$5,$6,$7,$8,$9,$10,$11)")
   .bind(id).bind(brain).bind(cx.actor).bind(job).bind(Json(organization::frozen(&p.projection))).bind(Json(super::parameters("leiden")?)).bind(p.epoch).bind(sequence).bind(p.projection.nodes.len() as i32).bind(p.projection.edges.len() as i32).bind(p.deadline).execute(&mut *tx).await?;
  sqlx::query("INSERT INTO knowledge_shadow_calculations(report_id,brain_id,feature_parameters,expected_gds_version,projection,preparation) VALUES($1,$2,$3,$4,$5,$6)")
   .bind(id).bind(brain).bind(Json(parameters)).bind(gds_version).bind(Json(&p.projection)).bind(Json(&p)).execute(&mut *tx).await?;
  sqlx::query("INSERT INTO knowledge_shadow_sources(id,brain_id,report_id,source_id,source_version_id,candidate_id,context) SELECT gen_random_uuid(),$1,$2,source_id,version,candidate,context FROM jsonb_to_recordset($3) x(id uuid,source_id uuid,version uuid,candidate uuid,context jsonb)")
   .bind(brain).bind(id).bind(Json(&p.sources)).execute(&mut *tx).await?;
  sqlx::query("INSERT INTO knowledge_organization_vectors(brain_id,report_id,entry_id,profile_id,request_id) SELECT $1,$2,entry,profile,request FROM jsonb_to_recordset($3) x(entry uuid,profile uuid,request uuid)")
   .bind(brain).bind(id).bind(Json(&p.vectors.entries)).execute(&mut *tx).await?;
  deadline(&mut tx,Some(p.deadline)).await?;tx.commit().await?;Ok(id)
 }).await.map_err(|_|failure("analytics_publication_timeout","Shadow scheduling exceeded its bounded writer window."))?
}

async fn saved(tx: &mut Tx<'_>, brain: Uuid, id: Uuid) -> Result<Prepared> {
    let p:Option<Json<Prepared>>=sqlx::query_scalar("SELECT c.preparation FROM knowledge_shadow_calculations c JOIN analytics_reports r ON r.brain_id=c.brain_id AND r.id=c.report_id WHERE c.brain_id=$1 AND c.report_id=$2 AND r.purpose='organization_shadow' AND r.state IN('queued','running','ready')").bind(brain).bind(id).fetch_optional(&mut **tx).await?;
    p.map(|p| p.0).ok_or_else(invalid)
}
async fn reprepare(
    state: &AppState,
    evaluator: &Auth,
    cx: Context,
    p: &Prepared,
) -> Result<Prepared> {
    let current = prepare(
        state,
        evaluator,
        cx,
        p.sources.clone(),
        p.mode,
        p.parameters,
        &p.gds_version,
    )
    .await?;
    // Seven-day wall-clock caps must not extend a frozen operation. The exact
    // retained dependencies/projection/epoch remain equal; deadlines can tighten.
    if current.epoch != p.epoch
        || current.policy != p.policy
        || serde_json::to_value(&current.sources).map_err(|_| invalid())?
            != serde_json::to_value(&p.sources).map_err(|_| invalid())?
        || current.vectors != p.vectors
        || current.projection != p.projection
        || current.metadata_groups != p.metadata_groups
        || current.deadline < p.deadline
    {
        return Err(changed());
    }
    let mut current = current;
    current.deadline = p.deadline;
    Ok(current)
}

pub(super) async fn run(
    state: &AppState,
    job: &ClaimedJob,
    armed: &mut Option<journal::Entry>,
) -> Result<()> {
    let (mut read, _, evaluator) = work::ready(state, job).await?;
    let p = saved(&mut read, job.brain_id, job.target_id).await?;
    read.commit().await?;
    let cx = Context {
        brain: job.brain_id,
        actor: job.actor_id,
        device: job.device_id,
    };
    let p = reprepare(state, &evaluator, cx, &p).await?;
    let mut output = None;
    if p.mode.native() && !p.projection.nodes.is_empty() {
        let entry = journal::arm(state, job).await?;
        *armed = Some(entry.clone());
        let (mut tx, r, _) = work::ready(state, job).await?;
        fresh(&mut tx, cx, &p).await?;
        journal::insert(&mut tx, &entry).await?;
        tx.commit().await?;
        let frozen = organization::frozen(&p.projection);
        let weights: Vec<_> = p.projection.edges.iter().map(|e| e.weight).collect();
        let mut calculate = Box::pin(native::calculate_weighted(
            state, &r, &frozen, &entry, &weights,
        ));
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        interval.tick().await;
        let result = loop {
            tokio::select! {result=&mut calculate=>break result?,_=interval.tick()=>{tokio::select!{biased;result=&mut calculate=>break result?,checked=work::progress(state,job,&entry)=>checked?,}}}
        };
        journal::close(&state.config, &state.pool, &state.http, &entry).await?;
        *armed = None;
        if result.version != p.gds_version {
            return Err(failure(
                "organization_engine_version_changed",
                "The experimental engine does not match its frozen version.",
            ));
        }
        output = Some(result);
    }
    reprepare(state, &evaluator, cx, &p).await?;
    let mut native_groups = BTreeMap::new();
    if let Some(output) = &output {
        let keys: BTreeMap<_, _> = organization::frozen(&p.projection)
            .nodes
            .into_iter()
            .enumerate()
            .map(|(i, k)| (k, i))
            .collect();
        if output.scores.len() != keys.len() {
            return Err(invalid());
        }
        let mut seen = BTreeSet::new();
        for score in &output.scores {
            let index = *keys.get(&score.key).ok_or_else(invalid)?;
            if !seen.insert(index) {
                return Err(invalid());
            }
            if let Vertex::Input { id, .. } = p.projection.nodes[index] {
                native_groups.insert(id, score.group.ok_or_else(invalid)?);
            }
        }
    }
    let result = groups::build(
        job.target_id,
        job.brain_id,
        &p,
        &native_groups,
        output.as_ref().map(|o| o.version.clone()),
    )?;
    tokio::time::timeout(Duration::from_millis(1500),async{
  let(mut tx,r,_)=work::ready(state,job).await?;sqlx::query("SET LOCAL statement_timeout='750ms'").execute(&mut *tx).await?;fresh(&mut tx,cx,&p).await?;if r.analytics_epoch!=p.epoch{return Err(changed());}
  sqlx::query("UPDATE knowledge_shadow_calculations SET result=$3 WHERE brain_id=$1 AND report_id=$2").bind(job.brain_id).bind(job.target_id).bind(Json(&result)).execute(&mut *tx).await?;
  sqlx::query("UPDATE analytics_reports SET state='ready',scores=$3,gds_version=$4,estimated_bytes=$5,projected_edge_count=$6,published_at=clock_timestamp(),error_code=NULL WHERE brain_id=$1 AND id=$2")
   .bind(job.brain_id).bind(job.target_id).bind(output.as_ref().map(|o|Json(&o.scores))).bind(output.as_ref().map(|o|&o.version)).bind(output.as_ref().map(|o|o.estimate)).bind(output.as_ref().map(|o|o.projected_edges as i32)).execute(&mut *tx).await?;
  let lease:DateTime<Utc>=sqlx::query_scalar("SELECT lease_until FROM jobs WHERE id=$1 AND lease_token=$2").bind(job.id).bind(job.lease_token).fetch_one(&mut *tx).await?;
  let updated=sqlx::query("UPDATE jobs SET state='succeeded',progress=100,lease_token=NULL,lease_until=NULL,error_code=NULL,updated_at=clock_timestamp() WHERE id=$1 AND state='running' AND lease_token=$2 AND lease_until>clock_timestamp()").bind(job.id).bind(job.lease_token).execute(&mut *tx).await?;
  if updated.rows_affected()!=1{return Err(failure("analytics_lease_lost","The experimental publication lease expired."));}
  db::audit(&mut tx,job.actor_id,job.brain_id,"organization.shadow.result",job.target_id,"unpublished").await?;
  fresh(&mut tx,cx,&p).await?;deadline(&mut tx,Some(lease.min(p.deadline))).await?;tx.commit().await?;Ok(())
 }).await.map_err(|_|failure("analytics_publication_timeout","Shadow result publication exceeded its bounded writer window."))?
}

/// Read only through explicit authenticated Rust tooling. Public analytics
/// routes reject this purpose, including when its exact report ID is known.
pub async fn result(
    state: &AppState,
    evaluator: &Auth,
    brain: Uuid,
    id: Uuid,
) -> Result<ShadowResult> {
    evaluator.require_browser()?;
    let cx = Context {
        brain,
        actor: evaluator.user.id,
        device: evaluator.device_id,
    };
    let mut tx = evaluator.preparation_tx(&state.pool).await?;
    settings(&mut tx).await?;
    db::require_role(&mut tx, brain, false).await?;
    let p = saved(&mut tx, brain, id).await?;
    let status: String = sqlx::query_scalar("SELECT state FROM analytics_reports WHERE brain_id=$1 AND id=$2 AND actor_id=$3 AND purpose='organization_shadow'").bind(brain).bind(id).bind(evaluator.user.id).fetch_optional(&mut *tx).await?.ok_or_else(invalid)?;
    if status != "ready" {
        return Err(failure(
            "shadow_result_pending",
            "The explicit experimental result is still queued or running.",
        ));
    }
    let result:Option<Json<ShadowResult>>=sqlx::query_scalar("SELECT c.result FROM knowledge_shadow_calculations c JOIN analytics_reports r ON r.brain_id=c.brain_id AND r.id=c.report_id WHERE c.brain_id=$1 AND c.report_id=$2 AND r.actor_id=$3 AND r.state='ready' AND r.purpose='organization_shadow'").bind(brain).bind(id).bind(evaluator.user.id).fetch_optional(&mut *tx).await?.flatten();
    let result = result.ok_or_else(invalid)?.0;
    tx.commit().await?;
    reprepare(state, evaluator, cx, &p).await?;
    tokio::time::timeout(Duration::from_millis(1500), async {
        let mut tx = evaluator.publication_tx(&state.pool).await?;
        sqlx::query("SET LOCAL statement_timeout='750ms'")
            .execute(&mut *tx)
            .await?;
        sqlx::query("SET LOCAL lock_timeout='250ms'")
            .execute(&mut *tx)
            .await?;
        db::require_writer(&mut tx, brain).await?;
        fresh(&mut tx, cx, &p).await?;
        tx.commit().await?;
        Ok::<_, Error>(())
    })
    .await
    .map_err(|_| {
        failure(
            "analytics_publication_timeout",
            "Shadow result qualification exceeded its bounded window.",
        )
    })??;
    Ok(result)
}
