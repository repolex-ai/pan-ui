// Force-directed RDF Knowledge Graph visualizer for Pan UI
export class PanGraphViewer {
  constructor(canvasElement, onNodeSelect) {
    this.canvas = canvasElement;
    this.ctx = canvasElement.getContext('2d');
    this.onNodeSelect = onNodeSelect;

    this.nodes = [];
    this.edges = [];
    this.nodeMap = new Map();

    this.width = this.canvas.width;
    this.height = this.canvas.height;
    this.transform = { x: this.width / 2, y: this.height / 2, k: 1 };

    this.dragNode = null;
    this.hoverNode = null;
    this.isPanning = false;
    this.panStart = { x: 0, y: 0 };
    this.animId = null;

    this.setupEvents();
  }

  resize() {
    const rect = this.canvas.getBoundingClientRect();
    this.canvas.width = rect.width * window.devicePixelRatio;
    this.canvas.height = rect.height * window.devicePixelRatio;
    this.width = rect.width;
    this.height = rect.height;
    this.render();
  }

  setData(images, triples) {
    this.nodes = [];
    this.edges = [];
    this.nodeMap = new Map();

    // 1. Add Image nodes
    images.forEach(img => {
      const node = {
        id: img.id,
        type: 'image',
        label: img.filename,
        rating: img.rating,
        isPicked: img.isPicked,
        raw: img,
        x: (Math.random() - 0.5) * 300,
        y: (Math.random() - 0.5) * 300,
        vx: 0,
        vy: 0,
        radius: 18,
        color: img.isPicked ? '#f59e0b' : '#3b82f6'
      };
      this.nodes.push(node);
      this.nodeMap.set(node.id, node);
    });

    // 2. Add Entities from Triples
    triples.forEach(t => {
      if (!this.nodeMap.has(t.object)) {
        let type = 'entity';
        let color = '#a855f7';
        let radius = 14;

        if (t.object.startsWith('set:')) {
          type = 'photoset';
          color = '#ec4899';
          radius = 18;
        } else if (t.object.startsWith('shoot:')) {
          type = 'shoot';
          color = '#06b6d4';
          radius = 16;
        } else if (t.object.startsWith('subject:')) {
          type = 'subject';
          color = '#10b981';
          radius = 15;
        } else if (t.object.startsWith('pose:')) {
          type = 'pose';
          color = '#eab308';
          radius = 12;
        }

        const node = {
          id: t.object,
          type,
          label: t.object.split(':')[1].replace(/_/g, ' '),
          x: (Math.random() - 0.5) * 400,
          y: (Math.random() - 0.5) * 400,
          vx: 0,
          vy: 0,
          radius,
          color
        };
        this.nodes.push(node);
        this.nodeMap.set(node.id, node);
      }

      if (this.nodeMap.has(t.subject) && this.nodeMap.has(t.object)) {
        this.edges.push({
          source: this.nodeMap.get(t.subject),
          target: this.nodeMap.get(t.object),
          predicate: t.predicate
        });
      }
    });

    this.startSimulation();
  }

  startSimulation() {
    let steps = 0;
    const tick = () => {
      this.updatePhysics();
      this.render();
      steps++;
      if (steps < 300 || this.dragNode) {
        this.animId = requestAnimationFrame(tick);
      }
    };
    cancelAnimationFrame(this.animId);
    this.animId = requestAnimationFrame(tick);
  }

