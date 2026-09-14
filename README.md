# Pan UI (`panV`)

> **The Dual-Control Visual Interface for Pan Instances & Semantic Photographic Knowledge Graphs**

`pan-ui` is the visual frontend for exploring, curating, and reasoning over Pan instances. It runs locally as a fast web application or desktop webview, featuring a **dual-control interface** where both humans (via UI gestures and hotkeys) and autonomous AI agents (via the `pansee` command-line tool) steer the shared viewport in real time.

---

## Key Architecture & Features

### 1. Dual-Control Interface (Human + Agent)
Inspired by the `cosee` / `coquette` interface in Copia:
* **Human-Driven:** Interactive navigation via shoots ("dumb folders"), disk-backed Photosets (XML), smart collections (SPARQL), and quick single-keypress triage (`1-5`, `P`, `X`, `V`, `Space`).
* **Agent-Driven (`pansee`):** An LLM or background agent can query Oxigraph, find candidate sets or near-neighbors, and **cast** queries, photosets, or specific images live to the human's screen.
* **Trust & Shared Context:** Whenever an agent casts to the screen, a high-visibility **Agent Cast Banner** appears, informing the user of the agent's intent (e.g., `⚡ @kira cast SPARQL: ?s pan:rating 5`).

### 2. Local Pan Instance Switcher
Switch seamlessly between different local `pand` daemons running across your workstation or mesh:
* 📷 **Studio 2026:** Mode 2 (Referenced Indexer) active dance & portrait sessions (`:7401`).
* 🗄️ **Primary Archive:** Mode 2 (Referenced Indexer) 1.2M historical photo catalog (`:7402`).
* 🎨 **Horae Generative Pool:** Mode 1 (Managed Store) agent synthetic renders & LoRA runs (`:7403`).

### 3. Dual View Modes
* **Gallery Grid Mode:** 120fps responsive image grid with 4K preview inspection, motor-drive burst stack badges (`[⧉ 12]`), star ratings, and pick flags.
* **RDF Knowledge Graph Mode:** Interactive force-directed graph canvas visualizing relationships between Images, Shoots, Photosets, Subjects/Models, Poses, and Optics via RDF predicates (`pan:inPhotoset`, `pan:hasSubject`, `pan:hasLens`, `pan:poseCategory`).

### 4. The Pan Visual Proxy Inspector
Side-panel inspector detailing the image's multi-modal perceptual representation:
* **Aesthetic Consensus:** Dual-model score (Qwen 27B + Gemma) and critique summary.
* **Kinematics & Pose:** 133-keypoint verification (RTMW) and posture classification.
* **Segmented Objects:** SAM 3 bounding box proposal tags.
* **Optics & Color:** Camera, lens, exposure, and Display P3 wide-gamut indicator.

---

## Getting Started

### 1. Start the Local Server
The mock server uses the Python 3 standard library with zero external dependencies:

```bash
cd repolex-ai/pan-ui
python3 server.py
```

This starts the server at **`http://127.0.0.1:7401`** with Server-Sent Events (SSE) broadcasting enabled.

### 2. Open in Your Browser
Visit `http://localhost:7401` in Safari, Chrome, or Arc.

### 3. Drive the UI with `pansee` (Agent Dual-Control)
In a separate terminal, use `pansee` to steer the browser window live:

```bash
# Check server status
./pansee status

# Cast a SPARQL search filter
./pansee cast sparql "SELECT ?s WHERE { ?s pan:rating 5 }"

# Cast a specific Photoset
./pansee cast set s_dance_portfolio_2026

# Switch view to the interactive RDF Knowledge Graph
./pansee cast view graph

# Switch view back to the Grid
./pansee cast view grid

# Select and inspect a specific image
./pansee cast image img_06

# Switch active Pan instance
./pansee cast instance primary-archive
```

---

## File Structure

```
repolex-ai/pan-ui/
├── README.md               # Architecture & usage guide
├── server.py               # Lightweight Python HTTP + SSE broadcast server
├── pansee                  # Executable CLI tool for agent dual-control
└── public/                 # Web assets
    ├── index.html          # Main application shell
    ├── style.css           # Photography dark theme (Display P3 aware)
    ├── app.js              # Application state, SPARQL runner, SSE handler
    ├── graph.js            # Force-directed HTML5 Canvas RDF visualizer
    └── mock_data.js        # Pan instance configurations, images, proxies, triples
```
