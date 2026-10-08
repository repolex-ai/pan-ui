//! pan-ui — look through a Pan store fast, and rate it from the keyboard.
//!
//! The data comes from `pand`, Pan's one daemon, which holds every store on
//! the machine. This is the same shape as git-lex-ui over `gitlexd` and
//! ravel-ui over `raveld`: one daemon sees every instance, and one page shows
//! any of them, one at a time.
//!
//! The browser only ever talks to this process. This process keeps an index
//! of each store it has been asked about (`index.rs`), a queue of marks Pan
//! has not saved yet (`queue.rs`), and otherwise passes requests to pand.

mod config;
mod index;
mod pand;
mod queue;

use axum::{
    Json, Router,
    extract::{Path as AxPath, Query, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use clap::Parser;
use index::Index;
use pand::{Pand, valid_id};
use queue::{Mark, Queue};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(name = "pan-ui", about = "Look through a Pan store fast, and rate it from the keyboard", version)]
struct Args {
    /// Port for pan-ui itself. git-lex-ui holds 8888 and ravel-ui 8889.
    #[arg(long, default_value = "8890")]
    port: u16,

    /// Where `pand` listens.
    #[arg(long, default_value_t = pand::DAEMON_PORT)]
    daemon_port: u16,

    /// Where pan-ui keeps its store indexes and its save queue.
    /// Defaults to ~/.pan-ui.
    #[arg(long)]
    data: Option<PathBuf>,

    /// Do not open a browser tab on startup.
    #[arg(long)]
    no_open: bool,

    /// Frontend build directory, read on every request so it can be edited
    /// live. Defaults to ./web/dist.
    #[arg(long)]
    web: Option<PathBuf>,
}

struct AppState {
    pand: Pand,
    queue: Arc<Queue>,
    data: PathBuf,
    web_dir: PathBuf,
    /// One slot per store that has been opened. The mutex orders a mark
    /// against an index being swapped in, so neither loses the other.
    slots: Mutex<HashMap<String, Slot>>,
}

#[derive(Default)]
struct Slot {
    index: Option<Arc<RwLock<Index>>>,
    building: bool,
    error: Option<String>,
}

type S = State<Arc<AppState>>;

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let web_dir = args.web.clone().unwrap_or_else(default_web_dir);
    let data = args.data.clone().unwrap_or_else(|| {
        PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into())).join(".pan-ui")
    });
    let pand = Pand::new(args.daemon_port);
    let queue = Queue::open(data.join("queue.json"));
    let waiting = queue.status().pending;
    let state = Arc::new(AppState {
        pand: pand.clone(),
        queue: Arc::clone(&queue),
        data: data.clone(),
        web_dir: web_dir.clone(),
        slots: Mutex::new(HashMap::new()),
    });
    tokio::spawn(Arc::clone(&queue).drain(pand));
    tokio::spawn(watch(Arc::clone(&state)));

    let app = Router::new()
        .route("/", get(index))
        .route("/api/health", get(api_health))
        .route("/api/stores", get(api_stores))
        .route("/api/stores/{store}/index", get(api_index))
        .route("/api/stores/{store}/images", get(api_images))
        .route("/api/stores/{store}/marks", post(api_marks))
        .route("/api/stores/{store}/rebuild", post(api_rebuild))
        .route("/api/thumb/{id}", get(api_thumb))
        .route("/api/media/{id}", get(api_media))
        .route("/api/facts/{id}", get(api_facts))
        .route("/api/config", get(api_config))
        .route("/api/config/{*name}", axum::routing::put(api_config_save))
        .route("/{*asset}", get(static_asset))
        .with_state(Arc::clone(&state));

    // Fixed port, and it fails loudly if taken. A front door that quietly
    // moves is a front door nobody can find.
    let addr = format!("127.0.0.1:{}", args.port);
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("pan-ui: could not bind {addr}: {e}");
            eprintln!("Something already holds port {}. Stop it, or pass --port.", args.port);
            std::process::exit(1);
        }
    };
    let url = format!("http://{addr}");
    println!("pan-ui listening on {url}");
    println!("Frontend served from {}", web_dir.display());
    println!("Data feed: pand on {}", state.pand.url());
    println!("Indexes and save queue in {}", data.display());
    if waiting > 0 {
        println!("{waiting} mark(s) from an earlier run still to save; sending them now.");
    }
    if !args.no_open {
        let _ = open::that_detached(&url);
    }
    let serve = axum::serve(listener, app).with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
        println!();
    });
    if let Err(e) = serve.await {
        eprintln!("server error: {e}");
    }
    let left = state.queue.status().pending;
    if left > 0 {
        println!("{left} mark(s) not saved to Pan yet; they are kept and go at the next start.");
    }
}

