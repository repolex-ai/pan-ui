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
mod lab;
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
    /// The Lab's precomputed data, per store, read on first use.
    labs: lab::Labs,
    /// Every image's caption, per store, for the Lab's word search: lower
    /// case, read from words.json on first use.
    words: Mutex<HashMap<String, Arc<Words>>>,
    /// The sets graph, per store, with the index build it was made from.
    set_graphs: Mutex<HashMap<String, (String, Arc<Value>)>>,
    /// A soul's finest memory notes from raveld, per store, with when they
    /// were fetched.
    notes: Mutex<HashMap<String, (std::time::Instant, Arc<Value>)>>,
    /// The properties a store's images carry, with counts: a six-second
    /// question on a big store, so asked once per index build.
    props: Mutex<HashMap<String, (String, Arc<Value>)>>,
}

struct Words {
    ids: Vec<String>,
    text: Vec<String>,
    lower: Vec<String>,
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
        labs: lab::Labs::default(),
        words: Mutex::new(HashMap::new()),
        set_graphs: Mutex::new(HashMap::new()),
        notes: Mutex::new(HashMap::new()),
        props: Mutex::new(HashMap::new()),
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
        .route("/api/stores/{store}/sets-of/{id}", get(api_sets_of))
        .route("/api/thumb/{id}", get(api_thumb))
        .route("/api/media/{id}", get(api_media))
        .route("/api/facts/{id}", get(api_facts))
        .route("/api/lab/{store}/status", get(api_lab_status))
        .route("/api/lab/{store}/file/{name}", get(api_lab_file))
        .route("/api/lab/{store}/near", post(api_lab_near))
        .route("/api/lab/{store}/stick/{id}", get(api_lab_stick))
        .route("/api/lab/{store}/words", get(api_lab_words))
        .route("/api/lab/{store}/hybrid", post(api_lab_hybrid))
        .route("/api/lab/{store}/sets", get(api_lab_sets))
        .route("/api/lab/{store}/notes", get(api_lab_notes))
        .route("/api/lab/{store}/props", get(api_lab_props))
        .route("/api/lab/{store}/facet", get(api_lab_facet).post(api_lab_facet_images))
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

/// Which sets of the store hold this image: the right panel's "Sets", and
/// how a set is opened on the image already under the cursor.
async fn api_sets_of(State(s): S, AxPath((store, id)): AxPath<(String, String)>) -> Response {
    if let Err(r) = check(&store).and_then(|_| check(&id)) {
        return r;
    }
    let ix = {
        let slots = s.slots.lock().unwrap();
        slots.get(&store).and_then(|x| x.index.clone())
    };
    let Some(ix) = ix else {
        return err(StatusCode::SERVICE_UNAVAILABLE, "this store's index is still being built");
    };
    let ix = ix.read().unwrap();
    Json(json!({ "id": id, "sets": ix.sets_of(&id) })).into_response()
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
// ------------------------------------------------------------------ the Lab

/// Load a store's Lab data off the async threads: the first read of a big
/// store is a few hundred megabytes.
async fn lab_for(s: &Arc<AppState>, store: &str) -> Result<Arc<lab::Lab>, Response> {
    check(store)?;
    let s2 = Arc::clone(s);
    let st = store.to_string();
    tokio::task::spawn_blocking(move || s2.labs.get(&s2.data, &st))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| err(StatusCode::NOT_FOUND, e))
}

/// What the Lab has for a store, and how far behind pand it is: new
/// embeddings land all the time, and the Lab's copy is a snapshot.
async fn api_lab_status(State(s): S, AxPath(store): AxPath<String>) -> Response {
    if let Err(r) = check(&store) {
        return r;
    }
    let d = lab::dir(&s.data, &store);
    let meta: Option<Value> = std::fs::read(d.join("meta.json")).ok().and_then(|b| serde_json::from_slice(&b).ok());
    let files: Vec<&str> = lab::FILES.iter().chain(["words.json"].iter()).copied().filter(|f| d.join(f).exists()).collect();
    let now = s.pand.get("/health").await.ok().and_then(|h| {
        h["counts"].as_array()?.iter().find(|c| c["store"] == store.as_str()).map(|c| c["embeddings"].clone())
    });
    Json(json!({
        "store": store,
        "ready": meta.is_some(),
        "n": meta.as_ref().map(|m| m["n"].clone()),
        "dim": meta.as_ref().map(|m| m["dim"].clone()),
        "built": meta.as_ref().map(|m| m["built"].clone()),
        "atlas": meta.as_ref().map(|m| m["atlas"].clone()),
        "files": files,
        "embeddings_now": now,
        "rebuild": format!("tools/.venv/bin/python tools/lab_build.py {store} && tools/.venv/bin/python tools/lab_words.py {store}"),
    }))
    .into_response()
}

async fn api_lab_file(State(s): S, AxPath((store, name)): AxPath<(String, String)>) -> Response {
    if let Err(r) = check(&store) {
        return r;
    }
    if !lab::FILES.contains(&name.as_str()) {
        return err(StatusCode::NOT_FOUND, format!("the Lab has no file `{name}`"));
    }
    match tokio::fs::read(lab::dir(&s.data, &store).join(&name)).await {
        Ok(b) => {
            let ct = if name.ends_with(".json") { "application/json" } else { "application/octet-stream" };
            ([(header::CONTENT_TYPE, ct), (header::CACHE_CONTROL, "no-cache")], b).into_response()
        }
        Err(e) => err(StatusCode::NOT_FOUND, format!("{name}: {e}")),
    }
}

/// The image ids of the sets that hold `id`, so a search can keep inside
/// them or out of them.
fn same_set_ids(s: &AppState, store: &str, id: &str) -> Option<std::collections::HashSet<String>> {
    let ix = s.slots.lock().unwrap().get(store).and_then(|x| x.index.clone())?;
    let ix = ix.read().unwrap();
    let &p = ix.pos.get(id)?;
    let mut out = std::collections::HashSet::new();
    for m in &ix.members {
        if m.binary_search(&p).is_ok() {
            out.extend(m.iter().map(|&q| ix.ids[q as usize].clone()));
        }
    }
    Some(out)
}

#[derive(Deserialize)]
struct NearBody {
    /// Images to move towards, each with a weight (1 if left out).
    #[serde(default)]
    plus: Vec<String>,
    #[serde(default)]
    plus_w: Vec<f32>,
    /// Images to move away from.
    #[serde(default)]
    minus: Vec<String>,
    #[serde(default)]
    skip: Vec<String>,
    k: Option<usize>,
    /// "inside" keeps to the sets that hold plus[0]; "outside" leaves them.
    scope: Option<String>,
    /// 0..1: noise added to the query, for wandering.
    #[serde(default)]
    jitter: f32,
}

/// Images like these, and unlike those: the sum of the plus vectors minus
/// the minus vectors, ranked against every image in the Lab's copy.
async fn api_lab_near(State(s): S, AxPath(store): AxPath<String>, Json(b): Json<NearBody>) -> Response {
    let l = match lab_for(&s, &store).await {
        Ok(l) => l,
        Err(r) => return r,
    };
    if b.plus.is_empty() {
        return err(StatusCode::BAD_REQUEST, "give at least one image in `plus`");
    }
    let mut q = vec![0f32; l.dim];
    for (i, id) in b.plus.iter().enumerate() {
        let Some(v) = l.of(id) else { return err(StatusCode::NOT_FOUND, format!("no vector for {id}")) };
        let w = b.plus_w.get(i).copied().unwrap_or(1.0);
        q.iter_mut().zip(v).for_each(|(a, x)| *a += w * x);
    }
    for id in &b.minus {
        let Some(v) = l.of(id) else { return err(StatusCode::NOT_FOUND, format!("no vector for {id}")) };
        q.iter_mut().zip(v).for_each(|(a, x)| *a -= x);
    }
    lab::normalise(&mut q);
    if b.jitter > 0.0 {
        let mut rng = lab::Rng::new();
        let sd = b.jitter / (l.dim as f32).sqrt();
        q.iter_mut().for_each(|a| *a += rng.gauss() * sd);
        lab::normalise(&mut q);
    }
    let mut skip: std::collections::HashSet<usize> =
        b.plus.iter().chain(&b.minus).chain(&b.skip).filter_map(|id| l.pos.get(id).map(|&p| p as usize)).collect();
    let k = b.k.unwrap_or(24).min(500);
    let scope = b.scope.as_deref().unwrap_or("all");
    if scope != "all" {
        let Some(same) = same_set_ids(&s, &store, &b.plus[0]) else {
            ensure(&s, &store);
            return err(StatusCode::SERVICE_UNAVAILABLE, "this store's index is still being built");
        };
        for (i, id) in l.ids.iter().enumerate() {
            if same.contains(id) != (scope == "inside") {
                skip.insert(i);
            }
        }
    }
    let hits: Vec<Value> =
        l.nearest(&q, k, &skip).into_iter().map(|(i, sim)| json!({"id": l.ids[i], "sim": sim})).collect();
    Json(json!({ "hits": hits, "of": l.ids.len() })).into_response()
}

#[derive(Deserialize)]
struct StickQuery {
    stride: Option<f32>,
    fuzz: Option<f32>,
    /// Comma-separated ids not to land on: where you have been.
    skip: Option<String>,
}

/// The joystick: from an image, one step in each of eight directions. The
/// directions are the two along which the images around it vary most (the
/// top principal components of its neighbourhood), and their diagonals.
async fn api_lab_stick(State(s): S, AxPath((store, id)): AxPath<(String, String)>, Query(q): Query<StickQuery>) -> Response {
    let l = match lab_for(&s, &store).await {
        Ok(l) => l,
        Err(r) => return r,
    };
    let Some(c) = l.of(&id) else { return err(StatusCode::NOT_FOUND, format!("no vector for {id}")) };
    let c = c.to_vec();
    let stride = q.stride.unwrap_or(0.5).clamp(0.05, 3.0);
    let fuzz = q.fuzz.unwrap_or(0.0).clamp(0.0, 1.0);
    let [a1, a2] = l.local_axes(&c, 48);
    let mut skip: std::collections::HashSet<usize> = q
        .skip
        .as_deref()
        .unwrap_or("")
        .split(',')
        .filter_map(|x| l.pos.get(x).map(|&p| p as usize))
        .collect();
    if let Some(&p) = l.pos.get(&id) {
        skip.insert(p as usize);
    }
    let mut rng = lab::Rng::new();
    let mut dirs = Vec::new();
    for (dx, dy) in [(0i8, -1i8), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1)] {
        let (fx, fy) = (dx as f32, dy as f32);
        let norm = (fx * fx + fy * fy).sqrt();
        let mut t: Vec<f32> =
            (0..l.dim).map(|i| c[i] + stride * (fx * a1[i] + fy * a2[i]) / norm).collect();
        lab::normalise(&mut t);
        // Fuzz: pick among the first few matches rather than always the
        // first, so the same move can land somewhere new.
        let depth = 1 + (fuzz * 8.0).round() as usize;
        let near = l.nearest(&t, depth, &skip);
        if near.is_empty() {
            continue;
        }
        let pick = near[((rng.next() * near.len() as f32) as usize).min(near.len() - 1)];
        skip.insert(pick.0);
        dirs.push(json!({"dx": dx, "dy": dy, "id": l.ids[pick.0], "sim": lab::dot(l.row(pick.0), &c)}));
    }
    Json(json!({ "centre": id, "dirs": dirs })).into_response()
}

