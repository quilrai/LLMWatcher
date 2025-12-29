import { invoke, getCurrentPort, setCurrentPort, escapeHtml } from './utils.js';

// Current MITM port
let currentMitmPort = 8888;

// ============ Status Display ============

// Show status message in settings
function showSettingsStatus(message, type, elementId = 'settings-status') {
  const status = document.getElementById(elementId);
  if (!status) return;
  status.textContent = message;
  status.className = 'settings-status show ' + type;

  // Auto-hide after 5 seconds for success
  if (type === 'success') {
    setTimeout(() => {
      status.className = 'settings-status';
    }, 5000);
  }
}

// Update sidebar port display
function updateProxyStatusDisplay(port, isRestarting = false) {
  const statusText = document.getElementById('proxy-status-text');
  const statusDot = document.getElementById('proxy-status-dot');

  if (statusText) {
    statusText.textContent = `Proxy: localhost:${port}`;
  }

  if (statusDot) {
    if (isRestarting) {
      statusDot.classList.add('restarting');
    } else {
      statusDot.classList.remove('restarting');
    }
  }
}

// ============ Port Settings ============

// Load port setting from backend
export async function loadPortSetting() {
  try {
    const port = await invoke('get_port_setting');
    setCurrentPort(port);
    const portInput = document.getElementById('port-input');
    if (portInput) {
      portInput.value = port;
    }
    updateProxyStatusDisplay(port);
  } catch (error) {
    console.error('Failed to load port setting:', error);
  }
}

// Save port setting and restart proxy
async function savePortSetting() {
  const portInput = document.getElementById('port-input');
  const saveBtn = document.getElementById('save-port-btn');
  const port = parseInt(portInput.value, 10);
  const currentPort = getCurrentPort();

  // Validate
  if (isNaN(port) || port < 1024 || port > 65535) {
    showSettingsStatus('Port must be between 1024 and 65535', 'error');
    return;
  }

  // Skip if port hasn't changed
  if (port === currentPort) {
    showSettingsStatus('Port unchanged', 'info');
    return;
  }

  saveBtn.disabled = true;
  saveBtn.textContent = 'Saving...';
  updateProxyStatusDisplay(port, true);

  try {
    // Save the port setting
    await invoke('save_port_setting', { port });
    setCurrentPort(port);

    // Restart the proxy server
    showSettingsStatus('Restarting proxy server...', 'info');
    await invoke('restart_proxy');

    // Wait for server to restart
    await new Promise(resolve => setTimeout(resolve, 1500));
    updateProxyStatusDisplay(port, false);
    showSettingsStatus(`Proxy server now running on port ${port}`, 'success');
  } catch (error) {
    updateProxyStatusDisplay(currentPort, false);
    showSettingsStatus(`Failed: ${error}`, 'error');
  } finally {
    saveBtn.disabled = false;
    saveBtn.textContent = 'Save';
  }
}

// ============ MITM Proxy Settings ============

// Load MITM port setting from backend
async function loadMitmPortSetting() {
  try {
    const port = await invoke('get_mitm_port_setting');
    currentMitmPort = port;
    const portInput = document.getElementById('mitm-port-input');
    if (portInput) {
      portInput.value = port;
    }
  } catch (error) {
    console.error('Failed to load MITM port setting:', error);
  }
}

// Save MITM port setting and restart proxy
async function saveMitmPortSetting() {
  const portInput = document.getElementById('mitm-port-input');
  const saveBtn = document.getElementById('save-mitm-port-btn');
  const port = parseInt(portInput.value, 10);

  // Validate
  if (isNaN(port) || port < 1024 || port > 65535) {
    showSettingsStatus('Port must be between 1024 and 65535', 'error', 'mitm-settings-status');
    return;
  }

  // Skip if port hasn't changed
  if (port === currentMitmPort) {
    showSettingsStatus('Port unchanged', 'info', 'mitm-settings-status');
    return;
  }

  saveBtn.disabled = true;
  saveBtn.textContent = 'Saving...';

  try {
    // Save the port setting
    await invoke('save_mitm_port_setting', { port });
    currentMitmPort = port;

    // Restart the MITM proxy server
    showSettingsStatus('Restarting MITM proxy server...', 'info', 'mitm-settings-status');
    await invoke('restart_mitm_proxy');

    // Wait for server to restart
    await new Promise(resolve => setTimeout(resolve, 1500));
    showSettingsStatus(`MITM Proxy now running on port ${port}`, 'success', 'mitm-settings-status');
  } catch (error) {
    showSettingsStatus(`Failed: ${error}`, 'error', 'mitm-settings-status');
  } finally {
    saveBtn.disabled = false;
    saveBtn.textContent = 'Save';
  }
}