fn default_web_dir() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_default();
    for c in [cwd.join("web/dist"), cwd.join("dist")] {
        if c.is_dir() {
            return c;
        }
    }
    cwd.join("web/dist")
}

fn err(code: StatusCode, msg: impl Into<String>) -> Response {
    (code, msg.into()).into_response()
}

fn check(id: &str) -> Result<(), Response> {
    if valid_id(id) { Ok(()) } else { Err(err(StatusCode::BAD_REQUEST, format!("`{id}` is not an id"))) }
}

// ------------------------------------------------------------------ indexes

fn cache_path(s: &AppState, store: &str) -> PathBuf {
    s.data.join("index").join(format!("{store}.json"))
}

/// Start building a store's index unless it is already being built. The
/// first time, the copy on disk is loaded so the page has something at once;
/// pand is asked anyway, because the copy may be days old.
fn ensure(s: &Arc<AppState>, store: &str) {
    {
        let mut slots = s.slots.lock().unwrap();
        let slot = slots.entry(store.to_string()).or_default();
        if slot.building || slot.index.is_some() {
            return;
        }
        slot.building = true;
    }
    let s = Arc::clone(s);
    let store = store.to_string();
    tokio::spawn(async move {
        let path = cache_path(&s, &store);
        if let Ok(Ok(Some(mut ix))) = tokio::task::spawn_blocking(move || {
            std::fs::read(&path).map(|b| serde_json::from_slice::<Index>(&b).ok())
        })
        .await
        {
            ix.reindex();
            install(&s, &store, ix, true);
        }
        rebuild(s, store).await;
    });
}

/// Ask pand for a fresh index and swap it in.
async fn rebuild(s: Arc<AppState>, store: String) {
    {
        let mut slots = s.slots.lock().unwrap();
        slots.entry(store.clone()).or_default().building = true;
    }
    match Index::build(&s.pand, &store).await {
        Ok(ix) => {
            let body = serde_json::to_vec(&ix);
            let path = cache_path(&s, &store);
            install(&s, &store, ix, false);
            if let Ok(body) = body {
                let _ = tokio::task::spawn_blocking(move || {
                    if let Some(dir) = path.parent() {
                        std::fs::create_dir_all(dir)?;
                    }
                    let tmp = path.with_extension("json.tmp");
                    std::fs::write(&tmp, body)?;
                    std::fs::rename(&tmp, &path)
                })
                .await;
            }
        }
        Err(e) => {
            let mut slots = s.slots.lock().unwrap();
            let slot = slots.entry(store).or_default();
            slot.building = false;
            slot.error = Some(e);
        }
    }
}

/// Swap an index in, with every mark Pan has not saved yet laid over it.
fn install(s: &AppState, store: &str, mut ix: Index, still_building: bool) {
    let mut slots = s.slots.lock().unwrap();
    for (id, m) in s.queue.pending_for(store) {
        apply(&mut ix, &id, m);
    }
    let slot = slots.entry(store.to_string()).or_default();
    slot.index = Some(Arc::new(RwLock::new(ix)));
    slot.building = still_building;
    slot.error = None;
}

fn apply(ix: &mut Index, id: &str, m: Mark) {
    if let Some(&p) = ix.pos.get(id) {
        if let Some(r) = m.rating {
            ix.rating[p as usize] = r.min(5);
        }
        if let Some(f) = m.flag {
            ix.flag[p as usize] = f.signum();
        }
    }
}

