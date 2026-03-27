// =============================================================================
// Dina Network Faucet — Frontend Application
// Communicates with the faucet REST API at /faucet/*
// =============================================================================

(function () {
  'use strict';

  // ---------------------------------------------------------------------------
  // Configuration
  // ---------------------------------------------------------------------------

  const CONFIG = {
    // Faucet API base URL — defaults to same origin, override via data attribute
    apiBase: document.documentElement.dataset.faucetApi || '/faucet',
    // Explorer base URL for transaction links
    explorerBase: document.documentElement.dataset.explorerUrl || '/explorer',
    // How often to refresh chain stats (ms)
    statsRefreshInterval: 15000,
    // Drip amount display
    dripAmount: '100',
    // Cooldown seconds (updated from API)
    cooldownSeconds: 60,
    // Address length in bytes
    addressBytes: 32,
  };

  // ---------------------------------------------------------------------------
  // DOM References
  // ---------------------------------------------------------------------------

  const dom = {
    addressInput: document.getElementById('address-input'),
    addressError: document.getElementById('address-error'),
    requestBtn: document.getElementById('request-btn'),
    btnText: document.getElementById('btn-text'),
    btnSpinner: document.getElementById('btn-spinner'),

    cooldown: document.getElementById('cooldown'),
    cooldownTimer: document.getElementById('cooldown-timer'),

    statusBox: document.getElementById('status-box'),
    statusTitle: document.getElementById('status-title'),
    statusDetail: document.getElementById('status-detail'),
    statusLink: document.getElementById('status-link'),

    balanceDisplay: document.getElementById('balance-display'),
    balanceAmount: document.getElementById('balance-amount'),

    historyList: document.getElementById('history-list'),
    historyEmpty: document.getElementById('history-empty'),

    // Sidebar stats
    statDispensed: document.getElementById('stat-dispensed'),
    statAddresses: document.getElementById('stat-addresses'),
    statRequests: document.getElementById('stat-requests'),
    statDrip: document.getElementById('stat-drip'),
    statCooldown: document.getElementById('stat-cooldown'),
    statDailyLimit: document.getElementById('stat-daily-limit'),
  };

  // ---------------------------------------------------------------------------
  // State
  // ---------------------------------------------------------------------------

  let state = {
    loading: false,
    cooldownRemaining: 0,
    cooldownInterval: null,
    currentAddress: '',
    totalReceived: 0,
    history: [],
  };

  // ---------------------------------------------------------------------------
  // Address Validation
  // ---------------------------------------------------------------------------

  function normalizeAddress(input) {
    let hex = input.trim();
    if (hex.startsWith('0x') || hex.startsWith('0X')) {
      hex = hex.slice(2);
    }
    return hex.toLowerCase();
  }

  function isValidAddress(hex) {
    if (hex.length !== CONFIG.addressBytes * 2) return false;
    return /^[0-9a-f]+$/.test(hex);
  }

  function validateInput() {
    const raw = dom.addressInput.value;
    const hex = normalizeAddress(raw);

    if (raw.trim() === '') {
      hideError();
      return false;
    }

    if (!isValidAddress(hex)) {
      if (hex.length > 0 && hex.length !== 64) {
        showError('Address must be 64 hex characters (32 bytes)');
      } else if (!/^[0-9a-f]*$/.test(hex)) {
        showError('Address contains invalid characters');
      }
      return false;
    }

    hideError();
    return true;
  }

  function showError(msg) {
    dom.addressError.textContent = msg;
    dom.addressError.classList.add('visible');
    dom.addressInput.classList.add('form-input--error');
  }

  function hideError() {
    dom.addressError.classList.remove('visible');
    dom.addressInput.classList.remove('form-input--error');
  }

  // ---------------------------------------------------------------------------
  // API Calls
  // ---------------------------------------------------------------------------

  async function requestFunds(addressHex) {
    const response = await fetch(`${CONFIG.apiBase}/request`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ address: addressHex }),
    });

    const data = await response.json();

    if (!response.ok) {
      const error = new Error(data.error || 'Request failed');
      error.status = response.status;
      error.data = data;
      throw error;
    }

    return data;
  }

  async function fetchStatus(addressHex) {
    const response = await fetch(`${CONFIG.apiBase}/status/${addressHex}`);
    if (!response.ok) {
      throw new Error('Failed to fetch status');
    }
    return response.json();
  }

  async function fetchStats() {
    const response = await fetch(`${CONFIG.apiBase}/stats`);
    if (!response.ok) {
      throw new Error('Failed to fetch stats');
    }
    return response.json();
  }

  // ---------------------------------------------------------------------------
  // UI Updates
  // ---------------------------------------------------------------------------

  function setLoading(loading) {
    state.loading = loading;
    dom.requestBtn.disabled = loading;
    if (loading) {
      dom.requestBtn.classList.add('btn--loading');
    } else {
      dom.requestBtn.classList.remove('btn--loading');
    }
  }

  function showStatus(type, title, detail, txHash) {
    dom.statusBox.className = `status visible status--${type}`;
    dom.statusTitle.textContent = title;
    dom.statusDetail.textContent = detail || '';

    if (txHash) {
      dom.statusLink.href = `${CONFIG.explorerBase}/#/tx/${txHash}`;
      dom.statusLink.style.display = 'inline-flex';
    } else {
      dom.statusLink.style.display = 'none';
    }
  }

  function hideStatus() {
    dom.statusBox.className = 'status';
  }

  function showBalance(totalMicroUnits) {
    const usdc = (totalMicroUnits / 1_000_000).toFixed(2);
    dom.balanceAmount.textContent = usdc;
    dom.balanceDisplay.classList.add('visible');
  }

  function hideBalance() {
    dom.balanceDisplay.classList.remove('visible');
  }

  // ---------------------------------------------------------------------------
  // Cooldown Timer
  // ---------------------------------------------------------------------------

  function startCooldown(seconds) {
    stopCooldown();
    state.cooldownRemaining = seconds;
    dom.requestBtn.disabled = true;

    updateCooldownDisplay();
    dom.cooldown.classList.add('visible');

    state.cooldownInterval = setInterval(function () {
      state.cooldownRemaining--;
      if (state.cooldownRemaining <= 0) {
        stopCooldown();
        dom.requestBtn.disabled = false;
      } else {
        updateCooldownDisplay();
      }
    }, 1000);
  }

  function stopCooldown() {
    if (state.cooldownInterval) {
      clearInterval(state.cooldownInterval);
      state.cooldownInterval = null;
    }
    dom.cooldown.classList.remove('visible');
    state.cooldownRemaining = 0;
  }

  function updateCooldownDisplay() {
    const mins = Math.floor(state.cooldownRemaining / 60);
    const secs = state.cooldownRemaining % 60;
    dom.cooldownTimer.textContent = mins > 0
      ? `${mins}:${secs.toString().padStart(2, '0')}`
      : `${secs}s`;
  }

  // ---------------------------------------------------------------------------
  // Transaction History
  // ---------------------------------------------------------------------------

  function addToHistory(item) {
    state.history.unshift(item);
    renderHistory();
  }

  function renderHistory() {
    if (state.history.length === 0) {
      dom.historyEmpty.style.display = 'block';
      dom.historyList.innerHTML = '';
      return;
    }

    dom.historyEmpty.style.display = 'none';
    dom.historyList.innerHTML = state.history.map(function (item) {
      var timeStr = formatTimestamp(item.timestamp);
      var amountStr = formatUSDC(item.amount);
      var txShort = item.tx_hash
        ? item.tx_hash.substring(0, 16) + '...'
        : 'pending';

      return [
        '<li class="history__item">',
        '  <div class="history__icon">+</div>',
        '  <div class="history__details">',
        '    <div class="history__amount">+' + amountStr + '</div>',
        '    <div class="history__time">' + timeStr + '</div>',
        '    <div class="history__tx">' + txShort + '</div>',
        '  </div>',
        '</li>',
      ].join('\n');
    }).join('\n');
  }

  // ---------------------------------------------------------------------------
  // Chain Stats
  // ---------------------------------------------------------------------------

  async function refreshStats() {
    try {
      var stats = await fetchStats();

      if (dom.statDispensed) {
        dom.statDispensed.textContent = formatUSDC(stats.total_dispensed);
      }
      if (dom.statAddresses) {
        dom.statAddresses.textContent = stats.unique_addresses.toLocaleString();
      }
      if (dom.statRequests) {
        dom.statRequests.textContent = stats.total_requests.toLocaleString();
      }
      if (dom.statDrip) {
        dom.statDrip.textContent = formatUSDC(stats.drip_amount);
      }
      if (dom.statCooldown) {
        dom.statCooldown.textContent = stats.cooldown_seconds + 's';
        CONFIG.cooldownSeconds = stats.cooldown_seconds;
      }
      if (dom.statDailyLimit) {
        dom.statDailyLimit.textContent = formatUSDC(stats.max_per_address_per_day);
      }
    } catch (e) {
      console.warn('Failed to fetch faucet stats:', e.message);
    }
  }

  // ---------------------------------------------------------------------------
  // Main Request Handler
  // ---------------------------------------------------------------------------

  async function handleRequest() {
    var raw = dom.addressInput.value;
    var hex = normalizeAddress(raw);

    if (!isValidAddress(hex)) {
      showError('Enter a valid 64-character hex address');
      return;
    }

    hideError();
    hideStatus();
    setLoading(true);
    state.currentAddress = hex;

    try {
      var result = await requestFunds(hex);

      showStatus(
        'success',
        'Funds sent successfully!',
        result.amount_display + ' sent to ' + hex.substring(0, 12) + '...',
        result.tx_hash || null
      );

      addToHistory({
        amount: result.amount,
        timestamp: result.timestamp,
        tx_hash: result.tx_hash || null,
        address: hex,
      });

      // Update balance
      state.totalReceived += result.amount;
      showBalance(state.totalReceived);

      // Start cooldown timer
      startCooldown(CONFIG.cooldownSeconds);

      // Refresh stats
      refreshStats();

    } catch (err) {
      if (err.status === 429) {
        // Rate limited — parse remaining seconds if available
        var match = err.message.match(/(\d+)\s*seconds/);
        var remaining = match ? parseInt(match[1], 10) : CONFIG.cooldownSeconds;

        showStatus('error', 'Rate Limited', err.message);
        startCooldown(remaining);
      } else {
        showStatus('error', 'Request Failed', err.message || 'Unknown error');
      }
    } finally {
      setLoading(false);
    }
  }

  // ---------------------------------------------------------------------------
  // Check Status on Address Input
  // ---------------------------------------------------------------------------

  var statusCheckDebounce = null;

  async function checkAddressStatus() {
    var hex = normalizeAddress(dom.addressInput.value);
    if (!isValidAddress(hex)) return;

    try {
      var status = await fetchStatus(hex);
      state.currentAddress = hex;
      state.totalReceived = status.total_received;

      if (status.total_received > 0) {
        showBalance(status.total_received);
      }

      if (!status.can_request && status.seconds_until_next > 0) {
        startCooldown(status.seconds_until_next);
      } else {
        stopCooldown();
        dom.requestBtn.disabled = false;
      }
    } catch (e) {
      // Silently fail — status check is best-effort
    }
  }

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------

  function formatUSDC(microUnits) {
    var whole = Math.floor(microUnits / 1_000_000);
    var frac = microUnits % 1_000_000;
    if (frac === 0) {
      return whole.toLocaleString() + ' USDC';
    }
    var fracStr = frac.toString().padStart(6, '0').replace(/0+$/, '');
    return whole.toLocaleString() + '.' + fracStr + ' USDC';
  }

  function formatTimestamp(unixSeconds) {
    var date = new Date(unixSeconds * 1000);
    var now = new Date();
    var diffMs = now - date;
    var diffSecs = Math.floor(diffMs / 1000);

    if (diffSecs < 60) return 'Just now';
    if (diffSecs < 3600) return Math.floor(diffSecs / 60) + 'm ago';
    if (diffSecs < 86400) return Math.floor(diffSecs / 3600) + 'h ago';

    return date.toLocaleDateString(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function copyToClipboard(text) {
    if (navigator.clipboard) {
      navigator.clipboard.writeText(text);
    }
  }

  // ---------------------------------------------------------------------------
  // Event Listeners
  // ---------------------------------------------------------------------------

  dom.requestBtn.addEventListener('click', handleRequest);

  dom.addressInput.addEventListener('input', function () {
    validateInput();

    // Debounced status check
    clearTimeout(statusCheckDebounce);
    var hex = normalizeAddress(dom.addressInput.value);
    if (isValidAddress(hex)) {
      statusCheckDebounce = setTimeout(checkAddressStatus, 500);
    } else {
      hideBalance();
      stopCooldown();
    }
  });

  dom.addressInput.addEventListener('keydown', function (e) {
    if (e.key === 'Enter' && !state.loading && !dom.requestBtn.disabled) {
      handleRequest();
    }
  });

  // Copyable values in the sidebar
  document.querySelectorAll('.network-info__value--copyable').forEach(function (el) {
    el.addEventListener('click', function () {
      copyToClipboard(el.textContent);
      var original = el.textContent;
      el.textContent = 'Copied!';
      setTimeout(function () { el.textContent = original; }, 1000);
    });
  });

  // ---------------------------------------------------------------------------
  // Initialize
  // ---------------------------------------------------------------------------

  refreshStats();
  setInterval(refreshStats, CONFIG.statsRefreshInterval);
  renderHistory();

  // Focus the input on load
  dom.addressInput.focus();

})();
