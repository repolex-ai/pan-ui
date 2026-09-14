import { PAN_INSTANCES, MOCK_SHOOTS, MOCK_PHOTOSETS, MOCK_SMART_COLLECTIONS, MOCK_IMAGES, MOCK_GRAPH_TRIPLES } from './mock_data.js';
import { PanGraphViewer } from './graph.js';

class PanApp {
  constructor() {
    this.instances = PAN_INSTANCES;
    this.currentInstance = this.instances[0]; // studio-2026
    this.currentView = 'grid'; // 'grid' | 'graph'
    this.filter = { type: 'all', label: 'All Photos' };
    this.images = [...MOCK_IMAGES];
    this.selectedImage = this.images[0];
    this.agentCast = null;

    // pand Daemon Live Telemetry State (SQLite Indexing Queue)
    this.pandTelemetry = {
      state: 'indexing',
      currentTask: 'Extracting 4K previews: frame 1,420 of 3,200',
      activeShoot: '2026/05_studio_dance',
      runnerStatus: 'Salad 3060: Qwen 27B active',
      progress: 44.3,
      queueStats: {
        discovered: 3420,
        extracted: 1420,
        enriched: 840,
        pending: 1780
      }
    };

    this.graphViewer = null;
    this.initDOM();
    this.initSSE();
    this.initShortcuts();
    this.render();
  }