/// Every minute, compare each open store's image count with pand's, and
/// rebuild the index of any store that has changed. New images arrive all
/// the time (Horae delivers them); this is how they show up.
async fn watch(s: Arc<AppState>) {
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        let Ok(h) = s.pand.get("/health").await else { continue };
        let counts: HashMap<String, u64> = h
            .get("counts")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|c| Some((c.get("store")?.as_str()?.to_string(), c.get("images")?.as_u64()?)))
                    .collect()
            })
            .unwrap_or_default();
        let stale: Vec<String> = {
            let slots = s.slots.lock().unwrap();
            slots
                .iter()
                .filter(|(_, slot)| !slot.building)
                .filter_map(|(store, slot)| {
                    let ix = slot.index.as_ref()?.read().unwrap();
                    let have = (ix.ids.len() + ix.undated) as u64;
                    (counts.get(store).is_some_and(|&n| n != have)).then(|| store.clone())
                })
                .collect()
        };
        for store in stale {
            rebuild(Arc::clone(&s), store).await;
        }
    }
}

// --------------------------------------------------------------------- api

async fn api_health(State(s): S) -> Response {
    let q = s.queue.status();
    match s.pand.get("/health").await {
        Ok(h) => Json(json!({
            "pand": { "ok": h.get("ok"), "version": h.get("version"), "uptime_secs": h.get("uptime_secs") },
            "pand_url": s.pand.url(),
            "queue": q,
        }))
        .into_response(),
        Err(e) => Json(json!({ "pand": null, "pand_url": s.pand.url(), "error": e, "queue": q })).into_response(),
    }
}

/// Every store pand holds, named after the folder it lives in, with its
/// image count from pand's health.
async fn api_stores(State(s): S) -> Response {
    let (stores, health) = tokio::join!(s.pand.get("/stores"), s.pand.get("/health"));
    let stores = match stores {
        Ok(v) => v,
        Err(e) => return err(StatusCode::BAD_GATEWAY, e),
    };
    let counts: HashMap<String, Value> = health
        .ok()
        .and_then(|h| h.get("counts").cloned())
        .and_then(|c| c.as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|c| Some((c.get("store")?.as_str()?.to_string(), c)))
        .collect();
    let Some(list) = stores.as_array() else {
        return err(StatusCode::BAD_GATEWAY, "pand's /stores is not a list");
    };
    let out: Vec<Value> = list
        .iter()
        .map(|st| {
            let id = st.get("id").and_then(Value::as_str).unwrap_or_default();
            let root = st.get("root").and_then(Value::as_str).unwrap_or_default();
            let c = counts.get(id);
            let n = |k: &str| c.and_then(|c| c.get(k)).and_then(Value::as_u64);
            json!({
                "id": id,
                "name": store_name(root),
                "root": root,
                "is_default": st.get("is_default"),
                "images": n("images"),
                "captions": n("captions"),
                "embeddings": n("embeddings"),
            })
        })
        .collect();
    Json(out).into_response()
}

/// `/…/7R1PL3F0RC3/lUX/.pan` is lUX's store; a bare `.pan` in a home folder
/// is the machine's default store.
fn store_name(root: &str) -> String {
    let trimmed = root.trim_end_matches('/');
    match trimmed.strip_suffix("/.pan") {
        Some(parent) => {
            let home = std::env::var("HOME").unwrap_or_default();
            if !home.is_empty() && parent == home {
                "~/.pan".into()
            } else {
                parent.rsplit('/').next().unwrap_or(parent).to_string()
            }
        }
        None => trimmed.rsplit('/').next().unwrap_or(trimmed).to_string(),
    }
}

/// Where a store's index stands, and its sets.
async fn api_index(State(s): S, AxPath(store): AxPath<String>) -> Response {
    if let Err(r) = check(&store) {
        return r;
    }
    ensure(&s, &store);
    let slots = s.slots.lock().unwrap();
    let slot = slots.get(&store);
    let building = slot.is_some_and(|x| x.building);
    let error = slot.and_then(|x| x.error.clone());
    let Some(ix) = slot.and_then(|x| x.index.clone()) else {
        return Json(json!({ "store": store, "ready": false, "building": building, "error": error })).into_response();
    };
    let ix = ix.read().unwrap();
    Json(json!({
        "store": store,
        "ready": true,
        "building": building,
        "error": error,
        "built": ix.built,
        "count": ix.ids.len(),
        "undated": ix.undated,
        "first": ix.t.first(),
        "last": ix.t.last(),
        "sets": ix.sets,
    }))
    .into_response()
}

#[derive(Deserialize)]
struct ImagesQuery {
    set: Option<String>,
}

