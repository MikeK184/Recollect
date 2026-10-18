//! Newly authored synthetic fixtures; no old corpus, annotations or responses.
use anyhow::{Result, ensure};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    routing::post,
};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::Mutex;
#[derive(Clone)]
pub struct Toy {
    pub id: &'static str,
    pub title: &'static str,
    pub text: String,
    pub scope: &'static str,
    pub kind: &'static str,
    pub mapping: Value,
}
fn anchor(q: &str, n: Value) -> Value {
    json!({"quote":q,"line_from":1,"line_to":1,"occurrence":n})
}
fn mention(key: &str, kind: &str, q: &str, n: Value) -> Value {
    json!({"key":key,"kind":kind,"span":anchor(q,n),"confidence":0.9})
}
pub fn mapping() -> Value {
    let a = anchor("Café aka Cafe.", Value::Null);
    let r = anchor("Café uses Lumen.", Value::Null);
    json!({"language":"en","mentions":[mention("c1","service","Café",json!(1)),mention("c2","service","Cafe",Value::Null),mention("c3","service","Café",json!(2)),mention("l","technology","Lumen",Value::Null),mention("o1","concept","éé",json!(1)),mention("o2","concept","éé",json!(2))],"aliases":[{"left":"c1","right":"c2","span":a}],"relations":[{"from":"c3","to":"l","relation":"uses","span":r},{"from":"c3","to":"l","relation":"uses","span":r}]})
}
pub fn fixtures() -> Vec<Toy> {
    let text = "Café aka Cafe. Café uses Lumen. ééé.";
    let simple = json!({"language":"en","mentions":[mention("s","service","Drift",Value::Null),mention("t","technology","Lumen",Value::Null)],"aliases":[],"relations":[{"from":"s","to":"t","relation":"uses","span":anchor("Drift uses Lumen.",Value::Null)}]});
    let mut rejected = simple.clone();
    rejected["mentions"][0]["span"]["quote"] = json!("foreign quote");
    vec![
        Toy {
            id: "unicode",
            title: "Accent service notes",
            text: text.into(),
            scope: "aurora/harbor/dev",
            kind: "validated",
            mapping: mapping(),
        },
        Toy {
            id: "paired",
            title: "Routing implementation",
            text: "Drift uses Lumen.".into(),
            scope: "aurora/harbor/dev",
            kind: "validated",
            mapping: simple.clone(),
        },
        Toy {
            id: "missing",
            title: "Fallback observation",
            text: "Drift uses Lumen.".into(),
            scope: "aurora/works/prod",
            kind: "missing",
            mapping: Value::Null,
        },
        Toy {
            id: "rejected",
            title: "Partial extraction",
            text: "Drift uses Lumen.".into(),
            scope: "aurora/works/prod",
            kind: "rejected",
            mapping: rejected,
        },
        Toy {
            id: "duplicate",
            title: "Accent service notes",
            text: text.into(),
            scope: "boreal/harbor/dev",
            kind: "validated",
            mapping: mapping(),
        },
        Toy {
            id: "incomplete",
            title: "Buffer trace",
            text: format!(
                "Aster starts.\n{}\nINCOMPLETE_VECTOR tail.",
                "Buffer records the trace. ".repeat(550)
            ),
            scope: "aurora/works/prod",
            kind: "validated",
            mapping: json!({"language":"en","mentions":[mention("a","technology","Aster",Value::Null)],"aliases":[],"relations":[]}),
        },
        Toy {
            id: "empty_a",
            title: "Empty control",
            text: "".into(),
            scope: "aurora/harbor/dev",
            kind: "empty",
            mapping: Value::Null,
        },
        Toy {
            id: "empty_b",
            title: "Blank control",
            text: "".into(),
            scope: "boreal/harbor/dev",
            kind: "empty",
            mapping: Value::Null,
        },
        Toy {
            id: "abstained",
            title: "Breeze note",
            text: "Le vent souffle.".into(),
            scope: "aurora/works/prod",
            kind: "absent",
            mapping: Value::Null,
        },
    ]
}
pub fn fixture_json(toys: &[Toy]) -> Value {
    json!({"schema":"recollect-native-independent-toys-1","sources":toys.iter().map(|t|json!({"document_id":t.id,"title":t.title,"text":t.text,"scope":t.scope,"language":if t.kind=="absent"{"fr"}else{"en"},"partition":"held_out","controlled_case":t.kind,"mapping_response":t.mapping})).collect::<Vec<_>>()})
}
#[derive(Clone)]
pub struct Transport {
    pub pending: Arc<Mutex<Option<(String, Value)>>>,
    pub calls: Arc<AtomicUsize>,
    pub vectors: Arc<AtomicUsize>,
}
impl Transport {
    pub fn new() -> Self {
        Self {
            pending: Arc::new(Mutex::new(None)),
            calls: Arc::new(AtomicUsize::new(0)),
            vectors: Arc::new(AtomicUsize::new(0)),
        }
    }
    pub async fn expect(&self, hash: String, value: Value) -> Result<()> {
        let mut p = self.pending.lock().await;
        ensure!(p.is_none(), "Pending controlled response");
        *p = Some((hash, value));
        Ok(())
    }
    pub fn router(&self) -> Router {
        Router::new()
            .route("/v1/responses", post(response))
            .route("/v1/embeddings", post(embedding))
            .layer(DefaultBodyLimit::max(65536))
            .with_state(self.clone())
    }
}
async fn response(
    State(w): State<Transport>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let Some((hash, payload)) = w.pending.lock().await.take() else {
        return Err(StatusCode::CONFLICT);
    };
    let actual = super::io::sha(&serde_json::to_vec(&body).map_err(|_| StatusCode::BAD_REQUEST)?)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if actual != hash {
        return Err(StatusCode::CONFLICT);
    }
    w.calls.fetch_add(1, Ordering::SeqCst);
    Ok(Json(
        json!({"model":body["model"],"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":payload.to_string()}]}],"usage":{"input_tokens":10,"output_tokens":10,"total_tokens":20}}),
    ))
}
async fn embedding(
    State(w): State<Transport>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let inputs = body["input"].as_array().ok_or(StatusCode::BAD_REQUEST)?;
    if body["dimensions"] != 3072 || inputs.len() > 100 {
        return Err(StatusCode::BAD_REQUEST);
    }
    w.vectors.fetch_add(1, Ordering::SeqCst);
    if inputs
        .iter()
        .any(|v| v.as_str().is_some_and(|s| s.contains("INCOMPLETE_VECTOR")))
    {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    let mut vector = vec![0.0; 3072];
    vector[0] = 1.0;
    Ok(Json(
        json!({"model":body["model"],"data":inputs.iter().enumerate().map(|(i,_)|json!({"index":i,"embedding":vector})).collect::<Vec<_>>(),"usage":{"prompt_tokens":10,"total_tokens":10}}),
    ))
}