fn words_for(s: &AppState, store: &str) -> Result<Arc<Words>, String> {
    if let Some(w) = s.words.lock().unwrap().get(store) {
        return Ok(Arc::clone(w));
    }
    #[derive(Deserialize)]
    struct Raw {
        ids: Vec<String>,
        text: Vec<String>,
    }
    let path = lab::dir(&s.data, store).join("words.json");
    let raw: Raw = serde_json::from_slice(&std::fs::read(&path).map_err(|e| {
        format!("no captions for store {store} yet ({e}): run tools/.venv/bin/python tools/lab_words.py {store}")
    })?)
    .map_err(|e| format!("words.json: {e}"))?;
    let lower = raw.text.iter().map(|t| t.to_lowercase()).collect();
    let w = Arc::new(Words { ids: raw.ids, text: raw.text, lower });
    s.words.lock().unwrap().insert(store.to_string(), Arc::clone(&w));
    Ok(w)
}

#[derive(Deserialize)]
struct WordsQuery {
    q: String,
    k: Option<usize>,
}

/// Images whose caption holds every word of the query; a word starting
/// with - must not be there. Newest first, because ids are not in time
/// order but the index knows when each image was made.
/// Whether a word starts somewhere in the text: "harp" finds "harps", not
/// "sharp".
fn has_word(text: &str, word: &str) -> bool {
    text.match_indices(word).any(|(i, _)| !text[..i].chars().next_back().is_some_and(|c| c.is_alphanumeric()))
}