/// The images of a store, or of one set in it, in time order. Columns, not
/// rows: 200,000 images is a few megabytes this way.
async fn api_images(State(s): S, AxPath(store): AxPath<String>, Query(q): Query<ImagesQuery>) -> Response {
    if let Err(r) = check(&store) {
        return r;
    }
    let ix = {
        let slots = s.slots.lock().unwrap();
        slots.get(&store).and_then(|x| x.index.clone())
    };
    let Some(ix) = ix else {
        ensure(&s, &store);
        return err(StatusCode::SERVICE_UNAVAILABLE, "this store's index is still being built");
    };
    let ix = ix.read().unwrap();
    let pick: Vec<usize> = match q.set.as_deref().filter(|x| !x.is_empty()) {
        None => (0..ix.ids.len()).collect(),
        Some(set) => match ix.set_position(set) {
            Some(k) => ix.members[k].iter().map(|&p| p as usize).collect(),
            None => return err(StatusCode::NOT_FOUND, format!("no set `{set}` in store {store}")),
        },
    };
    Json(json!({
        "store": store,
        "set": q.set,
        "built": ix.built,
        "ids": pick.iter().map(|&p| &ix.ids[p]).collect::<Vec<_>>(),
        "t": pick.iter().map(|&p| ix.t[p]).collect::<Vec<_>>(),
        "rating": pick.iter().map(|&p| ix.rating[p]).collect::<Vec<_>>(),
        "flag": pick.iter().map(|&p| ix.flag[p]).collect::<Vec<_>>(),
    }))
    .into_response()
}

#[derive(Deserialize)]
struct MarkBody {
    ids: Vec<String>,
    rating: Option<u8>,
    flag: Option<i8>,
}

/// A key press. Answered as soon as it is on disk in the queue; pand gets
/// it a moment later.
async fn api_marks(State(s): S, AxPath(store): AxPath<String>, Json(b): Json<MarkBody>) -> Response {
    if let Err(r) = check(&store) {
        return r;
    }
    if b.rating.is_some_and(|r| r > 5) {
        return err(StatusCode::BAD_REQUEST, "rating is 0 to 5");
    }
    if b.flag.is_some_and(|f| !(-1..=1).contains(&f)) {
        return err(StatusCode::BAD_REQUEST, "flag is -1 (reject), 0 (none) or 1 (pick)");
    }
    if b.rating.is_none() && b.flag.is_none() {
        return err(StatusCode::BAD_REQUEST, "nothing to mark: give rating or flag");
    }
    for id in &b.ids {
        if let Err(r) = check(id) {
            return r;
        }
    }
    let m = Mark { rating: b.rating, flag: b.flag };
    let slots = s.slots.lock().unwrap();
    let ix = slots.get(&store).and_then(|x| x.index.clone());
    for id in &b.ids {
        if let Err(e) = s.queue.push(&store, id, m) {
            return err(StatusCode::INTERNAL_SERVER_ERROR, format!("could not write the save queue: {e}"));
        }
        if let Some(ix) = &ix {
            apply(&mut ix.write().unwrap(), id, m);
        }
    }
    drop(slots);
    Json(json!({ "ok": true, "queue": s.queue.status() })).into_response()
}

async fn api_rebuild(State(s): S, AxPath(store): AxPath<String>) -> Response {
    if let Err(r) = check(&store) {
        return r;
    }
    let busy = s.slots.lock().unwrap().get(&store).is_some_and(|x| x.building);
    if !busy {
        tokio::spawn(rebuild(Arc::clone(&s), store));
    }
    Json(json!({ "ok": true })).into_response()
}

/// Image bytes never change under an id (a rating rewrites the file's
/// metadata, not its pixels), so the browser may keep them as long as it
/// likes. That is what makes the second look at an image free.
async fn pass_bytes(s: &AppState, path: String) -> Response {
    match s.pand.bytes(&path).await {
        Ok((b, ct)) => (
            [(header::CONTENT_TYPE, ct), (header::CACHE_CONTROL, "private, max-age=31536000, immutable".into())],
            b,
        )
            .into_response(),
        Err((code, msg)) => err(StatusCode::from_u16(code).unwrap_or(StatusCode::BAD_GATEWAY), msg),
    }
}

