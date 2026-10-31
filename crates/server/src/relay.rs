//! Same-origin relay to SpacetimeDB, for the browser. In live mode the page
//! subscribes and calls reducers over a websocket; relaying it through this
//! server means one origin to expose, and SpacetimeDB itself stays on
//! loopback. Only what a client of the axioms database needs is relayed:
//! the short-lived websocket token and that database's subscription. Its
//! admin surface (publishing, deleting, SQL) is not.

use crate::state::AppState;
use axum::Router;
use axum::body::Body;
use axum::extract::{Path, Request, State};
use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, post};
use hyper_util::rt::TokioIo;
use std::sync::Arc;

type St = State<Arc<AppState>>;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/identity/websocket-token", post(token))
        .route("/v1/database/{db}/subscribe", any(subscribe))
}

async fn token(State(s): St, req: Request) -> Response {
    match &s.live {
        Some(l) => relay(&l.upstream, req).await,
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn subscribe(State(s): St, Path(db): Path<String>, req: Request) -> Response {
    match &s.live {
        Some(l) if l.db == db => relay(&l.upstream, req).await,
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Forward one request to `upstream` (`http://host:port`), and when it
/// switches protocols, splice the two upgraded connections together.
async fn relay(upstream: &str, req: Request) -> Response {
    match forward(upstream, req).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("spacetimedb relay: {e:#}");
            (StatusCode::BAD_GATEWAY, "SpacetimeDB is unreachable").into_response()
        }
    }
}

async fn forward(upstream: &str, mut req: Request) -> anyhow::Result<Response> {
    let authority = upstream
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_owned();
    let stream = tokio::net::TcpStream::connect(&authority).await?;
    let (mut sender, conn) =
        hyper::client::conn::http1::handshake::<_, Body>(TokioIo::new(stream)).await?;
    tokio::spawn(async move {
        if let Err(e) = conn.with_upgrades().await {
            tracing::debug!("spacetimedb relay connection: {e}");
        }
    });
    let client = req
        .headers()
        .contains_key(header::UPGRADE)
        .then(|| hyper::upgrade::on(&mut req));
    let path = req.uri().path_and_query().map_or("/", |p| p.as_str());
    *req.uri_mut() = path.parse::<Uri>()?;
    req.headers_mut().insert(header::HOST, authority.parse()?);
    let mut resp = sender.send_request(req).await?;
    if let (StatusCode::SWITCHING_PROTOCOLS, Some(client)) = (resp.status(), client) {
        let server = hyper::upgrade::on(&mut resp);
        tokio::spawn(async move {
            match tokio::try_join!(client, server) {
                Ok((c, s)) => {
                    let (mut c, mut s) = (TokioIo::new(c), TokioIo::new(s));
                    let _ = tokio::io::copy_bidirectional(&mut c, &mut s).await;
                }
                Err(e) => tracing::warn!("spacetimedb relay upgrade: {e}"),
            }
        });
    }
    Ok(resp.map(Body::new))
}