// ============ CA Certificate ============

// Load CA certificate info
async function loadCaCertInfo() {
  try {
    const exists = await invoke('ca_exists');
    const pathEl = document.getElementById('ca-cert-path');

    if (exists) {
      const path = await invoke('get_ca_cert_path');
      if (pathEl) {
        pathEl.textContent = path;
      }
    } else {
      if (pathEl) {
        pathEl.textContent = 'Certificate will be generated on first use';
      }
    }
  } catch (error) {
    console.error('Failed to load CA cert info:', error);
  }
}

// Copy CA path to clipboard
async function copyCaPath() {
  try {
    const path = await invoke('get_ca_cert_path');
    await navigator.clipboard.writeText(path);

    const btn = document.getElementById('copy-ca-path-btn');
    const originalText = btn.innerHTML;
    btn.innerHTML = '<i data-lucide="check" style="width: 14px; height: 14px;"></i> Copied!';
    lucide.createIcons();

    setTimeout(() => {
      btn.innerHTML = originalText;
      lucide.createIcons();
    }, 2000);
  } catch (error) {
    console.error('Failed to copy CA path:', error);
  }
}

// Open CA certificate for installation
async function installCaCert() {
  const btn = document.getElementById('install-ca-btn');
  const originalText = btn.innerHTML;

  try {
    btn.innerHTML = '<i data-lucide="loader" style="width: 14px; height: 14px;"></i> Opening...';
    lucide.createIcons();

    await invoke('open_ca_cert');

    btn.innerHTML = '<i data-lucide="check" style="width: 14px; height: 14px;"></i> Opened!';
    lucide.createIcons();

    setTimeout(() => {
      btn.innerHTML = originalText;
      lucide.createIcons();
    }, 2000);
  } catch (error) {
    alert(`Failed to open certificate: ${error}`);
    btn.innerHTML = originalText;
    lucide.createIcons();
  }
}

// ============ DLP Settings ============

// Load DLP settings
async function loadDlpSettings() {
  try {
    const settings = await invoke('get_dlp_settings');

    // Update built-in API keys checkbox
    const apiKeysCheckbox = document.getElementById('dlp-api-keys');
    if (apiKeysCheckbox) {
      apiKeysCheckbox.checked = settings.api_keys_enabled;
    }

    // Render custom patterns
    renderCustomPatterns(settings.custom_patterns);
  } catch (error) {
    console.error('Failed to load DLP settings:', error);
  }
}

// Render custom patterns list
function renderCustomPatterns(patterns) {
  const container = document.getElementById('custom-patterns');
  if (!container) return;

  if (patterns.length === 0) {
    container.innerHTML = '<p class="empty-text">No custom patterns added</p>';
    return;
  }

  container.innerHTML = patterns.map(pattern => `
    <div class="dlp-pattern-item" data-id="${pattern.id}">
      <input type="checkbox" class="dlp-checkbox dlp-custom-toggle" data-id="${pattern.id}" ${pattern.enabled ? 'checked' : ''} />
      <span class="dlp-pattern-name">${escapeHtml(pattern.name)}</span>
      <span class="dlp-pattern-badge ${pattern.pattern_type}">${pattern.pattern_type}</span>
      <button class="dlp-pattern-delete" data-id="${pattern.id}" title="Delete pattern">
        <i data-lucide="trash-2"></i>
      </button>
    </div>
  `).join('');

  // Re-initialize Lucide icons for new elements
  lucide.createIcons();

  // Add event listeners for toggles
  container.querySelectorAll('.dlp-custom-toggle').forEach(checkbox => {
    checkbox.addEventListener('change', async (e) => {
      e.stopPropagation();
      const id = parseInt(checkbox.dataset.id);
      try {
        await invoke('toggle_dlp_pattern', { id, enabled: checkbox.checked });
      } catch (error) {
        console.error('Failed to toggle pattern:', error);
        checkbox.checked = !checkbox.checked;
      }
    });
  });

  // Add event listeners for delete buttons
  container.querySelectorAll('.dlp-pattern-delete').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = parseInt(btn.dataset.id);
      if (confirm('Delete this pattern?')) {
        try {
          await invoke('delete_dlp_pattern', { id });
          loadDlpSettings();
        } catch (error) {
          console.error('Failed to delete pattern:', error);
        }
      }
    });
  });
}