async fn api_thumb(State(s): S, AxPath(id): AxPath<String>) -> Response {
    if let Err(r) = check(&id) {
        return r;
    }
    pass_bytes(&s, format!("/media/{id}/thumbnail")).await
}

async fn api_media(State(s): S, AxPath(id): AxPath<String>) -> Response {
    if let Err(r) = check(&id) {
        return r;
    }
    pass_bytes(&s, format!("/media/{id}")).await
}

async fn api_facts(State(s): S, AxPath(id): AxPath<String>) -> Response {
    if let Err(r) = check(&id) {
        return r;
    }
    match s.pand.get(&format!("/media/{id}/facts")).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => err(StatusCode::BAD_GATEWAY, e),
    }
}

/// pand's config file and caption prompts, as text.
async fn api_config() -> Response {
    let dir = config::dir();
    match tokio::task::spawn_blocking(move || config::read_all(&dir)).await {
        Ok(Ok(files)) => Json(json!({ "dir": config::dir().display().to_string(), "files": files })).into_response(),
        Ok(Err(e)) => err(StatusCode::NOT_FOUND, e),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

#[derive(Deserialize)]
struct SaveBody {
    text: String,
    /// The version the editor loaded; absent to create a new prompt.
    base: Option<u64>,
}

/// Save one config file or prompt. pand reads them only at start, so the
/// reply says so; it is not restarted from here.
async fn api_config_save(AxPath(name): AxPath<String>, Json(b): Json<SaveBody>) -> Response {
    let dir = config::dir();
    let r = tokio::task::spawn_blocking(move || config::save(&dir, &name, &b.text, b.base)).await;
    match r {
        Ok(Ok(saved)) => Json(json!({ "ok": true, "saved": saved, "takes_effect": "at the next pand restart" })).into_response(),
        Ok(Err(config::SaveError::Invalid(e))) => err(StatusCode::UNPROCESSABLE_ENTITY, e),
        Ok(Err(config::SaveError::Changed)) => err(
            StatusCode::CONFLICT,
            "the file changed on disk since you opened it; reload to see the new version (your edit is still in the editor)",
        ),
        Ok(Err(config::SaveError::Io(e))) => err(StatusCode::INTERNAL_SERVER_ERROR, e),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

// ------------------------------------------------------------------ static

async fn index(State(s): S) -> Response {
    match std::fs::read_to_string(s.web_dir.join("index.html")) {
        Ok(html) => Html(html).into_response(),
        Err(_) => Html(format!(
            "<pre style=\"font:14px ui-monospace,monospace;padding:2rem\">\
             pan-ui is running, but the frontend is not built.\n\n\
             Looked in: {}\n\n\
             Build it with:  cd web &amp;&amp; npm install &amp;&amp; npm run build\n\
             Or point elsewhere with:  pan-ui --web &lt;dir&gt;\n</pre>",
            s.web_dir.display()
        ))
        .into_response(),
    }
}

async fn static_asset(State(s): S, AxPath(asset): AxPath<String>) -> Response {
    let rel = PathBuf::from(asset.trim_start_matches('/'));
    let full = s.web_dir.join(&rel);
    let root = s.web_dir.canonicalize().unwrap_or_else(|_| s.web_dir.clone());
    let canon = match full.canonicalize() {
        Ok(c) => c,
        Err(_) => return err(StatusCode::NOT_FOUND, "not found"),
    };
    if !canon.starts_with(&root) {
        return err(StatusCode::FORBIDDEN, "outside the web root");
    }
    let ct = match canon.extension().and_then(|e| e.to_str()) {
        Some("js") | Some("mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("html") => "text/html; charset=utf-8",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    };
    match std::fs::read(&canon) {
        Ok(bytes) => ([(header::CONTENT_TYPE, ct)], bytes).into_response(),
        Err(_) => err(StatusCode::NOT_FOUND, "not found"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_stores_by_folder() {
        assert_eq!(store_name("/Volumes/f00/repos/7R1PL3F0RC3/lUX/.pan"), "lUX");
        assert_eq!(store_name("/somewhere/else/pan-store"), "pan-store");
        if let Ok(home) = std::env::var("HOME") {
            assert_eq!(store_name(&format!("{home}/.pan")), "~/.pan");
        }
    }
}