  updatePhysics() {
    const kRepel = 1200;
    const kSpring = 0.05;
    const damp = 0.85;

    // Node-Node Repulsion
    for (let i = 0; i < this.nodes.length; i++) {
      for (let j = i + 1; j < this.nodes.length; j++) {
        const a = this.nodes[i];
        const b = this.nodes[j];
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let dist = Math.sqrt(dx * dx + dy * dy) || 1;
        if (dist < 350) {
          let force = (kRepel / (dist * dist));
          let fx = (dx / dist) * force;
          let fy = (dy / dist) * force;
          a.vx -= fx;
          a.vy -= fy;
          b.vx += fx;
          b.vy += fy;
        }
      }
    }

    // Edge Attraction
    this.edges.forEach(edge => {
      let dx = edge.target.x - edge.source.x;
      let dy = edge.target.y - edge.source.y;
      let dist = Math.sqrt(dx * dx + dy * dy) || 1;
      let force = (dist - 110) * kSpring;
      let fx = (dx / dist) * force;
      let fy = (dy / dist) * force;
      edge.source.vx += fx;
      edge.source.vy += fy;
      edge.target.vx -= fx;
      edge.target.vy -= fy;
    });

    // Apply velocities with damping
    this.nodes.forEach(node => {
      if (node !== this.dragNode) {
        node.vx *= damp;
        node.vy *= damp;
        // Center gravity
        node.vx -= node.x * 0.002;
        node.vy -= node.y * 0.002;

        node.x += node.vx;
        node.y += node.vy;
      }
    });
  }

  render() {
    const dpr = window.devicePixelRatio || 1;
    this.ctx.save();
    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);

    // Camera transform
    this.ctx.scale(dpr, dpr);
    this.ctx.translate(this.transform.x, this.transform.y);
    this.ctx.scale(this.transform.k, this.transform.k);

    // Draw grid lines background
    this.drawBackgroundGrid();

    // Draw Edges
    this.ctx.lineWidth = 1.2;
    this.edges.forEach(edge => {
      const isHovered = (edge.source === this.hoverNode || edge.target === this.hoverNode);
      this.ctx.strokeStyle = isHovered ? '#38bdf8' : '#27272a';
      this.ctx.lineWidth = isHovered ? 2.0 : 1.0;

      this.ctx.beginPath();
      this.ctx.moveTo(edge.source.x, edge.source.y);
      this.ctx.lineTo(edge.target.x, edge.target.y);
      this.ctx.stroke();

      // Edge predicate label
      if (isHovered || this.transform.k > 1.3) {
        const mx = (edge.source.x + edge.target.x) / 2;
        const my = (edge.source.y + edge.target.y) / 2;
        this.ctx.fillStyle = '#71717a';
        this.ctx.font = '9px monospace';
        this.ctx.textAlign = 'center';
        this.ctx.fillText(edge.predicate, mx, my - 4);
      }
    });

    // Draw Nodes
    this.nodes.forEach(node => {
      this.ctx.save();
      this.ctx.translate(node.x, node.y);

      const isHover = (node === this.hoverNode);
      const radius = node.radius + (isHover ? 3 : 0);

      // Node halo
      if (isHover || node.isPicked) {
        this.ctx.beginPath();
        this.ctx.arc(0, 0, radius + 4, 0, Math.PI * 2);
        this.ctx.fillStyle = node.color + '44';
        this.ctx.fill();
      }

      // Main shape
      this.ctx.beginPath();
      if (node.type === 'photoset') {
        // Hexagon
        for (let a = 0; a < 6; a++) {
          const angle = (Math.PI / 3) * a;
          const px = Math.cos(angle) * radius;
          const py = Math.sin(angle) * radius;
          if (a === 0) this.ctx.moveTo(px, py);
          else this.ctx.lineTo(px, py);
        }
        this.ctx.closePath();
      } else if (node.type === 'shoot') {
        // Rounded square
        this.ctx.roundRect(-radius, -radius, radius * 2, radius * 2, 4);
      } else if (node.type === 'subject') {
        // Diamond
        this.ctx.moveTo(0, -radius);
        this.ctx.lineTo(radius, 0);
        this.ctx.lineTo(0, radius);
        this.ctx.lineTo(-radius, 0);
        this.ctx.closePath();
      } else {
        // Circle (Image / Attribute)
        this.ctx.arc(0, 0, radius, 0, Math.PI * 2);
      }

      this.ctx.fillStyle = node.color;
      this.ctx.fill();
      this.ctx.lineWidth = 1.5;
      this.ctx.strokeStyle = '#09090b';
      this.ctx.stroke();

      // Node label
      this.ctx.fillStyle = isHover ? '#ffffff' : '#a1a1aa';
      this.ctx.font = isHover ? 'bold 11px -apple-system, sans-serif' : '10px -apple-system, sans-serif';
      this.ctx.textAlign = 'center';
      this.ctx.fillText(node.label, 0, radius + 14);

      // Rating badge on images
      if (node.type === 'image' && node.rating) {
        this.ctx.fillStyle = '#f59e0b';
        this.ctx.font = '8px sans-serif';
        this.ctx.fillText('★'.repeat(node.rating), 0, -radius - 5);
      }

      this.ctx.restore();
    });