// Show add pattern modal
function showAddPatternModal() {
  const modal = document.getElementById('add-pattern-modal');
  modal.classList.add('show');

  // Reset form
  document.getElementById('pattern-name').value = '';
  document.getElementById('pattern-values').value = '';
  document.querySelector('input[name="pattern-type"][value="keyword"]').checked = true;

  // Focus name input
  setTimeout(() => document.getElementById('pattern-name').focus(), 100);
}

// Hide add pattern modal
function hideAddPatternModal() {
  const modal = document.getElementById('add-pattern-modal');
  modal.classList.remove('show');
}

// Save new pattern
async function saveNewPattern() {
  const name = document.getElementById('pattern-name').value.trim();
  const patternType = document.querySelector('input[name="pattern-type"]:checked').value;
  const patternsText = document.getElementById('pattern-values').value;

  // Parse patterns (one per line, filter empty lines)
  const patterns = patternsText
    .split('\n')
    .map(p => p.trim())
    .filter(p => p.length > 0);

  if (!name) {
    alert('Please enter a name');
    return;
  }

  if (patterns.length === 0) {
    alert('Please enter at least one pattern');
    return;
  }

  const saveBtn = document.getElementById('save-pattern-btn');
  saveBtn.disabled = true;
  saveBtn.textContent = 'Saving...';

  try {
    await invoke('add_dlp_pattern', { name, patternType, patterns });
    hideAddPatternModal();
    loadDlpSettings();
  } catch (error) {
    alert(`Failed to save: ${error}`);
  } finally {
    saveBtn.disabled = false;
    saveBtn.textContent = 'Save';
  }
}

// Initialize DLP settings
function initDlpSettings() {
  // Built-in API keys toggle
  const apiKeysCheckbox = document.getElementById('dlp-api-keys');
  if (apiKeysCheckbox) {
    apiKeysCheckbox.addEventListener('change', async () => {
      try {
        await invoke('set_dlp_builtin', { key: 'api_keys', enabled: apiKeysCheckbox.checked });
      } catch (error) {
        console.error('Failed to update API keys setting:', error);
        apiKeysCheckbox.checked = !apiKeysCheckbox.checked;
      }
    });
  }

  // Add pattern button
  const addPatternBtn = document.getElementById('add-pattern-btn');
  if (addPatternBtn) {
    addPatternBtn.addEventListener('click', showAddPatternModal);
  }

  // Modal close buttons
  const closeModalBtn = document.getElementById('close-pattern-modal');
  const cancelBtn = document.getElementById('cancel-pattern-btn');
  if (closeModalBtn) closeModalBtn.addEventListener('click', hideAddPatternModal);
  if (cancelBtn) cancelBtn.addEventListener('click', hideAddPatternModal);

  // Modal save button
  const savePatternBtn = document.getElementById('save-pattern-btn');
  if (savePatternBtn) {
    savePatternBtn.addEventListener('click', saveNewPattern);
  }

  // Close modal on backdrop click
  const modal = document.getElementById('add-pattern-modal');
  if (modal) {
    modal.addEventListener('click', (e) => {
      if (e.target === modal) hideAddPatternModal();
    });
  }

  // Close modal on Escape key
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && modal?.classList.contains('show')) {
      hideAddPatternModal();
    }
  });

  // Load DLP settings
  loadDlpSettings();
}

// ============ Initialize Settings ============

export function initSettings() {
  // Reverse proxy port settings
  const saveBtn = document.getElementById('save-port-btn');
  const portInput = document.getElementById('port-input');

  if (saveBtn) {
    saveBtn.addEventListener('click', savePortSetting);
  }

  if (portInput) {
    portInput.addEventListener('keypress', (e) => {
      if (e.key === 'Enter') {
        savePortSetting();
      }
    });
  }

  // MITM proxy port settings
  const saveMitmBtn = document.getElementById('save-mitm-port-btn');
  const mitmPortInput = document.getElementById('mitm-port-input');

  if (saveMitmBtn) {
    saveMitmBtn.addEventListener('click', saveMitmPortSetting);
  }

  if (mitmPortInput) {
    mitmPortInput.addEventListener('keypress', (e) => {
      if (e.key === 'Enter') {
        saveMitmPortSetting();
      }
    });
  }

  // CA certificate buttons
  const installCaBtn = document.getElementById('install-ca-btn');
  const copyPathBtn = document.getElementById('copy-ca-path-btn');

  if (installCaBtn) {
    installCaBtn.addEventListener('click', installCaCert);
  }

  if (copyPathBtn) {
    copyPathBtn.addEventListener('click', copyCaPath);
  }

  // Load settings
  loadPortSetting();
  loadMitmPortSetting();
  loadCaCertInfo();

  // Initialize DLP settings
  initDlpSettings();
}
