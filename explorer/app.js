// Dina Explorer - Block Explorer Application

(function () {
  'use strict';

  // ---------------------------------------------------------------------------
  // Configuration
  // ---------------------------------------------------------------------------

  const DEFAULT_API = 'https://dina-proxy-ca-jy6qm6s57a-nn.a.run.app';
  const REFRESH_INTERVAL_MS = 5000;
  const PAGE_SIZE = 20;
  const MICRO_USDC = 1_000_000; // 1 USDC = 1,000,000 micro-units

  // ---------------------------------------------------------------------------
  // API Client
  // ---------------------------------------------------------------------------

  class DinaAPI {
    constructor(baseUrl) {
      this.baseUrl = (baseUrl || DEFAULT_API).replace(/\/+$/, '');
    }

    async _fetch(path, retries = 2) {
      const url = `${this.baseUrl}${path}`;
      for (let attempt = 0; attempt <= retries; attempt++) {
        try {
          const res = await fetch(url);
          if (!res.ok) throw new Error(`HTTP ${res.status}: ${res.statusText}`);
          return await res.json();
        } catch (err) {
          if (attempt === retries) throw err;
          await new Promise(r => setTimeout(r, 500 * (attempt + 1)));
        }
      }
    }

    // Chain info — validator serves /health instead of /status
    getStatus()       { return this._fetch('/health'); }
    getNetInfo()      { return this._fetch('/health'); }

    // Blocks — validator serves /v1/block/latest, /v1/block/{height}
    getLatestBlock()  { return this._fetch('/v1/block/latest'); }
    getBlock(height)  { return this._fetch(`/v1/block/${height}`); }
    getBlocks(page = 1, limit = PAGE_SIZE) {
      // Validator doesn't have a paginated blocks list — fetch latest and build a list
      return this._fetch('/v1/block/latest').then(block => {
        const b = block.block || block;
        return { blocks: [b], total: b.height || 1 };
      });
    }

    // Transactions
    getTx(hash) { return this._fetch(`/v1/transaction/${hash}`); }
    getTxs(page = 1, limit = PAGE_SIZE) {
      return this._fetch('/v1/transactions')
        .then(data => {
          const txs = (data.transactions || []).map(tx => ({
            hash: tx.tx_hash,
            from: tx.from,
            to: tx.to,
            amount: tx.amount,
            fee: tx.fee,
            block: tx.block_height,
            status: tx.status,
            type: tx.type || tx.tx_type,
          }));
          return { txs, total: data.total || txs.length };
        })
        .catch(() => ({ txs: [], total: 0 }));
    }
    getTxsByBlock(height) {
      return Promise.resolve({ txs: [] });
    }

    // Accounts
    getAccount(addr) {
      return this._fetch(`/v1/balance/${addr}`).then(data => ({
        account: { address: addr, balance: data.balance || 0, nonce: data.nonce || 0, type: 'standard' }
      }));
    }
    getAccountTxs(addr, page = 1, limit = PAGE_SIZE) {
      return this._fetch(`/v1/transactions/${addr}`)
        .then(data => {
          const txs = (data.transactions || []).map(tx => ({
            hash: tx.tx_hash,
            from: tx.from,
            to: tx.to,
            amount: tx.amount,
            fee: tx.fee,
            block: tx.block_height,
            status: tx.status,
            type: tx.type,
          }));
          return { txs, total: txs.length };
        })
        .catch(() => ({ txs: [], total: 0 }));
    }

    // Devices — not available on current validator
    getDevices(page = 1, limit = PAGE_SIZE) {
      return Promise.resolve({ devices: [], total: 0 });
    }
    getDevice(id) {
      return Promise.reject(new Error('Device lookup not available on testnet yet'));
    }

    // Search — no search endpoint, use heuristic routing in the app
    search(query) {
      return Promise.reject(new Error('Search API not available — use direct navigation'));
    }
  }

  // ---------------------------------------------------------------------------
  // Utilities
  // ---------------------------------------------------------------------------

  function truncAddr(addr, chars = 6) {
    if (!addr) return '';
    if (addr.length <= chars * 2 + 3) return addr;
    return `${addr.slice(0, chars + 2)}...${addr.slice(-chars)}`;
  }

  function formatUsdc(microAmount) {
    if (microAmount == null) return '0.00';
    const num = Number(microAmount) / MICRO_USDC;
    return num.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 6 });
  }

  function relativeTime(ts) {
    if (!ts) return '';
    const date = typeof ts === 'string' ? new Date(ts) : new Date(ts * 1000);
    const diff = (Date.now() - date.getTime()) / 1000;
    if (diff < 0) return 'just now';
    if (diff < 60) return `${Math.floor(diff)}s ago`;
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return `${Math.floor(diff / 86400)}d ago`;
  }

  function fullTime(ts) {
    if (!ts) return '';
    const date = typeof ts === 'string' ? new Date(ts) : new Date(ts * 1000);
    return date.toISOString().replace('T', ' ').replace('Z', ' UTC');
  }

  function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
  }

  function copyToClipboard(text, btnEl) {
    navigator.clipboard.writeText(text).then(() => {
      if (btnEl) {
        btnEl.classList.add('copied');
        btnEl.textContent = '\u2713';
        setTimeout(() => {
          btnEl.classList.remove('copied');
          btnEl.textContent = '\u2398';
        }, 1500);
      }
    });
  }

  function statusBadge(status) {
    const s = (status || '').toLowerCase();
    if (s === 'success' || s === 'confirmed' || s === 'active' || s === 'verified') {
      return `<span class="status status-success"><span class="status-dot"></span>${escapeHtml(status)}</span>`;
    }
    if (s === 'failed' || s === 'revoked' || s === 'rejected') {
      return `<span class="status status-error"><span class="status-dot"></span>${escapeHtml(status)}</span>`;
    }
    return `<span class="status status-pending"><span class="status-dot"></span>${escapeHtml(status || 'pending')}</span>`;
  }

  function renderLoading() {
    return '<div class="loading"><div class="spinner"></div><div class="loading-text">Loading...</div></div>';
  }

  function renderError(msg, retryFn) {
    const id = 'retry-' + Math.random().toString(36).slice(2, 8);
    setTimeout(() => {
      const btn = document.getElementById(id);
      if (btn && retryFn) btn.addEventListener('click', retryFn);
    }, 0);
    return `<div class="error-box">
      <span>${escapeHtml(msg)}</span>
      ${retryFn ? `<button class="retry-btn" id="${id}">Retry</button>` : ''}
    </div>`;
  }

  function renderEmpty(message) {
    return `<div class="empty-state">
      <div class="empty-state-icon">\u2205</div>
      <div>${escapeHtml(message)}</div>
    </div>`;
  }

  function copyableValue(val) {
    const id = 'cp-' + Math.random().toString(36).slice(2, 8);
    setTimeout(() => {
      const btn = document.getElementById(id);
      if (btn) btn.addEventListener('click', () => copyToClipboard(val, btn));
    }, 0);
    return `<span class="mono">${escapeHtml(val)}</span><button class="copy-btn" id="${id}" title="Copy">\u2398</button>`;
  }

  function pagination(currentPage, totalPages, onNavigate) {
    if (totalPages <= 1) return '';
    const id = 'pag-' + Math.random().toString(36).slice(2, 8);
    let html = `<div class="pagination" id="${id}">`;
    html += `<button class="page-btn" data-page="${currentPage - 1}" ${currentPage <= 1 ? 'disabled' : ''}>\u2190 Prev</button>`;

    const start = Math.max(1, currentPage - 2);
    const end = Math.min(totalPages, currentPage + 2);
    if (start > 1) html += `<button class="page-btn" data-page="1">1</button>`;
    if (start > 2) html += `<span class="page-info">...</span>`;
    for (let i = start; i <= end; i++) {
      html += `<button class="page-btn ${i === currentPage ? 'active' : ''}" data-page="${i}">${i}</button>`;
    }
    if (end < totalPages - 1) html += `<span class="page-info">...</span>`;
    if (end < totalPages) html += `<button class="page-btn" data-page="${totalPages}">${totalPages}</button>`;

    html += `<button class="page-btn" data-page="${currentPage + 1}" ${currentPage >= totalPages ? 'disabled' : ''}>Next \u2192</button>`;
    html += '</div>';

    setTimeout(() => {
      const el = document.getElementById(id);
      if (!el) return;
      el.querySelectorAll('.page-btn').forEach(btn => {
        btn.addEventListener('click', () => {
          const p = parseInt(btn.dataset.page);
          if (!isNaN(p) && p >= 1 && p <= totalPages) onNavigate(p);
        });
      });
    }, 0);

    return html;
  }

  // ---------------------------------------------------------------------------
  // Router
  // ---------------------------------------------------------------------------

  class Router {
    constructor() {
      this.routes = [];
      this.currentRoute = null;
      window.addEventListener('hashchange', () => this.resolve());
    }

    on(pattern, handler) {
      // Convert pattern like '/blocks/:height' to regex
      const paramNames = [];
      const regex = new RegExp(
        '^' + pattern.replace(/:(\w+)/g, (_, name) => {
          paramNames.push(name);
          return '([^/]+)';
        }) + '$'
      );
      this.routes.push({ pattern, regex, paramNames, handler });
      return this;
    }

    resolve() {
      const hash = (window.location.hash || '#/').slice(1) || '/';
      for (const route of this.routes) {
        const match = hash.match(route.regex);
        if (match) {
          const params = {};
          route.paramNames.forEach((name, i) => {
            params[name] = decodeURIComponent(match[i + 1]);
          });
          this.currentRoute = route.pattern;
          route.handler(params);
          return;
        }
      }
      // Fallback to blocks
      window.location.hash = '#/blocks';
    }

    navigate(path) {
      window.location.hash = '#' + path;
    }
  }

  // ---------------------------------------------------------------------------
  // Application
  // ---------------------------------------------------------------------------

  class DinaExplorer {
    constructor() {
      this.api = new DinaAPI(this._getApiUrl());
      this.router = new Router();
      this.refreshTimer = null;
      this.refreshPaused = false;
      this.currentRefresh = null; // the function to call on auto-refresh

      this._setupRoutes();
      this._setupNav();
      this._startAutoRefresh();
      this._refreshChainStats();
    }

    _getApiUrl() {
      const params = new URLSearchParams(window.location.search);
      return params.get('api') || DEFAULT_API;
    }

    _setupRoutes() {
      const r = this.router;
      r.on('/blocks', () => this._pageBlocks(1));
      r.on('/blocks/page/:page', p => this._pageBlocks(parseInt(p.page)));
      r.on('/block/:height', p => this._pageBlockDetail(p.height));
      r.on('/txs', () => this._pageTxs(1));
      r.on('/txs/page/:page', p => this._pageTxs(parseInt(p.page)));
      r.on('/tx/:hash', p => this._pageTxDetail(p.hash));
      r.on('/accounts', () => this._pageAccountSearch());
      r.on('/account/:addr', p => this._pageAccountDetail(p.addr, 1));
      r.on('/account/:addr/page/:page', p => this._pageAccountDetail(p.addr, parseInt(p.page)));
      r.on('/devices', () => this._pageDevices(1));
      r.on('/devices/page/:page', p => this._pageDevices(parseInt(p.page)));
      r.on('/device/:id', p => this._pageDeviceDetail(p.id));
      r.on('/search', () => this._pageSearch());
      r.on('/', () => { window.location.hash = '#/blocks'; });
      r.resolve();
    }

    _setupNav() {
      document.querySelectorAll('.nav-tab').forEach(tab => {
        tab.addEventListener('click', () => {
          this.router.navigate(tab.dataset.route);
        });
      });

      // Pause auto-refresh on user interaction with page content
      const main = document.getElementById('content');
      let pauseTimeout;
      main.addEventListener('click', () => {
        this.refreshPaused = true;
        this._updateRefreshIndicator();
        clearTimeout(pauseTimeout);
        pauseTimeout = setTimeout(() => {
          this.refreshPaused = false;
          this._updateRefreshIndicator();
        }, 15000);
      });
    }

    _updateActiveNav() {
      const hash = window.location.hash || '#/blocks';
      document.querySelectorAll('.nav-tab').forEach(tab => {
        const route = tab.dataset.route;
        tab.classList.toggle('active', hash.startsWith('#' + route.split('/')[1] ? '#/' + route.split('/')[1] : '#' + route));
      });
      // Simpler: match on first path segment
      const segment = hash.split('/')[1] || 'blocks';
      document.querySelectorAll('.nav-tab').forEach(tab => {
        const tabSeg = (tab.dataset.route || '').split('/')[1] || '';
        tab.classList.toggle('active', tabSeg === segment ||
          (segment === 'block' && tabSeg === 'blocks') ||
          (segment === 'tx' && tabSeg === 'txs') ||
          (segment === 'account' && tabSeg === 'accounts') ||
          (segment === 'device' && tabSeg === 'devices'));
      });
    }

    _startAutoRefresh() {
      this.refreshTimer = setInterval(() => {
        if (!this.refreshPaused && typeof this.currentRefresh === 'function') {
          this.currentRefresh();
        }
        this._refreshChainStats();
      }, REFRESH_INTERVAL_MS);
    }

    _updateRefreshIndicator() {
      const dot = document.getElementById('refresh-dot');
      if (dot) {
        dot.classList.toggle('paused', this.refreshPaused);
      }
      const label = document.getElementById('refresh-label');
      if (label) {
        label.textContent = this.refreshPaused ? 'Paused' : 'Live';
      }
    }

    async _refreshChainStats() {
      try {
        const status = await this.api.getStatus();
        const height = status.latest_block_height || status.sync_info?.latest_block_height || status.height || '-';
        const tps = status.tps != null ? status.tps : '-';
        const peers = status.peers != null ? status.peers : (status.n_peers != null ? status.n_peers : '-');

        document.getElementById('stat-height').textContent = Number(height).toLocaleString();
        document.getElementById('stat-tps').textContent = tps;
        document.getElementById('stat-peers').textContent = peers;
      } catch {
        // Silently fail for stats refresh
      }
    }

    _render(html) {
      document.getElementById('content').innerHTML = html;
      this._updateActiveNav();
    }

    // --- Blocks Page ---
    async _pageBlocks(page) {
      this._render(renderLoading());
      this.currentRefresh = () => this._pageBlocks(page);

      try {
        const data = await this.api.getBlocks(page, PAGE_SIZE);
        const blocks = data.blocks || data.data || data || [];
        const total = data.total_count || data.total || blocks.length;
        const totalPages = Math.max(1, Math.ceil(total / PAGE_SIZE));

        let html = '<div class="card">';
        html += '<div class="card-header"><span class="card-title">Recent Blocks</span>';
        html += `<span class="card-badge">${Number(total).toLocaleString()} total</span></div>`;
        html += '<div class="table-wrap"><table><thead><tr>';
        html += '<th>Height</th><th>Hash</th><th>Proposer</th><th>Txs</th><th>Time</th>';
        html += '</tr></thead><tbody>';

        if (blocks.length === 0) {
          html += `<tr><td colspan="5">${renderEmpty('No blocks found')}</td></tr>`;
        }

        for (const b of blocks) {
          const height = b.height || b.header?.height || '';
          const hash = b.hash || b.block_hash || '';
          const proposer = b.proposer || b.header?.proposer_address || '';
          const txCount = b.tx_count != null ? b.tx_count : (b.num_txs != null ? b.num_txs : (b.txs?.length || 0));
          const time = b.time || b.header?.time || '';

          html += '<tr>';
          html += `<td><a class="clickable" href="#/block/${height}">${Number(height).toLocaleString()}</a></td>`;
          html += `<td class="mono">${truncAddr(hash, 8)}</td>`;
          html += `<td class="mono">${truncAddr(proposer)}</td>`;
          html += `<td>${txCount}</td>`;
          html += `<td title="${fullTime(time)}">${relativeTime(time)}</td>`;
          html += '</tr>';
        }

        html += '</tbody></table></div>';
        html += pagination(page, totalPages, p => this.router.navigate(`/blocks/page/${p}`));
        html += '</div>';
        this._render(html);
      } catch (err) {
        this._render(renderError(`Failed to load blocks: ${err.message}`, () => this._pageBlocks(page)));
      }
    }

    // --- Block Detail ---
    async _pageBlockDetail(height) {
      this._render(renderLoading());
      this.currentRefresh = null;

      try {
        const [block, txsData] = await Promise.all([
          this.api.getBlock(height),
          this.api.getTxsByBlock(height).catch(() => ({ txs: [] }))
        ]);

        const b = block.block || block;
        const header = b.header || b;
        const txs = txsData.txs || txsData.data || txsData || [];

        let html = '<a class="back-link" href="#/blocks">\u2190 Back to Blocks</a>';
        html += '<div class="card">';
        html += `<div class="card-header"><span class="card-title">Block #${Number(height).toLocaleString()}</span></div>`;
        html += '<div class="detail-grid">';

        html += `<div class="detail-row"><div class="detail-label">Height</div><div class="detail-value mono">${Number(header.height || height).toLocaleString()}</div></div>`;
        html += `<div class="detail-row"><div class="detail-label">Hash</div><div class="detail-value">${copyableValue(b.hash || b.block_hash || '')}</div></div>`;
        html += `<div class="detail-row"><div class="detail-label">Proposer</div><div class="detail-value">${copyableValue(header.proposer_address || header.proposer || '')}</div></div>`;
        html += `<div class="detail-row"><div class="detail-label">Time</div><div class="detail-value">${fullTime(header.time || b.time || '')}</div></div>`;
        html += `<div class="detail-row"><div class="detail-label">Transactions</div><div class="detail-value">${b.tx_count || b.num_txs || txs.length}</div></div>`;

        if (header.app_hash) {
          html += `<div class="detail-row"><div class="detail-label">App Hash</div><div class="detail-value mono">${truncAddr(header.app_hash, 12)}</div></div>`;
        }

        html += '</div></div>';

        // Transactions in this block
        if (txs.length > 0) {
          html += '<div class="card">';
          html += '<div class="card-header"><span class="card-title">Transactions</span></div>';
          html += '<div class="table-wrap"><table><thead><tr>';
          html += '<th>Hash</th><th>From</th><th>To</th><th>Amount</th><th>Status</th>';
          html += '</tr></thead><tbody>';

          for (const tx of txs) {
            const txHash = tx.hash || tx.tx_hash || '';
            html += '<tr>';
            html += `<td><a class="clickable" href="#/tx/${txHash}">${truncAddr(txHash, 8)}</a></td>`;
            html += `<td class="mono"><a class="clickable" href="#/account/${tx.from || tx.sender || ''}">${truncAddr(tx.from || tx.sender || '')}</a></td>`;
            html += `<td class="mono"><a class="clickable" href="#/account/${tx.to || tx.recipient || ''}">${truncAddr(tx.to || tx.recipient || '')}</a></td>`;
            html += `<td class="amount">${formatUsdc(tx.amount || tx.value || 0)} USDC</td>`;
            html += `<td>${statusBadge(tx.status || 'success')}</td>`;
            html += '</tr>';
          }

          html += '</tbody></table></div></div>';
        }

        this._render(html);
      } catch (err) {
        this._render(renderError(`Failed to load block ${height}: ${err.message}`, () => this._pageBlockDetail(height)));
      }
    }

    // --- Transactions Page ---
    async _pageTxs(page) {
      this._render(renderLoading());
      this.currentRefresh = () => this._pageTxs(page);

      try {
        const data = await this.api.getTxs(page, PAGE_SIZE);
        const txs = data.txs || [];
        const total = data.total || txs.length;
        const totalPages = Math.max(1, Math.ceil(total / PAGE_SIZE));

        let html = '<div class="card">';
        html += '<div class="card-header"><span class="card-title">Recent Transactions</span>';
        html += `<span class="card-badge">${Number(total).toLocaleString()} total</span></div>`;

        // Network stats bar
        html += '<div style="padding:0.75rem 1rem;border-bottom:1px solid var(--border-color);display:flex;gap:2rem;font-size:0.75rem;color:var(--text-dim)">';
        html += '<span>Block time: <strong style="color:var(--accent)">100ms</strong></span>';
        html += '<span>Finality: <strong style="color:var(--accent)">1 block (instant)</strong></span>';
        html += '<span>Fees: <strong style="color:var(--accent)">$0.00</strong></span>';
        html += '<span>Network: <strong style="color:var(--text)">Dina Testnet</strong></span>';
        html += '</div>';

        html += '<div class="table-wrap"><table><thead><tr>';
        html += '<th>Hash</th><th>Type</th><th>From</th><th>To</th><th>Amount</th><th>Fee</th><th>Block</th><th>Status</th>';
        html += '</tr></thead><tbody>';

        if (txs.length === 0) {
          html += `<tr><td colspan="8">${renderEmpty('No transactions yet — send some USDC!')}</td></tr>`;
        }

        for (const tx of txs) {
          const txHash = tx.hash || tx.tx_hash || '';
          const from = tx.from || tx.sender || '';
          const to = tx.to || tx.recipient || '';
          const blockH = tx.block || tx.block_height || tx.height || '';
          const txType = tx.type || tx.tx_type || 'transfer';
          const isFaucet = from.replace(/0x/,'').replace(/0/g,'') === '';
          const typeLabel = isFaucet ? 'faucet' : txType;
          const typeColor = isFaucet ? '#34d399' : txType === 'transfer' ? '#60a5fa' : '#94a3b8';

          // Main row
          html += `<tr class="clickable" onclick="this.nextElementSibling.style.display=this.nextElementSibling.style.display==='none'?'table-row':'none'" style="cursor:pointer">`;
          html += `<td><span class="mono" style="color:var(--accent)">${truncAddr(txHash, 10)}</span></td>`;
          html += `<td><span style="background:${typeColor}22;color:${typeColor};padding:2px 8px;border-radius:4px;font-size:0.7rem;font-weight:600;text-transform:uppercase">${typeLabel}</span></td>`;
          html += `<td class="mono"><a class="clickable" href="#/account/${from}">${truncAddr(from)}</a></td>`;
          html += `<td class="mono"><a class="clickable" href="#/account/${to}">${truncAddr(to)}</a></td>`;
          html += `<td class="amount" style="font-weight:600">${formatUsdc(tx.amount || tx.value || 0)} USDC</td>`;
          html += `<td class="amount" style="color:var(--accent)">$0.00</td>`;
          html += `<td><a class="clickable" href="#/block/${blockH}">#${Number(blockH).toLocaleString()}</a></td>`;
          html += `<td>${statusBadge('confirmed')}</td>`;
          html += '</tr>';

          // Expandable detail row
          html += `<tr style="display:none;background:var(--bg-darker)"><td colspan="8" style="padding:1rem">`;
          html += '<div style="display:grid;grid-template-columns:1fr 1fr;gap:0.5rem;font-size:0.75rem">';
          html += `<div><span style="color:var(--text-dim)">Transaction Hash</span><br><span class="mono" style="word-break:break-all">${txHash}</span></div>`;
          html += `<div><span style="color:var(--text-dim)">Block</span><br><a class="clickable" href="#/block/${blockH}">#${Number(blockH).toLocaleString()}</a></div>`;
          html += `<div><span style="color:var(--text-dim)">From</span><br><a class="clickable mono" href="#/account/${from}" style="word-break:break-all">${from}</a></div>`;
          html += `<div><span style="color:var(--text-dim)">To</span><br><a class="clickable mono" href="#/account/${to}" style="word-break:break-all">${to}</a></div>`;
          html += `<div><span style="color:var(--text-dim)">Amount</span><br><strong>${formatUsdc(tx.amount || 0)} USDC</strong></div>`;
          html += `<div><span style="color:var(--text-dim)">Fee</span><br><strong style="color:var(--accent)">$0.00 (zero fees)</strong></div>`;
          html += `<div><span style="color:var(--text-dim)">Nonce</span><br><span class="mono">${tx.nonce || 0}</span></div>`;
          html += `<div><span style="color:var(--text-dim)">Status</span><br>${statusBadge('confirmed')} <span style="color:var(--accent)">Finalized (1 block = 100ms)</span></div>`;
          html += `<div><span style="color:var(--text-dim)">Finality</span><br><strong style="color:var(--accent)">100ms (instant, irreversible)</strong></div>`;
          html += `<div><span style="color:var(--text-dim)">Type</span><br><span style="text-transform:capitalize">${typeLabel}</span></div>`;
          html += '</div>';
          html += '</td></tr>';
        }

        html += '</tbody></table></div>';
        html += pagination(page, totalPages, p => this.router.navigate(`/txs/page/${p}`));
        html += '</div>';
        this._render(html);
      } catch (err) {
        this._render(renderError(`Failed to load transactions: ${err.message}`, () => this._pageTxs(page)));
      }
    }

    // --- Transaction Detail ---
    async _pageTxDetail(hash) {
      this._render(renderLoading());
      this.currentRefresh = null;

      try {
        const tx = await this.api.getTx(hash);
        const t = tx.tx || tx;

        let html = '<a class="back-link" href="#/txs">\u2190 Back to Transactions</a>';
        html += '<div class="card">';
        html += `<div class="card-header"><span class="card-title">Transaction Details</span>${statusBadge(t.status || 'success')}</div>`;
        html += '<div class="detail-grid">';

        html += `<div class="detail-row"><div class="detail-label">Hash</div><div class="detail-value">${copyableValue(t.hash || t.tx_hash || hash)}</div></div>`;
        html += `<div class="detail-row"><div class="detail-label">Status</div><div class="detail-value">${statusBadge(t.status || 'success')}</div></div>`;
        html += `<div class="detail-row"><div class="detail-label">Block</div><div class="detail-value"><a class="clickable" href="#/block/${t.block_height || t.height || ''}">${Number(t.block_height || t.height || 0).toLocaleString()}</a></div></div>`;
        html += `<div class="detail-row"><div class="detail-label">Time</div><div class="detail-value">${fullTime(t.time || t.timestamp || '')}</div></div>`;

        const from = t.from || t.sender || '';
        const to = t.to || t.recipient || '';
        html += `<div class="detail-row"><div class="detail-label">From</div><div class="detail-value"><a class="clickable" href="#/account/${from}">${copyableValue(from)}</a></div></div>`;
        html += `<div class="detail-row"><div class="detail-label">To</div><div class="detail-value"><a class="clickable" href="#/account/${to}">${copyableValue(to)}</a></div></div>`;
        html += `<div class="detail-row"><div class="detail-label">Amount</div><div class="detail-value amount">${formatUsdc(t.amount || t.value || 0)} USDC</div></div>`;
        html += `<div class="detail-row"><div class="detail-label">Fee</div><div class="detail-value amount">${formatUsdc(t.fee || 0)} USDC</div></div>`;

        if (t.type || t.tx_type) {
          html += `<div class="detail-row"><div class="detail-label">Type</div><div class="detail-value">${escapeHtml(t.type || t.tx_type)}</div></div>`;
        }
        if (t.memo) {
          html += `<div class="detail-row"><div class="detail-label">Memo</div><div class="detail-value">${escapeHtml(t.memo)}</div></div>`;
        }
        if (t.nonce != null) {
          html += `<div class="detail-row"><div class="detail-label">Nonce</div><div class="detail-value mono">${t.nonce}</div></div>`;
        }
        if (t.gas_used != null) {
          html += `<div class="detail-row"><div class="detail-label">Gas Used</div><div class="detail-value mono">${Number(t.gas_used).toLocaleString()}</div></div>`;
        }

        html += '</div></div>';
        this._render(html);
      } catch (err) {
        this._render(renderError(`Failed to load transaction: ${err.message}`, () => this._pageTxDetail(hash)));
      }
    }

    // --- Account Search ---
    _pageAccountSearch() {
      this.currentRefresh = null;
      let html = '<div class="card">';
      html += '<div class="card-header"><span class="card-title">Lookup Account</span></div>';
      html += '<div class="search-container"><div class="search-box">';
      html += '<input class="search-input" id="account-input" type="text" placeholder="Enter account address (0x...)">';
      html += '<button class="search-btn" id="account-go">View Account</button>';
      html += '</div></div></div>';
      this._render(html);

      const goFn = () => {
        const val = document.getElementById('account-input').value.trim();
        if (val) this.router.navigate(`/account/${val}`);
      };
      document.getElementById('account-go').addEventListener('click', goFn);
      document.getElementById('account-input').addEventListener('keydown', e => {
        if (e.key === 'Enter') goFn();
      });
    }

    // --- Account Detail ---
    async _pageAccountDetail(addr, page) {
      this._render(renderLoading());
      this.currentRefresh = () => this._pageAccountDetail(addr, page);

      try {
        const [acct, txsData] = await Promise.all([
          this.api.getAccount(addr),
          this.api.getAccountTxs(addr, page, PAGE_SIZE).catch(() => ({ txs: [], total: 0 }))
        ]);

        const a = acct.account || acct;
        const txs = txsData.txs || txsData.data || txsData || [];
        const total = txsData.total_count || txsData.total || txs.length;
        const totalPages = Math.max(1, Math.ceil(total / PAGE_SIZE));

        let html = '<a class="back-link" href="#/accounts">\u2190 Back</a>';

        // Balance card
        html += '<div class="balance-card">';
        html += '<div>';
        html += `<div class="balance-label">Balance</div>`;
        html += `<div class="balance-amount">${formatUsdc(a.balance || 0)} USDC</div>`;
        html += '</div>';
        html += '<div style="margin-left:auto; text-align:right;">';
        html += `<div class="balance-label">Nonce</div>`;
        html += `<div style="font-size:1.2rem; font-family:var(--font-mono); color:var(--text-primary)">${a.nonce || 0}</div>`;
        html += '</div>';
        html += '</div>';

        // Account info
        html += '<div class="card">';
        html += '<div class="card-header"><span class="card-title">Account Info</span></div>';
        html += '<div class="detail-grid">';
        html += `<div class="detail-row"><div class="detail-label">Address</div><div class="detail-value">${copyableValue(a.address || addr)}</div></div>`;

        if (a.type) {
          html += `<div class="detail-row"><div class="detail-label">Type</div><div class="detail-value">${escapeHtml(a.type)}</div></div>`;
        }
        if (a.created_at) {
          html += `<div class="detail-row"><div class="detail-label">Created</div><div class="detail-value">${fullTime(a.created_at)}</div></div>`;
        }
        html += '</div></div>';

        // Transaction history
        html += '<div class="card">';
        html += `<div class="card-header"><span class="card-title">Transaction History</span><span class="card-badge">${Number(total).toLocaleString()} txs</span></div>`;

        if (txs.length > 0) {
          html += '<div class="table-wrap"><table><thead><tr>';
          html += '<th>Hash</th><th>Direction</th><th>Counterparty</th><th>Amount</th><th>Time</th>';
          html += '</tr></thead><tbody>';

          for (const tx of txs) {
            const txHash = tx.hash || tx.tx_hash || '';
            const from = tx.from || tx.sender || '';
            const to = tx.to || tx.recipient || '';
            const isSend = from.toLowerCase() === addr.toLowerCase();
            const counterparty = isSend ? to : from;

            html += '<tr>';
            html += `<td><a class="clickable" href="#/tx/${txHash}">${truncAddr(txHash, 8)}</a></td>`;
            html += `<td>${isSend ? '<span style="color:var(--error)">OUT</span>' : '<span style="color:var(--success)">IN</span>'}</td>`;
            html += `<td class="mono"><a class="clickable" href="#/account/${counterparty}">${truncAddr(counterparty)}</a></td>`;
            html += `<td class="amount ${isSend ? 'amount-negative' : 'amount-positive'}">${isSend ? '-' : '+'}${formatUsdc(tx.amount || tx.value || 0)}</td>`;
            html += `<td title="${fullTime(tx.time || tx.timestamp)}">${relativeTime(tx.time || tx.timestamp)}</td>`;
            html += '</tr>';
          }

          html += '</tbody></table></div>';
          html += pagination(page, totalPages, p => this.router.navigate(`/account/${addr}/page/${p}`));
        } else {
          html += renderEmpty('No transactions for this account');
        }

        html += '</div>';
        this._render(html);
      } catch (err) {
        this._render(renderError(`Failed to load account ${truncAddr(addr)}: ${err.message}`, () => this._pageAccountDetail(addr, page)));
      }
    }

    // --- Devices Page ---
    async _pageDevices(page) {
      this._render(renderLoading());
      this.currentRefresh = () => this._pageDevices(page);

      try {
        const data = await this.api.getDevices(page, PAGE_SIZE);
        const devices = data.devices || data.data || data || [];
        const total = data.total_count || data.total || devices.length;
        const totalPages = Math.max(1, Math.ceil(total / PAGE_SIZE));

        let html = '<div class="card">';
        html += '<div class="card-header"><span class="card-title">Registered Devices</span>';
        html += `<span class="card-badge">${Number(total).toLocaleString()} devices</span></div>`;
        html += '<div class="table-wrap"><table><thead><tr>';
        html += '<th>Device ID</th><th>Owner</th><th>Type</th><th>Attestation</th><th>Status</th><th>Registered</th>';
        html += '</tr></thead><tbody>';

        if (devices.length === 0) {
          html += `<tr><td colspan="6">${renderEmpty('No devices registered')}</td></tr>`;
        }

        for (const d of devices) {
          const id = d.device_id || d.id || '';
          const owner = d.owner || d.owner_address || '';
          const attScore = d.attestation_score != null ? d.attestation_score : (d.trust_score != null ? d.trust_score : null);

          html += '<tr>';
          html += `<td><a class="clickable" href="#/device/${id}">${truncAddr(id, 8)}</a></td>`;
          html += `<td class="mono"><a class="clickable" href="#/account/${owner}">${truncAddr(owner)}</a></td>`;
          html += `<td>${escapeHtml(d.device_type || d.type || 'Unknown')}</td>`;
          html += '<td>';
          if (attScore != null) {
            const pct = Math.min(100, Math.max(0, Number(attScore)));
            html += `<div style="display:flex;align-items:center;gap:8px;">`;
            html += `<div class="attestation-bar"><div class="attestation-fill" style="width:${pct}%"></div></div>`;
            html += `<span class="mono" style="font-size:0.8rem">${pct}%</span></div>`;
          } else {
            html += '-';
          }
          html += '</td>';
          html += `<td>${statusBadge(d.status || 'active')}</td>`;
          html += `<td title="${fullTime(d.registered_at || d.created_at)}">${relativeTime(d.registered_at || d.created_at)}</td>`;
          html += '</tr>';
        }

        html += '</tbody></table></div>';
        html += pagination(page, totalPages, p => this.router.navigate(`/devices/page/${p}`));
        html += '</div>';
        this._render(html);
      } catch (err) {
        this._render(renderError(`Failed to load devices: ${err.message}`, () => this._pageDevices(page)));
      }
    }

    // --- Device Detail ---
    async _pageDeviceDetail(id) {
      this._render(renderLoading());
      this.currentRefresh = null;

      try {
        const device = await this.api.getDevice(id);
        const d = device.device || device;

        let html = '<a class="back-link" href="#/devices">\u2190 Back to Devices</a>';
        html += '<div class="card">';
        html += `<div class="card-header"><span class="card-title">Device Details</span>${statusBadge(d.status || 'active')}</div>`;
        html += '<div class="detail-grid">';

        html += `<div class="detail-row"><div class="detail-label">Device ID</div><div class="detail-value">${copyableValue(d.device_id || d.id || id)}</div></div>`;
        html += `<div class="detail-row"><div class="detail-label">Owner</div><div class="detail-value"><a class="clickable" href="#/account/${d.owner || d.owner_address || ''}">${copyableValue(d.owner || d.owner_address || '')}</a></div></div>`;
        html += `<div class="detail-row"><div class="detail-label">Type</div><div class="detail-value">${escapeHtml(d.device_type || d.type || 'Unknown')}</div></div>`;
        html += `<div class="detail-row"><div class="detail-label">Status</div><div class="detail-value">${statusBadge(d.status || 'active')}</div></div>`;

        const attScore = d.attestation_score != null ? d.attestation_score : d.trust_score;
        if (attScore != null) {
          const pct = Math.min(100, Math.max(0, Number(attScore)));
          html += `<div class="detail-row"><div class="detail-label">Attestation</div><div class="detail-value">`;
          html += `<div style="display:flex;align-items:center;gap:12px;">`;
          html += `<div class="attestation-bar" style="width:160px"><div class="attestation-fill" style="width:${pct}%"></div></div>`;
          html += `<span class="mono">${pct}%</span></div></div></div>`;
        }

        if (d.firmware_version) {
          html += `<div class="detail-row"><div class="detail-label">Firmware</div><div class="detail-value mono">${escapeHtml(d.firmware_version)}</div></div>`;
        }
        if (d.model) {
          html += `<div class="detail-row"><div class="detail-label">Model</div><div class="detail-value">${escapeHtml(d.model)}</div></div>`;
        }
        if (d.registered_at || d.created_at) {
          html += `<div class="detail-row"><div class="detail-label">Registered</div><div class="detail-value">${fullTime(d.registered_at || d.created_at)}</div></div>`;
        }
        if (d.last_seen || d.last_active) {
          html += `<div class="detail-row"><div class="detail-label">Last Seen</div><div class="detail-value">${fullTime(d.last_seen || d.last_active)}</div></div>`;
        }
        if (d.attestation_hash) {
          html += `<div class="detail-row"><div class="detail-label">Attestation Hash</div><div class="detail-value">${copyableValue(d.attestation_hash)}</div></div>`;
        }

        html += '</div></div>';
        this._render(html);
      } catch (err) {
        this._render(renderError(`Failed to load device: ${err.message}`, () => this._pageDeviceDetail(id)));
      }
    }

    // --- Search Page ---
    _pageSearch() {
      this.currentRefresh = null;

      let html = '<div class="card">';
      html += '<div class="card-header"><span class="card-title">Search</span></div>';
      html += '<div class="search-container"><div class="search-box">';
      html += '<input class="search-input" id="search-input" type="text" placeholder="Search by block height, tx hash, address, or device ID">';
      html += '<button class="search-btn" id="search-go">Search</button>';
      html += '</div></div>';
      html += '<div id="search-results"></div>';
      html += '</div>';
      this._render(html);

      const doSearch = () => {
        const val = document.getElementById('search-input').value.trim();
        if (!val) return;
        this._executeSearch(val);
      };
      document.getElementById('search-go').addEventListener('click', doSearch);
      document.getElementById('search-input').addEventListener('keydown', e => {
        if (e.key === 'Enter') doSearch();
      });
      document.getElementById('search-input').focus();
    }

    async _executeSearch(query) {
      const results = document.getElementById('search-results');
      if (!results) return;
      results.innerHTML = renderLoading();

      // Detect query type and navigate directly
      const q = query.trim();

      // Pure number => block height
      if (/^\d+$/.test(q)) {
        this.router.navigate(`/block/${q}`);
        return;
      }

      // 0x with 64 hex chars => tx hash
      if (/^0x[0-9a-fA-F]{64}$/.test(q) || /^[0-9a-fA-F]{64}$/.test(q)) {
        this.router.navigate(`/tx/${q}`);
        return;
      }

      // 0x with 40 hex chars => address
      if (/^0x[0-9a-fA-F]{40}$/.test(q)) {
        this.router.navigate(`/account/${q}`);
        return;
      }

      // Otherwise try the search API
      try {
        const data = await this.api.search(q);
        if (data.type === 'block' && data.result) {
          this.router.navigate(`/block/${data.result.height || q}`);
        } else if (data.type === 'tx' && data.result) {
          this.router.navigate(`/tx/${data.result.hash || q}`);
        } else if (data.type === 'account' && data.result) {
          this.router.navigate(`/account/${data.result.address || q}`);
        } else if (data.type === 'device' && data.result) {
          this.router.navigate(`/device/${data.result.device_id || data.result.id || q}`);
        } else {
          results.innerHTML = renderEmpty(`No results found for "${escapeHtml(q)}"`);
        }
      } catch (err) {
        // If search API fails, try heuristic navigation
        if (q.startsWith('0x') || q.startsWith('dina')) {
          // Could be address or device
          results.innerHTML = `<div style="padding:16px">
            <p style="color:var(--text-secondary);margin-bottom:12px">Could not determine type automatically. Try:</p>
            <div style="display:flex;gap:8px;flex-wrap:wrap">
              <a href="#/account/${q}" class="page-btn">View as Account</a>
              <a href="#/device/${q}" class="page-btn">View as Device</a>
              <a href="#/tx/${q}" class="page-btn">View as Transaction</a>
            </div>
          </div>`;
        } else {
          results.innerHTML = renderError(`Search failed: ${err.message}`);
        }
      }
    }
  }

  // ---------------------------------------------------------------------------
  // Boot
  // ---------------------------------------------------------------------------

  // Global search function for transactions page
  window._searchTxs = function() {
    const input = document.getElementById('tx-search-input');
    if (input && input.value.length >= 32) {
      window.location.hash = '#/account/' + input.value.trim();
    }
  };

  document.addEventListener('DOMContentLoaded', () => {
    window.explorer = new DinaExplorer();
  });

})();