  initDOM() {
    // Topbar controls
    this.instanceBtn = document.getElementById('btn-instance-select');
    this.instanceDrawer = document.getElementById('instance-drawer');
    this.currentInstanceLabel = document.getElementById('current-instance-name');
    this.currentInstancePort = document.getElementById('current-instance-port');
    this.omniboxInput = document.getElementById('omnibox-input');
    this.gridBtn = document.getElementById('btn-view-grid');
    this.graphBtn = document.getElementById('btn-view-graph');
    this.agentBanner = document.getElementById('agent-banner');

    // pand Status Telemetry DOM
    this.statusTicker = document.getElementById('pand-status-ticker');
    this.statusDot = document.getElementById('pand-status-dot');
    this.statusText = document.getElementById('pand-status-text');
    this.queueBadge = document.getElementById('pand-queue-badge');
    this.telemetryDrawer = document.getElementById('telemetry-drawer');

    // Sidebar lists
    this.listShoots = document.getElementById('list-shoots');
    this.listPhotosets = document.getElementById('list-photosets');
    this.listSmart = document.getElementById('list-smart');

    // Center view
    this.galleryGrid = document.getElementById('gallery-grid');
    this.graphContainer = document.getElementById('graph-container');
    this.canvasElement = document.getElementById('rdf-canvas');
    this.viewTitle = document.getElementById('viewport-title');
    this.viewCount = document.getElementById('viewport-count');

    // Inspector
    this.inspectorImg = document.getElementById('inspector-img');
    this.inspectorFilename = document.getElementById('inspector-filename');
    this.inspectorScore = document.getElementById('inspector-score');
    this.inspectorCritique = document.getElementById('inspector-critique');
    this.inspectorPoseType = document.getElementById('inspector-pose-type');
    this.inspectorPoseStatus = document.getElementById('inspector-pose-status');
    this.inspectorTags = document.getElementById('inspector-tags');
    this.inspectorExifCamera = document.getElementById('exif-camera');
    this.inspectorExifLens = document.getElementById('exif-lens');
    this.inspectorExifSettings = document.getElementById('exif-settings');
    this.btnPick = document.getElementById('btn-action-pick');
    this.btnReject = document.getElementById('btn-action-reject');

    // Instance Selector Dropdown Click
    this.instanceBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      this.instanceDrawer.classList.toggle('open');
      this.telemetryDrawer.classList.remove('open');
    });

    // Telemetry Drawer Dropdown Click
    this.statusTicker.addEventListener('click', (e) => {
      e.stopPropagation();
      this.telemetryDrawer.classList.toggle('open');
      this.instanceDrawer.classList.remove('open');
      this.renderTelemetry();
    });

    document.addEventListener('click', () => {
      this.instanceDrawer.classList.remove('open');
      this.telemetryDrawer.classList.remove('open');
    });

    // View Toggles
    this.gridBtn.addEventListener('click', () => this.setView('grid'));
    this.graphBtn.addEventListener('click', () => this.setView('graph'));

    // Omnibox
    this.omniboxInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        this.handleSearch(this.omniboxInput.value.trim());
      }
    });

    // Triage Action Buttons
    this.btnPick.addEventListener('click', () => this.togglePick(this.selectedImage));
    this.btnReject.addEventListener('click', () => this.toggleReject(this.selectedImage));

    // Clear Agent Banner Button
    const clearBannerBtn = document.getElementById('btn-clear-banner');
    if (clearBannerBtn) {
      clearBannerBtn.addEventListener('click', () => this.clearAgentBanner());
    }

    // Graph Viewer Init
    this.graphViewer = new PanGraphViewer(this.canvasElement, (node) => {
      if (node.type === 'image' && node.raw) {
        this.selectImage(node.raw);
      }
    });

    window.addEventListener('resize', () => {
      if (this.currentView === 'graph') {
        this.graphViewer.resize();
      }
    });
  }

  initSSE() {
    // Connect to SSE stream for live agent casts via `pansee` CLI
    try {
      const eventSource = new EventSource('/api/events');
      eventSource.onmessage = (e) => {
        const event = JSON.parse(e.data);
        this.handleAgentEvent(event);
      };
      eventSource.onerror = () => {
        // Standalone static mode fallback without live server
      };
    } catch (err) {
      console.log('SSE not connected (running in static demo mode)');
    }
  }

  handleAgentEvent(event) {
    console.log('⚡ Agent Cast Event received:', event);

    if (event.type === 'cast_sparql') {
      this.applySparql(event.query, event.agent || 'familiar');
    } else if (event.type === 'cast_set') {
      this.filterByPhotoset(event.setId, event.agent || 'familiar');
    } else if (event.type === 'cast_image') {
      const img = this.images.find(i => i.id === event.imageId || i.filename === event.imageId);
      if (img) this.selectImage(img);
    } else if (event.type === 'cast_view') {
      this.setView(event.view);
    } else if (event.type === 'cast_instance') {
      this.switchInstance(event.instanceId);
    } else if (event.type === 'pand_status') {
      this.updatePandStatus(event);
    }
  }

  updatePandStatus(data) {
    if (data.task) this.pandTelemetry.currentTask = data.task;
    if (data.state) this.pandTelemetry.state = data.state;
    if (data.progress !== undefined) this.pandTelemetry.progress = data.progress;
    if (data.queueDepth !== undefined) this.pandTelemetry.queueStats.pending = data.queueDepth;
    this.renderTelemetry();
  }

  renderTelemetry() {
    const t = this.pandTelemetry;
    this.statusText.textContent = `pand: ${t.currentTask}`;
    this.queueBadge.textContent = `${t.queueStats.pending} queued`;
    this.statusDot.classList.toggle('active', t.state !== 'idle');

    this.telemetryDrawer.innerHTML = `
      <div class="telemetry-header">
        <span class="telemetry-title">pand Daemon Telemetry</span>
        <span style="font-size: 10px; font-family: var(--font-mono); color: var(--accent-cyan);">${t.state.toUpperCase()}</span>
      </div>
      <div class="telemetry-task-box">
        <div style="color: var(--text-primary); font-weight: 600; margin-bottom: 4px;">Current Task</div>
        <div style="color: var(--text-secondary);">${t.currentTask}</div>
        <div class="telemetry-progress-bar">
          <div class="telemetry-progress-fill" style="width: ${t.progress}%;"></div>
        </div>
        <div style="display: flex; justify-content: space-between; font-size: 10px; color: var(--text-muted); margin-top: 4px; font-family: var(--font-mono);">
          <span>Shoot: ${t.activeShoot}</span>
          <span>${t.progress.toFixed(1)}%</span>
        </div>
      </div>

      <div style="font-size: 11px; font-weight: 700; text-transform: uppercase; color: var(--text-muted); margin-bottom: 6px;">
        SQLite Indexer Queue (~/.pan/queue.db)
      </div>
      <div class="queue-grid">
        <div class="queue-metric">
          <div class="queue-metric-label">Discovered Files</div>
          <div class="queue-metric-val">${t.queueStats.discovered.toLocaleString()}</div>
        </div>
        <div class="queue-metric">
          <div class="queue-metric-label">4K Previews Sliced</div>
          <div class="queue-metric-val">${t.queueStats.extracted.toLocaleString()}</div>
        </div>
        <div class="queue-metric">
          <div class="queue-metric-label">AI Perception Pass</div>
          <div class="queue-metric-val">${t.queueStats.enriched.toLocaleString()}</div>
        </div>
        <div class="queue-metric">
          <div class="queue-metric-label">Pending Queue Depth</div>
          <div class="queue-metric-val" style="color: var(--accent-amber);">${t.queueStats.pending.toLocaleString()}</div>
        </div>
      </div>

      <div style="font-size: 10px; color: var(--text-muted); border-top: 1px solid var(--border-subtle); padding-top: 8px; display: flex; justify-content: space-between;">
        <span>Runner: ${t.runnerStatus}</span>
        <span style="color: var(--accent-emerald);">● Online</span>
      </div>
    `;
  }

  initShortcuts() {
    window.addEventListener('keydown', (e) => {
      if (e.target.tagName === 'INPUT') return;

      if (e.key === 'p' || e.key === 'P') {
        this.togglePick(this.selectedImage);
      } else if (e.key === 'x' || e.key === 'X') {
        this.toggleReject(this.selectedImage);
      } else if (e.key >= '1' && e.key <= '5') {
        this.setRating(this.selectedImage, parseInt(e.key));
      } else if (e.key === 'v' || e.key === 'V') {
        this.setView(this.currentView === 'grid' ? 'graph' : 'grid');
      } else if (e.key === 'k' && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        this.omniboxInput.focus();
      }
    });
  }

  switchInstance(instanceId) {
    const inst = this.instances.find(i => i.id === instanceId);
    if (!inst) return;
    this.currentInstance = inst;
    this.currentInstanceLabel.textContent = inst.name;
    this.currentInstancePort.textContent = `:${inst.port}`;
    this.instanceDrawer.classList.remove('open');

    // Update filter back to all
    this.filter = { type: 'all', label: 'All Photos' };
    this.render();
  }

  setView(view) {
    this.currentView = view;
    if (view === 'grid') {
      this.gridBtn.classList.add('active');
      this.graphBtn.classList.remove('active');
      this.galleryGrid.style.display = 'grid';
      this.graphContainer.classList.remove('active');
    } else {
      this.graphBtn.classList.add('active');
      this.gridBtn.classList.remove('active');
      this.galleryGrid.style.display = 'none';
      this.graphContainer.classList.add('active');
      this.graphViewer.resize();
      this.graphViewer.setData(this.getFilteredImages(), MOCK_GRAPH_TRIPLES);
    }
  }

  handleSearch(query) {
    if (!query) {
      this.filter = { type: 'all', label: 'All Photos' };
    } else if (query.toLowerCase().startsWith('select ') || query.includes('pan:')) {
      this.applySparql(query, 'user');
      return;
    } else {
      this.filter = { type: 'search', query, label: `Search: "${query}"` };
    }
    this.render();
  }

  applySparql(query, actor = 'familiar') {
    this.filter = { type: 'sparql', query, label: `SPARQL: ${query}` };
    this.agentCast = {
      actor,
      message: `Casting SPARQL query: ${query}`,
      timestamp: new Date().toLocaleTimeString()
    };
    this.render();
  }

  filterByPhotoset(setId, actor = null) {
    const set = MOCK_PHOTOSETS.find(s => s.id === setId);
    const setName = set ? set.name : setId;
    this.filter = { type: 'photoset', setId, label: `Photoset: ${setName}` };

    if (actor) {
      this.agentCast = {
        actor,
        message: `Pushed Photoset: ${setName}`,
        timestamp: new Date().toLocaleTimeString()
      };
    }
    this.render();
  }

  filterByShoot(shootId) {
    this.filter = { type: 'shoot', shootId, label: `Shoot: ${shootId}` };
    this.render();
  }

  clearAgentBanner() {
    this.agentCast = null;
    this.renderAgentBanner();
  }

  togglePick(img) {
    if (!img) return;
    img.isPicked = !img.isPicked;
    if (img.isPicked) img.isRejected = false;
    this.render();
  }

  toggleReject(img) {
    if (!img) return;
    img.isRejected = !img.isRejected;
    if (img.isRejected) img.isPicked = false;
    this.render();
  }

  setRating(img, rating) {
    if (!img) return;
    img.rating = (img.rating === rating) ? 0 : rating;
    this.render();
  }

  selectImage(img) {
    this.selectedImage = img;
    this.renderInspector();
    this.renderGridHighlights();
  }

  getFilteredImages() {
    let list = this.images;

    if (this.filter.type === 'shoot') {
      list = list.filter(i => i.folderPath === this.filter.shootId);
    } else if (this.filter.type === 'photoset') {
      list = list.filter(i => i.photosets.includes(this.filter.setId));
    } else if (this.filter.type === 'search') {
      const q = this.filter.query.toLowerCase();
      list = list.filter(i => 
        i.filename.toLowerCase().includes(q) ||
        i.proxy.shortDescription.toLowerCase().includes(q) ||
        i.proxy.sceneObjects.some(o => o.toLowerCase().includes(q)) ||
        i.subject.toLowerCase().includes(q)
      );
    } else if (this.filter.type === 'sparql') {
      const q = this.filter.query.toLowerCase();
      if (q.includes('pan:rating 5')) {
        list = list.filter(i => i.rating === 5);
      } else if (q.includes('pan:ispicked true')) {
        list = list.filter(i => i.isPicked);
      } else if (q.includes('aestheticappeal')) {
        list = list.filter(i => i.proxy.aestheticConsensus.mean >= 90);
      }
    }

    return list;
  }

  render() {
    this.renderInstanceDrawer();
    this.renderNavigation();
    this.renderAgentBanner();
    this.renderGrid();
    this.renderInspector();

    if (this.currentView === 'graph') {
      this.graphViewer.setData(this.getFilteredImages(), MOCK_GRAPH_TRIPLES);
    }
  }

  renderInstanceDrawer() {
    this.instanceDrawer.innerHTML = `
      <div style="font-size: 11px; font-weight: 700; text-transform: uppercase; color: var(--text-muted); margin-bottom: 8px;">
        Local Pan Daemons (${this.instances.length})
      </div>
      ${this.instances.map(inst => `
        <div class="instance-card ${inst.id === this.currentInstance.id ? 'active' : ''}" data-id="${inst.id}">
          <div class="instance-card-header">
            <span class="instance-card-title">${inst.name}</span>
            <span class="instance-card-mode">${inst.mode.split(' ')[0]}</span>
          </div>
          <div class="instance-card-desc">${inst.description}</div>
          <div class="instance-card-footer">
            <span>Port ${inst.port}</span>
            <span>${inst.itemCount}</span>
          </div>
        </div>
      `).join('')}
    `;

    this.instanceDrawer.querySelectorAll('.instance-card').forEach(card => {
      card.addEventListener('click', () => {
        this.switchInstance(card.dataset.id);
      });
    });
  }

  renderNavigation() {
    // 1. Shoots ("Dumb Folders")
    this.listShoots.innerHTML = MOCK_SHOOTS.map(s => `
      <li class="nav-item ${this.filter.type === 'shoot' && this.filter.shootId === s.id ? 'active' : ''}" data-shoot="${s.id}">
        <div class="nav-item-left">
          <span>📁</span>
          <span>${s.name.replace('2026/', '')}</span>
        </div>
        <span class="nav-count-badge">${s.count}</span>
      </li>
    `).join('');

    this.listShoots.querySelectorAll('.nav-item').forEach(el => {
      el.addEventListener('click', () => this.filterByShoot(el.dataset.shoot));
    });

    // 2. Photosets
    this.listPhotosets.innerHTML = MOCK_PHOTOSETS.map(p => `
      <li class="nav-item ${this.filter.type === 'photoset' && this.filter.setId === p.id ? 'active' : ''}" data-set="${p.id}">
        <div class="nav-item-left">
          <span>⬡</span>
          <span>${p.name}</span>
        </div>
        <span class="xml-badge">XML</span>
      </li>
    `).join('');

    this.listPhotosets.querySelectorAll('.nav-item').forEach(el => {
      el.addEventListener('click', () => this.filterByPhotoset(el.dataset.set));
    });

    // 3. Smart Collections
    this.listSmart.innerHTML = MOCK_SMART_COLLECTIONS.map(c => `
      <li class="nav-item" data-sparql="${c.sparql}" data-name="${c.name}">
        <div class="nav-item-left">
          <span>${c.icon}</span>
          <span>${c.name}</span>
        </div>
        <span class="nav-count-badge">SPARQL</span>
      </li>
    `).join('');

    this.listSmart.querySelectorAll('.nav-item').forEach(el => {
      el.addEventListener('click', () => {
        this.applySparql(el.dataset.sparql, 'smart-collection');
      });
    });
  }

  renderAgentBanner() {
    if (!this.agentCast) {
      this.agentBanner.style.display = 'none';
      return;
    }

    this.agentBanner.style.display = 'flex';
    this.agentBanner.innerHTML = `
      <div class="agent-banner-info">
        <span class="agent-tag">⚡ AGENT: @${this.agentCast.actor}</span>
        <span>${this.agentCast.message}</span>
        <span class="agent-sparql-snippet">${this.agentCast.timestamp}</span>
      </div>
      <div class="agent-banner-actions">
        <button id="btn-clear-banner" class="btn-banner-clear">Dismiss Cast</button>
      </div>
    `;

    document.getElementById('btn-clear-banner').addEventListener('click', () => this.clearAgentBanner());
  }

  renderGrid() {
    const items = this.getFilteredImages();
    this.viewTitle.textContent = this.filter.label;
    this.viewCount.textContent = `${items.length} of ${this.images.length} photos`;

    this.galleryGrid.innerHTML = items.map(img => `
      <div class="photo-card ${img.id === this.selectedImage?.id ? 'selected' : ''}" data-id="${img.id}">
        <div class="photo-card-thumb-wrap">
          <img class="photo-card-thumb" src="${img.previewUrl}" alt="${img.filename}" loading="lazy" />
          <div class="card-badges-top">
            ${img.burstCount > 1 ? `<span class="burst-badge">⧉ ${img.burstCount}</span>` : '<span></span>'}
            ${img.isPicked ? `<span class="pick-badge">★ PICK</span>` : ''}
          </div>
        </div>
        <div class="photo-card-meta">
          <div class="card-filename-row">
            <span class="card-filename">${img.filename}</span>
            <span class="card-rating">${'★'.repeat(img.rating)}</span>
          </div>
          <div class="card-details-row">
            <span>${img.exif.lens.split(' ')[1] || '85mm'}</span>
            <span>${img.exif.shutter} · ${img.exif.aperture}</span>
          </div>
        </div>
      </div>
    `).join('');

    this.galleryGrid.querySelectorAll('.photo-card').forEach(card => {
      card.addEventListener('click', () => {
        const found = this.images.find(i => i.id === card.dataset.id);
        if (found) this.selectImage(found);
      });
    });
  }

  renderGridHighlights() {
    this.galleryGrid.querySelectorAll('.photo-card').forEach(card => {
      card.classList.toggle('selected', card.dataset.id === this.selectedImage?.id);
    });
  }

  renderInspector() {
    const img = this.selectedImage;
    if (!img) return;

    this.inspectorImg.src = img.previewUrl;
    this.inspectorFilename.textContent = img.filename;

    // Consensus Score
    const consensus = img.proxy.aestheticConsensus;
    this.inspectorScore.textContent = `${consensus.mean.toFixed(1)} / 100`;
    document.getElementById('gauge-bar').style.width = `${consensus.mean}%`;
    this.inspectorCritique.textContent = `“${consensus.critique}”`;

    // Pose Metrics
    this.inspectorPoseType.textContent = img.proxy.poseMetrics.postureType;
    this.inspectorPoseStatus.textContent = img.proxy.poseMetrics.anatomicalIntegrity;

    // Tags
    this.inspectorTags.innerHTML = img.proxy.sceneObjects.map(tag => `
      <span class="tag-pill">${tag}</span>
    `).join('');

    // EXIF
    this.inspectorExifCamera.textContent = img.exif.camera;
    this.inspectorExifLens.textContent = img.exif.lens;
    this.inspectorExifSettings.textContent = `${img.exif.focalLength} · ${img.exif.aperture} · ${img.exif.shutter} · ISO ${img.exif.iso}`;

    // Action button states
    this.btnPick.classList.toggle('picked', img.isPicked);
    this.btnReject.classList.toggle('rejected', img.isRejected);
  }
}

window.addEventListener('DOMContentLoaded', () => {
  window.app = new PanApp();
});
