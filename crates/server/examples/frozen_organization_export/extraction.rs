//! Fresh successful gateway request -> current tuple adapter -> shadow admission.
use super::{
    io,
    owned::{Owned, api},
    scopes::Imported,
    toy::{Toy, Transport},
};
use anyhow::{Result, ensure};
use recollect_server::{
    auth::Auth, graph::analytics::shadow, knowledge_mapping::production, model_gateway as gateway,
};
use serde_json::{Value, json};
use uuid::Uuid;
#[derive(Clone)]
pub struct Observed {
    pub toy: Toy,
    pub imported: Option<Imported>,
    pub extraction: Value,
    pub candidate: Option<Uuid>,
    pub provenance: Value,
}
pub async fn extract(
    o: &Owned,
    auth: &Auth,
    t: Toy,
    s: Option<Imported>,
    wire: &Transport,
) -> Result<Observed> {
    let text_hash = io::sha(t.text.as_bytes())?;
    let Some(s) = s else {
        ensure!(
            t.text.is_empty(),
            "Nonempty source cannot be silently skipped"
        );
        return Ok(Observed {
            toy: t,
            imported: None,
            extraction: json!({"state":"not_attempted","failure":null,"candidates":null}),
            candidate: None,
            provenance: json!({"reason":"exact_empty_import_ineligible","source_version_id":null,"text_sha256":text_hash,"byte_length":0}),
        });
    };
    let selection:Value=sqlx::query_scalar("SELECT selection FROM source_import_scopes WHERE brain_id=$1 AND version_id=$2 AND operation_id=$3").bind(s.brain).bind(s.version).bind(s.operation).fetch_one(&o.admin).await?;
    ensure!(
        selection == serde_json::to_value(&s.selection)?,
        "Canonical import selection changed"
    );
    let policy: Uuid =
        sqlx::query_scalar("SELECT policy_id FROM model_policy_heads WHERE brain_id=$1")
            .bind(s.brain)
            .fetch_one(&o.admin)
            .await?;
    let cx = gateway::Context {
        brain: s.brain,
        actor: o.actor,
        device: Some(s.device),
    };
    let mut candidate = None;
    let mut request_provenance = Value::Null;
    let mut probes = Value::Null;
    let extraction = if t.kind == "missing" {
        json!({"state":"missing","failure":null,"candidates":null})
    } else {
        let operation = Uuid::new_v4();
        let inv = gateway::extraction(operation, s.version);
        ensure!(
            inv.prompt_label == "source-facts-4" && inv.schema_label == "source-facts-4",
            "Current builder tuple drift"
        );
        let preview = api(gateway::preview_for_policy(&o.state, cx, &inv, policy).await)?;
        let body = serde_json::to_vec(&preview.body)?;
        ensure!(body.len() <= 65536, "Complete request exceeds cap");
        let payload = json!({"claims":[],"mapping":t.mapping});
        let response_hash = io::sha(&serde_json::to_vec(&payload)?)?;
        wire.expect(io::sha(&body)?, payload).await?;
        let response =
            api(gateway::invoke_previewed_for_policy(&o.state, cx, inv, &preview).await)?;
        ensure!(
            response.request.state == "succeeded"
                && response.request.actor_id == o.actor
                && response.request.device_id == Some(s.device)
                && response.request.operation_id == operation,
            "Actual producer request not successful or bound"
        );
        request_provenance=sqlx::query_scalar("SELECT jsonb_build_object('request_id',q.id,'operation_id',q.operation_id,'policy_id',q.policy_id,'purpose',q.purpose,'state',q.state,'provider',q.provider,'requested_model',q.model,'returned_model',q.returned_model,'prompt_label',q.prompt_label,'schema_label',q.schema_label,'admitted_mapping_digest',q.mapping_output_sha256,'mapping_source_version_id',q.mapping_source_version_id,'offered_reference_digest',(SELECT encode(sha256(convert_to(coalesce(jsonb_agg(jsonb_build_object('kind',i.kind,'id',i.input_id) ORDER BY i.kind,i.input_id),'[]'::jsonb)::text,'UTF8')),'hex') FROM model_request_inputs i WHERE i.brain_id=q.brain_id AND i.request_id=q.id)) FROM model_requests q WHERE q.brain_id=$1 AND q.id=$2").bind(s.brain).bind(response.request.id).fetch_one(&o.admin).await?;
        request_provenance["complete_request_body_sha256"] = json!(io::sha(&body)?);
        request_provenance["controlled_response_sha256"] = json!(response_hash);
        let Some(gateway::Output::Json(value)) = response.output else {
            anyhow::bail!("Controlled JSON output missing")
        };
        if value["mapping"].is_null() {
            ensure!(t.kind == "absent", "Unexpected producer abstention");
            json!({"state":"absent","failure":null,"candidates":null})
        } else {
            match production::adapt_for_request(
                s.version,
                &t.text,
                &value["mapping"],
                &response.request.prompt_label,
                &response.request.schema_label,
            ) {
                Ok(dto) => {
                    ensure!(t.kind == "validated", "Rejected toy unexpectedly valid");
                    let wrong_tuple = production::adapt_for_request(
                        s.version,
                        &t.text,
                        &value["mapping"],
                        "source-facts-3",
                        "source-facts-4",
                    )
                    .is_err();
                    let mut wrong_span = value["mapping"].clone();
                    wrong_span["mentions"][0]["span"]["quote"] = json!("unoffered sentinel");
                    let span_rejected = production::adapt_for_request(
                        s.version,
                        &t.text,
                        &wrong_span,
                        &response.request.prompt_label,
                        &response.request.schema_label,
                    )
                    .is_err();
                    let mut altered = value["mapping"].clone();
                    altered["mentions"][0]["confidence"] = json!(0.31);
                    let digest_rejected =
                        shadow::admit(&o.state, auth, cx, response.request.id, s.version, &altered)
                            .await
                            .is_err();
                    let device_rejected = shadow::admit(
                        &o.state,
                        auth,
                        gateway::Context {
                            device: Some(Uuid::new_v4()),
                            ..cx
                        },
                        response.request.id,
                        s.version,
                        &value["mapping"],
                    )
                    .await
                    .is_err();
                    ensure!(
                        wrong_tuple && span_rejected && digest_rejected && device_rejected,
                        "Invalid origin/adapter probe admitted"
                    );
                    candidate = Some(api(shadow::admit(
                        &o.state,
                        auth,
                        cx,
                        response.request.id,
                        s.version,
                        &value["mapping"],
                    )
                    .await)?);
                    probes = json!({"wrong_tuple_rejected":wrong_tuple,"wrong_span_rejected":span_rejected,"wrong_mapping_digest_rejected":digest_rejected,"wrong_device_rejected":device_rejected});
                    json!({"state":"validated","failure":null,"candidates":dto})
                }
                Err(f) => {
                    ensure!(t.kind == "rejected", "Valid controlled toy rejected");
                    json!({"state":"rejected","failure":format!("{f:?}"),"candidates":null})
                }
            }
        }
    };
    let provenance = json!({"document_id":t.id,"brain_id":s.brain,"source_id":s.source,"source_version_id":s.version,"import_operation_id":s.operation,"device_id":s.device,"import_policy_id":s.policy,"extraction_policy_id":policy,"canonical_selection":s.selection,"text_sha256":text_hash,"byte_length":t.text.len(),"extraction_state":extraction["state"],"extraction_failure":extraction["failure"],"candidate_id":candidate,"request":request_provenance,"rejection_probes":probes});
    Ok(Observed {
        toy: t,
        imported: Some(s),
        extraction,
        candidate,
        provenance,
    })
}
