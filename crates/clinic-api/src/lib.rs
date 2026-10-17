//! Clinic Ops API. Synthetic data only; every AI decision is checked, signed (ML-DSA-65) and chained.
pub mod synthetic;

use axum::extract::State;
use axum::http::{HeaderValue, Method, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use decisions::{DecisionRecord, GateOutcome, Ledger, LedgerError, Receipt, input_hash};
use entropa_core::Probe;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tower_http::cors::{AllowOrigin, CorsLayer};

const SEED: u64 = 2026;
const APPOINTMENTS: usize = 20;

pub struct AppState {
    pub ledger: Mutex<Ledger>,
    pub now: fn() -> u64,
}

type Shared = Arc<AppState>;

/// What a caller may send. The raw `input` is hashed on arrival and then dropped; it is never stored.
#[derive(Deserialize)]
struct NewDecision {
    actor: String,
    kind: String,
    input: String,
    output: String,
    human_gate: Option<GateOutcome>,
}

/// Build the router. `cors_origin`: the one browser origin allowed to call the API (the healthcare front end).
pub fn app(probe: Probe, now: fn() -> u64, cors_origin: Option<String>) -> Router {
    let state = Arc::new(AppState { ledger: Mutex::new(Ledger::new(probe, now())), now });
    let mut router = Router::new()
        .route("/health", get(health))
        .route("/appointments", get(appointments))
        .route("/summary", get(summary))
        .route("/decisions", get(list_decisions).post(post_decision))
        .route("/verify", post(verify))
        .route("/chain/head", get(chain_head))
        .with_state(state);
    if let Some(origin) = cors_origin.and_then(|o| o.parse::<HeaderValue>().ok()) {
        router = router.layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::predicate(move |o: &HeaderValue, _| o == &origin))
                .allow_methods([Method::GET, Method::POST])
                .allow_headers([axum::http::header::CONTENT_TYPE]),
        );
    }
    router
}

async fn health(State(s): State<Shared>) -> Json<Value> {
    let l = s.ledger.lock().unwrap();
    Json(json!({"status": "ok", "chain_len": l.len()}))
}

async fn appointments() -> Json<Vec<synthetic::Appointment>> {
    Json(synthetic::appointments(SEED, APPOINTMENTS))
}

async fn summary() -> Json<synthetic::Summary> {
    Json(synthetic::summary(&synthetic::appointments(SEED, APPOINTMENTS)))
}

async fn post_decision(State(s): State<Shared>, Json(d): Json<NewDecision>) -> (StatusCode, Json<Value>) {
    let ts = (s.now)();
    let rec = DecisionRecord {
        actor: d.actor,
        kind: d.kind,
        input_hash: input_hash(d.input.as_bytes()),
        output: d.output,
        human_gate: d.human_gate,
        timestamp: ts,
    };
    let mut l = s.ledger.lock().unwrap();
    match l.record(&rec, ts) {
        Ok(receipt) => (StatusCode::CREATED, Json(serde_json::to_value(receipt).unwrap())),
        Err(LedgerError::Rejected(v)) => {
            let names: Vec<String> = v.iter().map(|x| format!("{x:?}")).collect();
            (StatusCode::UNPROCESSABLE_ENTITY, Json(json!({"error": "rejected", "violations": names})))
        }
        Err(LedgerError::Chain(e)) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{e:?}")}))),
    }
}

async fn list_decisions(State(s): State<Shared>) -> Json<Vec<Value>> {
    let l = s.ledger.lock().unwrap();
    Json(
        l.blocks()
            .iter()
            .skip(1) // genesis carries no decision
            .filter_map(|b| {
                b.transactions.first().map(|tx| {
                    json!({"index": b.index, "actor": tx.from, "kind": tx.kind, "block_hash": b.hash, "timestamp": b.timestamp})
                })
            })
            .collect(),
    )
}

async fn verify(State(s): State<Shared>, Json(r): Json<Receipt>) -> Json<Value> {
    let l = s.ledger.lock().unwrap();
    Json(json!({"valid": l.verify_receipt(&r)}))
}