    this.ctx.restore();
  }

  drawBackgroundGrid() {
    const size = 60;
    const startX = -1000;
    const startY = -1000;
    const endX = 1000;
    const endY = 1000;

    this.ctx.strokeStyle = '#18181b';
    this.ctx.lineWidth = 0.5;
    this.ctx.beginPath();
    for (let x = startX; x <= endX; x += size) {
      this.ctx.moveTo(x, startY);
      this.ctx.lineTo(x, endY);
    }
    for (let y = startY; y <= endY; y += size) {
      this.ctx.moveTo(startX, y);
      this.ctx.lineTo(endX, y);
    }
    this.ctx.stroke();
  }

  setupEvents() {
    const getPos = (e) => {
      const rect = this.canvas.getBoundingClientRect();
      const clientX = e.clientX - rect.left;
      const clientY = e.clientY - rect.top;
      return {
        x: (clientX - this.transform.x) / this.transform.k,
        y: (clientY - this.transform.y) / this.transform.k,
        rawX: clientX,
        rawY: clientY
      };
    };

    const findNodeAt = (pos) => {
      return this.nodes.find(n => {
        const dx = n.x - pos.x;
        const dy = n.y - pos.y;
        return Math.sqrt(dx * dx + dy * dy) <= (n.radius + 6);
      });
    };

    this.canvas.addEventListener('mousedown', (e) => {
      const pos = getPos(e);
      const hit = findNodeAt(pos);
      if (hit) {
        this.dragNode = hit;
        this.startSimulation();
        if (this.onNodeSelect) this.onNodeSelect(hit);
      } else {
        this.isPanning = true;
        this.panStart = { x: pos.rawX - this.transform.x, y: pos.rawY - this.transform.y };
      }
    });

    window.addEventListener('mousemove', (e) => {
      if (!this.canvas.parentElement) return;
      const pos = getPos(e);

      if (this.dragNode) {
        this.dragNode.x = pos.x;
        this.dragNode.y = pos.y;
        this.dragNode.vx = 0;
        this.dragNode.vy = 0;
        this.render();
      } else if (this.isPanning) {
        this.transform.x = pos.rawX - this.panStart.x;
        this.transform.y = pos.rawY - this.panStart.y;
        this.render();
      } else {
        const hit = findNodeAt(pos);
        if (hit !== this.hoverNode) {
          this.hoverNode = hit;
          this.canvas.style.cursor = hit ? 'pointer' : 'default';
          this.render();
        }
      }
    });

    window.addEventListener('mouseup', () => {
      this.dragNode = null;
      this.isPanning = false;
    });

    this.canvas.addEventListener('wheel', (e) => {
      e.preventDefault();
      const zoomFactor = e.deltaY < 0 ? 1.1 : 0.9;
      const newK = Math.max(0.3, Math.min(4.0, this.transform.k * zoomFactor));

      const rect = this.canvas.getBoundingClientRect();
      const cx = e.clientX - rect.left;
      const cy = e.clientY - rect.top;

      this.transform.x = cx - (cx - this.transform.x) * (newK / this.transform.k);
      this.transform.y = cy - (cy - this.transform.y) * (newK / this.transform.k);
      this.transform.k = newK;

      this.render();
    });
  }
}