fn word_hits(s: &AppState, store: &str, q: &str, k: usize) -> Result<(Vec<String>, usize), String> {
    let w = words_for(s, store)?;
    let (mut need, mut not) = (Vec::new(), Vec::new());
    for t in q.to_lowercase().split_whitespace() {
        match t.strip_prefix('-') {
            Some(x) if !x.is_empty() => not.push(x.to_string()),
            _ => need.push(t.to_string()),
        }
    }
    if need.is_empty() {
        return Err("give at least one word".into());
    }
    let hits: Vec<usize> = (0..w.ids.len())
        .filter(|&i| need.iter().all(|t| has_word(&w.lower[i], t)) && !not.iter().any(|t| has_word(&w.lower[i], t)))
        .collect();
    let total = hits.len();
    let when = |id: &str| -> i64 {
        s.slots.lock().unwrap().get(store).and_then(|x| x.index.clone()).and_then(|ix| {
            let ix = ix.read().unwrap();
            ix.pos.get(id).map(|&p| ix.t[p as usize])
        }).unwrap_or(0)
    };
    let mut ranked: Vec<(i64, &String)> = hits.iter().map(|&i| (when(&w.ids[i]), &w.ids[i])).collect();
    ranked.sort_by(|a, b| b.0.cmp(&a.0));
    Ok((ranked.into_iter().take(k).map(|(_, id)| id.clone()).collect(), total))
}