async fn chain_head(State(s): State<Shared>) -> Json<Value> {
    let l = s.ledger.lock().unwrap();
    Json(json!({"height": l.len(), "head_hash": l.head_hash(), "pubkey": l.public_key(), "verified": l.verify().is_ok()}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    fn fixed_now() -> u64 {
        1_800_000_000
    }

    fn router() -> Router {
        app(Probe::spawn(), fixed_now, Some("https://clinic.example".into()))
    }

    async fn call(r: &Router, req: Request<Body>) -> (StatusCode, Value, axum::http::HeaderMap) {
        let res = r.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let headers = res.headers().clone();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let v = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, v, headers)
    }

    fn get(path: &str) -> Request<Body> {
        Request::builder().uri(path).body(Body::empty()).unwrap()
    }

    fn post(path: &str, body: Value) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(path)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    fn good() -> Value {
        json!({"actor":"copilot-review","kind":"review","input":"diff of the clinic scheduler","output":"One nit: rename a variable.","human_gate":"Approved"})
    }

    #[tokio::test]
    async fn health_is_ok_and_reports_the_chain_length() {
        let (s, v, _) = call(&router(), get("/health")).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(v["status"], "ok");
        assert_eq!(v["chain_len"], 1);
    }

    #[tokio::test]
    async fn appointments_are_synthetic_and_stable_between_calls() {
        let r = router();
        let (s, a, _) = call(&r, get("/appointments")).await;
        let (_, b, _) = call(&r, get("/appointments")).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(a.as_array().unwrap().len(), 20);
        assert_eq!(a, b);
        assert!(a[0]["patient"].as_str().unwrap().starts_with("Synthetic Patient "));
    }

    #[tokio::test]
    async fn summary_totals_match_the_appointments() {
        let (s, v, _) = call(&router(), get("/summary")).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(v["total"], 20);
    }

    #[tokio::test]
    async fn a_good_decision_is_recorded_and_returns_a_receipt() {
        let r = router();
        let (s, v, _) = call(&r, post("/decisions", good())).await;
        assert_eq!(s, StatusCode::CREATED);
        assert_eq!(v["index"], 1);
        assert_eq!(v["block_hash"].as_str().unwrap().len(), 64);
        let (_, head, _) = call(&r, get("/chain/head")).await;
        assert_eq!(head["height"], 2);
        assert_eq!(head["verified"], true);
        assert_eq!(head["head_hash"], v["block_hash"]);
    }

    #[tokio::test]
    async fn a_decision_with_patient_data_in_the_output_is_refused_and_not_recorded() {
        let r = router();
        let mut bad = good();
        bad["output"] = json!("Patient SSN 123-45-6789 flagged");
        let (s, v, _) = call(&r, post("/decisions", bad)).await;
        assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(v["violations"].to_string().contains("ssn"));
        let (_, head, _) = call(&r, get("/chain/head")).await;
        assert_eq!(head["height"], 1);
    }

    #[tokio::test]
    async fn a_decision_without_a_human_gate_is_refused() {
        let mut bad = good();
        bad.as_object_mut().unwrap().remove("human_gate");
        let (s, v, _) = call(&router(), post("/decisions", bad)).await;
        assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(v["violations"].to_string().contains("GateMissing"));
    }

    #[tokio::test]
    async fn raw_input_is_never_stored_or_echoed() {
        let r = router();
        let mut d = good();
        d["input"] = json!("very secret clinic note XYZZY");
        let (_, receipt, _) = call(&r, post("/decisions", d)).await;
        let (_, list, _) = call(&r, get("/decisions")).await;
        let (_, head, _) = call(&r, get("/chain/head")).await;
        for blob in [receipt.to_string(), list.to_string(), head.to_string()] {
            assert!(!blob.contains("XYZZY"), "raw input leaked: {blob}");
        }
    }

    #[tokio::test]
    async fn the_decision_list_shows_metadata_not_content() {
        let r = router();
        call(&r, post("/decisions", good())).await;
        let (s, list, _) = call(&r, get("/decisions")).await;
        assert_eq!(s, StatusCode::OK);
        let items = list.as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["index"], 1);
        assert_eq!(items[0]["actor"], "copilot-review");
        assert_eq!(items[0]["kind"], "review");
    }

    #[tokio::test]
    async fn verify_accepts_a_real_receipt_and_rejects_a_tampered_one() {
        let r = router();
        let (_, receipt, _) = call(&r, post("/decisions", good())).await;
        let (s, ok, _) = call(&r, post("/verify", receipt.clone())).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(ok["valid"], true);
        let mut forged = receipt;
        forged["block_hash"] = json!("0".repeat(64));
        let (_, bad, _) = call(&r, post("/verify", forged)).await;
        assert_eq!(bad["valid"], false);
    }

    #[tokio::test]
    async fn cors_allows_only_the_configured_origin() {
        let r = router();
        let req = Request::builder().uri("/health").header("origin", "https://clinic.example").body(Body::empty()).unwrap();
        let (_, _, h) = call(&r, req).await;
        assert_eq!(h.get("access-control-allow-origin").unwrap(), "https://clinic.example");
        let req = Request::builder().uri("/health").header("origin", "https://evil.example").body(Body::empty()).unwrap();
        let (_, _, h) = call(&r, req).await;
        assert!(h.get("access-control-allow-origin").is_none());
    }

    #[tokio::test]
    async fn unknown_routes_are_404() {
        let (s, _, _) = call(&router(), get("/nope")).await;
        assert_eq!(s, StatusCode::NOT_FOUND);
    }
}