async fn api_lab_words(State(s): S, AxPath(store): AxPath<String>, Query(q): Query<WordsQuery>) -> Response {
    if let Err(r) = check(&store) {
        return r;
    }
    ensure(&s, &store);
    let k = q.k.unwrap_or(60).min(1000);
    let s2 = Arc::clone(&s);
    let res = tokio::task::spawn_blocking(move || {
        let r = word_hits(&s2, &store, &q.q, k)?;
        let w = words_for(&s2, &store)?;
        let pos: HashMap<&str, usize> = w.ids.iter().enumerate().map(|(i, x)| (x.as_str(), i)).collect();
        let caps: Vec<&str> = r.0.iter().map(|id| pos.get(id.as_str()).map(|&i| w.text[i].as_str()).unwrap_or("")).collect();
        Ok::<Value, String>(json!({"ids": r.0, "captions": caps, "total": r.1}))
    })
    .await;
    match res {
        Ok(Ok(v)) => Json(v).into_response(),
        Ok(Err(e)) => err(StatusCode::NOT_FOUND, e),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

#[derive(Deserialize)]
struct HybridBody {
    q: String,
    k: Option<usize>,
    /// How many caption matches make the seed.
    seeds: Option<usize>,
}

/// Words, then looks: the images whose captions match, then the images
/// that look like them (the mean of the matches' vectors) whose captions do
/// not match at all. The second list is what a word search alone never
/// finds.
async fn api_lab_hybrid(State(s): S, AxPath(store): AxPath<String>, Json(b): Json<HybridBody>) -> Response {
    ensure(&s, &store);
    let l = match lab_for(&s, &store).await {
        Ok(l) => l,
        Err(r) => return r,
    };
    let s2 = Arc::clone(&s);
    let st = store.clone();
    let q = b.q.clone();
    let words = match tokio::task::spawn_blocking(move || word_hits(&s2, &st, &q, 100_000)).await {
        Ok(Ok(w)) => w,
        Ok(Err(e)) => return err(StatusCode::NOT_FOUND, e),
        Err(e) => return err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    let (said, total) = words;
    // Seeds spread evenly through the matches, oldest to newest: the
    // vectors cluster hard by the batch an image was made in, so forty
    // seeds from one evening would only find that evening again.
    let have: Vec<usize> = said.iter().filter_map(|id| l.pos.get(id).map(|&p| p as usize)).collect();
    let want = b.seeds.unwrap_or(40).min(have.len());
    let seeds: Vec<usize> = (0..want).map(|j| have[j * have.len() / want.max(1)]).collect();
    if seeds.is_empty() {
        return Json(json!({"said": said.iter().take(60).collect::<Vec<_>>(), "total": total, "looks": [], "seeds": 0}))
            .into_response();
    }
    let mut q = vec![0f32; l.dim];
    for &i in &seeds {
        q.iter_mut().zip(l.row(i)).for_each(|(a, x)| *a += x);
    }
    lab::normalise(&mut q);
    let said_set: std::collections::HashSet<&str> = said.iter().map(|x| x.as_str()).collect();
    let skip: std::collections::HashSet<usize> =
        l.ids.iter().enumerate().filter(|(_, id)| said_set.contains(id.as_str())).map(|(i, _)| i).collect();
    let k = b.k.unwrap_or(60).min(500);
    let looks: Vec<Value> =
        l.nearest(&q, k, &skip).into_iter().map(|(i, sim)| json!({"id": l.ids[i], "sim": sim})).collect();
    Json(json!({
        "said": said.iter().take(k).collect::<Vec<_>>(),
        "total": total,
        "seeds": seeds.len(),
        "looks": looks,
    }))
    .into_response()
}

/// Every set as one node: where its images sit on the Atlas on average, and
/// the sets it looks most like (cosine between the sets' mean vectors).
/// Worked out once per index build and kept.
async fn api_lab_sets(State(s): S, AxPath(store): AxPath<String>) -> Response {
    let l = match lab_for(&s, &store).await {
        Ok(l) => l,
        Err(r) => return r,
    };
    let Some(ix) = s.slots.lock().unwrap().get(&store).and_then(|x| x.index.clone()) else {
        ensure(&s, &store);
        return err(StatusCode::SERVICE_UNAVAILABLE, "this store's index is still being built");
    };
    let key = format!("{}:{}", ix.read().unwrap().built, l.ids.len());
    if let Some((k, v)) = s.set_graphs.lock().unwrap().get(&store) {
        if *k == key {
            return Json(v.as_ref().clone()).into_response();
        }
    }
    let atlas_path = lab::dir(&s.data, &store).join("atlas.f32");
    let st = store.clone();
    let built = tokio::task::spawn_blocking(move || {
        let atlas: Vec<f32> = std::fs::read(&atlas_path)
            .map(|raw| raw.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect())
            .unwrap_or_default();
        let ix = ix.read().unwrap();
        let mut nodes = Vec::new();
        let mut cent: Vec<Vec<f32>> = Vec::new();
        for (k, info) in ix.sets.iter().enumerate() {
            let rows: Vec<usize> = ix.members[k]
                .iter()
                .filter_map(|&p| l.pos.get(&ix.ids[p as usize]).map(|&r| r as usize))
                .collect();
            let mut v = vec![0f32; l.dim];
            let (mut x, mut y) = (0f32, 0f32);
            for &r in &rows {
                v.iter_mut().zip(l.row(r)).for_each(|(a, b)| *a += b);
                if atlas.len() >= 2 * (r + 1) {
                    x += atlas[2 * r];
                    y += atlas[2 * r + 1];
                }
            }
            lab::normalise(&mut v);
            let m = rows.len().max(1) as f32;
            nodes.push(json!({
                "id": info.id, "count": info.count, "first": info.first, "last": info.last,
                "on_map": rows.len(),
                "x": if rows.is_empty() { Value::Null } else { json!(x / m) },
                "y": if rows.is_empty() { Value::Null } else { json!(y / m) },
                "cover": rows.first().map(|&r| l.ids[r].clone()),
            }));
            cent.push(if rows.is_empty() { Vec::new() } else { v });
        }
        // Each set's six most alike sets.
        for i in 0..cent.len() {
            if cent[i].is_empty() {
                continue;
            }
            let mut best: Vec<(usize, f32)> = (0..cent.len())
                .filter(|&j| j != i && !cent[j].is_empty())
                .map(|j| (j, lab::dot(&cent[i], &cent[j])))
                .collect();
            best.sort_by(|a, b| b.1.total_cmp(&a.1));
            best.truncate(6);
            nodes[i]["near"] = json!(best.iter().map(|&(j, sim)| json!({"i": j, "sim": sim})).collect::<Vec<_>>());
        }
        json!({ "store": st, "sets": nodes })
    })
    .await;
    match built {
        Ok(v) => {
            let v = Arc::new(v);
            s.set_graphs.lock().unwrap().insert(store, (key, Arc::clone(&v)));
            Json(v.as_ref().clone()).into_response()
        }
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

/// An IRI fit to put between angle brackets in a query, or None.
fn safe_iri(p: &str) -> Option<&str> {
    let ok = p.starts_with("http") && !p.chars().any(|c| matches!(c, '<' | '>' | '"' | '{' | '}' | '|' | '\\' | '^' | '`') || c.is_whitespace());
    ok.then_some(p)
}

/// A SPARQL term from a results binding: an IRI, or a literal with its
/// datatype or language, escaped.
fn term(b: &Value) -> Option<String> {
    let v = b["value"].as_str()?;
    match b["type"].as_str()? {
        "uri" => safe_iri(v).map(|x| format!("<{x}>")),
        "literal" | "typed-literal" => {
            let esc = v.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "\\r");
            Some(match (b["datatype"].as_str(), b["xml:lang"].as_str()) {
                (_, Some(l)) => format!("\"{esc}\"@{l}"),
                (Some(d), _) => format!("\"{esc}\"^^<{}>", safe_iri(d)?),
                _ => format!("\"{esc}\""),
            })
        }
        _ => None,
    }
}

/// Every property the store's images carry, with how many statements use
/// it, most used first.
async fn api_lab_props(State(s): S, AxPath(store): AxPath<String>) -> Response {
    if let Err(r) = check(&store) {
        return r;
    }
    ensure(&s, &store);
    let built = s.slots.lock().unwrap().get(&store).and_then(|x| x.index.clone()).map(|ix| ix.read().unwrap().built.clone());
    let built = built.unwrap_or_default();
    if let Some((b, v)) = s.props.lock().unwrap().get(&store) {
        if *b == built {
            return Json(v.as_ref().clone()).into_response();
        }
    }
    let q = "SELECT ?p (COUNT(*) AS ?n) WHERE { ?m a <https://repolex.ai/ontology/pan/Image> ; ?p ?v } GROUP BY ?p ORDER BY DESC(?n)";
    match s.pand.select(&store, &["p", "n"], q).await {
        Ok(rows) => {
            let props: Vec<Value> = rows
                .into_iter()
                .filter_map(|r| Some(json!({"p": r[0].clone()?, "n": r[1].as_deref()?.parse::<u64>().ok()?})))
                .collect();
            let v = Arc::new(json!({ "props": props }));
            s.props.lock().unwrap().insert(store, (built, Arc::clone(&v)));
            Json(v.as_ref().clone()).into_response()
        }
        Err(e) => err(StatusCode::BAD_GATEWAY, e),
    }
}

#[derive(Deserialize)]
struct FacetQuery {
    p: String,
}

/// The commonest values of one property, with counts.
async fn api_lab_facet(State(s): S, AxPath(store): AxPath<String>, Query(q): Query<FacetQuery>) -> Response {
    if let Err(r) = check(&store) {
        return r;
    }
    let Some(p) = safe_iri(&q.p) else { return err(StatusCode::BAD_REQUEST, "not a property IRI") };
    let query = format!(
        "SELECT ?v (COUNT(*) AS ?n) WHERE {{ ?m a <https://repolex.ai/ontology/pan/Image> ; <{p}> ?v }} GROUP BY ?v ORDER BY DESC(?n) LIMIT 80"
    );
    match s.pand.select_terms(&store, &query).await {
        Ok(rows) => Json(json!({ "p": p, "values": rows })).into_response(),
        Err(e) => err(StatusCode::BAD_GATEWAY, e),
    }
}

#[derive(Deserialize)]
struct FacetImagesBody {
    p: String,
    /// A value exactly as /facet returned it.
    v: Value,
}

/// The images with that value for that property, newest first.
async fn api_lab_facet_images(State(s): S, AxPath(store): AxPath<String>, Json(b): Json<FacetImagesBody>) -> Response {
    if let Err(r) = check(&store) {
        return r;
    }
    let (Some(p), Some(v)) = (safe_iri(&b.p), term(&b.v)) else {
        return err(StatusCode::BAD_REQUEST, "not a property and value");
    };
    let query = format!("SELECT ?m WHERE {{ ?m a <https://repolex.ai/ontology/pan/Image> ; <{p}> {v} }}");
    let rows = match s.pand.select(&store, &["m"], &query).await {
        Ok(r) => r,
        Err(e) => return err(StatusCode::BAD_GATEWAY, e),
    };
    let ids: Vec<String> =
        rows.into_iter().filter_map(|r| r[0].as_deref().map(|m| m.trim_end_matches('>').rsplit('/').next().unwrap_or(m).to_string())).collect();
    let total = ids.len();
    let ix = s.slots.lock().unwrap().get(&store).and_then(|x| x.index.clone());
    let mut dated: Vec<(i64, String)> = match ix {
        Some(ix) => {
            let ix = ix.read().unwrap();
            ids.into_iter().map(|id| (ix.pos.get(&id).map(|&q| ix.t[q as usize]).unwrap_or(0), id)).collect()
        }
        None => ids.into_iter().map(|id| (0, id)).collect(),
    };
    dated.sort_by(|a, b| b.0.cmp(&a.0));
    Json(json!({ "total": total, "ids": dated.into_iter().take(400).map(|x| x.1).collect::<Vec<_>>() })).into_response()
}

/// Unix seconds from an RFC 3339 time, as raveld writes them.
fn rfc3339_secs(x: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(x).ok().map(|d| d.timestamp())
}

/// Where ravel's daemon listens. A soul's Pan store and its ravel record
/// share the soul's id, so a store id is also a soul id.
const RAVELD: &str = "http://127.0.0.1:7881";

/// The soul's finest-grained memory notes (level 0, each a few turns of its
/// conversation), with the span of time each covers, oldest first. From
/// raveld's memory graph; kept for ten minutes.
async fn api_lab_notes(State(s): S, AxPath(store): AxPath<String>) -> Response {
    if let Err(r) = check(&store) {
        return r;
    }
    if let Some((at, v)) = s.notes.lock().unwrap().get(&store) {
        if at.elapsed() < Duration::from_secs(600) {
            return Json(v.as_ref().clone()).into_response();
        }
    }
    let q = "PREFIX r: <https://repolex.ai/ontology/ravel/> \
             SELECT ?s ?e ?t WHERE { GRAPH <https://repolex.ai/ravel/NamedGraph/memory-v1> { \
             ?m r:memoryLevel 0 ; r:windowStart ?s ; r:windowEnd ?e ; r:memoryText ?t } } ORDER BY ?s";
    let reply = reqwest::Client::new()
        .post(format!("{RAVELD}/souls/{store}/query"))
        .json(&json!({ "query": q }))
        .timeout(Duration::from_secs(120))
        .send()
        .await;
    let body: Value = match reply {
        Ok(r) if r.status().is_success() => match r.json().await {
            Ok(v) => v,
            Err(e) => return err(StatusCode::BAD_GATEWAY, format!("raveld's answer was not JSON: {e}")),
        },
        Ok(r) => {
            let code = r.status();
            let text = r.text().await.unwrap_or_default();
            return err(StatusCode::NOT_FOUND, format!("raveld has no memory for {store} ({code}): {text}"));
        }
        Err(e) => return err(StatusCode::BAD_GATEWAY, format!("raveld is not answering on {RAVELD}: {e}")),
    };
    let secs = |v: &Value| v["value"].as_str().and_then(rfc3339_secs);
    // An answer without a results list is raveld saying something else
    // (busy, an error in a 200): pass it on rather than keep an empty
    // diary for ten minutes.
    let Some(rows) = body["results"]["bindings"].as_array().cloned() else {
        let head: String = body.to_string().chars().take(300).collect();
        return err(StatusCode::BAD_GATEWAY, format!("raveld answered without results: {head}"));
    };
    let (mut start, mut end, mut text) = (Vec::new(), Vec::new(), Vec::new());
    for b in &rows {
        if let (Some(a), Some(z), Some(t)) = (secs(&b["s"]), secs(&b["e"]), b["t"]["value"].as_str()) {
            start.push(a);
            end.push(z);
            text.push(t.to_string());
        }
    }
    let keep = !start.is_empty();
    let v = Arc::new(json!({ "soul": store, "start": start, "end": end, "text": text }));
    if keep {
        s.notes.lock().unwrap().insert(store, (std::time::Instant::now(), Arc::clone(&v)));
    }
    Json(v.as_ref().clone()).into_response()
}

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

    #[test]
    fn terms_go_back_exactly() {
        use serde_json::json;
        assert_eq!(super::term(&json!({"type": "literal", "value": "serene"})).as_deref(), Some("\"serene\""));
        assert_eq!(
            super::term(&json!({"type": "literal", "value": "4", "datatype": "http://www.w3.org/2001/XMLSchema#integer"})).as_deref(),
            Some("\"4\"^^<http://www.w3.org/2001/XMLSchema#integer>")
        );
        assert_eq!(super::term(&json!({"type": "literal", "value": "a \"q\""})).as_deref(), Some("\"a \\\"q\\\"\""));
        assert_eq!(super::term(&json!({"type": "uri", "value": "https://x/y"})).as_deref(), Some("<https://x/y>"));
        assert_eq!(super::term(&json!({"type": "uri", "value": "https://x/> } DROP"})), None);
    }

    #[test]
    fn reads_raveld_times() {
        assert_eq!(rfc3339_secs("2026-04-09T16:44:59.221Z"), Some(1775753099));
        assert_eq!(rfc3339_secs("2026-04-09T16:44:59Z"), Some(1775753099));
        assert_eq!(rfc3339_secs("nonsense"), None);
    }

    #[test]
    fn words_match_at_their_start() {
        assert!(has_word("a small harp", "harp"));
        assert!(has_word("two harps", "harp"));
        assert!(has_word("harp at the start", "harp"));
        assert!(!has_word("a sharp edge", "harp"));
        assert!(has_word("a sharp edge and a harp", "harp"));
    }
}
