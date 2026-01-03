import { invoke, getCurrentPort, escapeHtml } from './utils.js';

// Store backends for editing
let customBackends = [];

// Parse settings JSON with defaults
function parseSettings(settingsJson) {
  try {
    const settings = JSON.parse(settingsJson || '{}');
    return {
      dlp_enabled: settings.dlp_enabled !== false, // default true
      rate_limit_requests: settings.rate_limit_requests || 0,
      rate_limit_minutes: settings.rate_limit_minutes || 1
    };
  } catch {
    return { dlp_enabled: true, rate_limit_requests: 0, rate_limit_minutes: 1 };
  }
}

// Build settings JSON from form values
function buildSettingsJson(dlpEnabled, rateRequests, rateMinutes) {
  return JSON.stringify({
    dlp_enabled: dlpEnabled,
    rate_limit_requests: rateRequests,
    rate_limit_minutes: rateMinutes
  });
}

// Show status message
function showBackendsStatus(message, type) {
  // Create or find status element
  let status = document.getElementById('backends-status');
  if (!status) {
    const cardBody = document.querySelector('#backends-tab .card-body');
    if (cardBody) {
      status = document.createElement('div');
      status.id = 'backends-status';
      status.className = 'settings-status';
      cardBody.insertBefore(status, cardBody.firstChild);
    }
  }

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

// Load custom backends from backend
export async function loadCustomBackends() {
  try {
    customBackends = await invoke('get_custom_backends');
    renderBackends(customBackends);
  } catch (error) {
    console.error('Failed to load custom backends:', error);
    const container = document.getElementById('backends-list');
    if (container) {
      container.innerHTML = '<p class="empty-text">Failed to load backends</p>';
    }
  }
}

// Render backends list
function renderBackends(backends) {
  const container = document.getElementById('backends-list');
  if (!container) return;

  const port = getCurrentPort();

  if (backends.length === 0) {
    container.innerHTML = `
      <div class="backends-empty">
        <p class="empty-text">No custom backends configured</p>
        <p class="empty-hint">Click "Add Backend" to add an OpenAI-compatible API endpoint.</p>
      </div>
    `;
    return;
  }

  container.innerHTML = backends.map(backend => {
    const settings = parseSettings(backend.settings);
    const dlpBadge = settings.dlp_enabled
      ? '<span class="backend-setting-badge dlp-on">DLP On</span>'
      : '<span class="backend-setting-badge dlp-off">DLP Off</span>';
    const rateBadge = settings.rate_limit_requests > 0
      ? `<span class="backend-setting-badge rate-limit">${settings.rate_limit_requests}/${settings.rate_limit_minutes}min</span>`
      : '<span class="backend-setting-badge no-rate-limit">No Rate Limit</span>';

    return `
    <div class="backend-item ${backend.enabled ? '' : 'disabled'}" data-id="${backend.id}">
      <div class="backend-info">
        <div class="backend-header">
          <input type="checkbox" class="dlp-checkbox backend-toggle" data-id="${backend.id}" ${backend.enabled ? 'checked' : ''} />
          <span class="backend-name">${escapeHtml(backend.name)}</span>
          <span class="backend-status ${backend.enabled ? 'enabled' : 'disabled'}">${backend.enabled ? 'Active' : 'Disabled'}</span>
        </div>
        <div class="backend-details">
          <div class="backend-url">
            <span class="backend-label">Proxy URL:</span>
            <code>http://localhost:${port}/${escapeHtml(backend.name)}</code>
          </div>
          <div class="backend-url">
            <span class="backend-label">Target:</span>
            <code>${escapeHtml(backend.base_url)}</code>
          </div>
        </div>
        <div class="backend-settings-summary">
          ${dlpBadge}
          ${rateBadge}
        </div>
      </div>
      <div class="backend-actions">
        <button class="dlp-pattern-edit backend-edit" data-id="${backend.id}" title="Edit backend">
          <i data-lucide="pencil"></i>
        </button>
        <button class="dlp-pattern-delete backend-delete" data-id="${backend.id}" title="Delete backend">
          <i data-lucide="trash-2"></i>
        </button>
      </div>
    </div>
  `;
  }).join('');

  // Re-initialize Lucide icons
  lucide.createIcons();

  // Add event listeners for toggles
  container.querySelectorAll('.backend-toggle').forEach(checkbox => {
    checkbox.addEventListener('change', async (e) => {
      e.stopPropagation();
      const id = parseInt(checkbox.dataset.id);
      try {
        await invoke('toggle_custom_backend', { id, enabled: checkbox.checked });
        showBackendsStatus('Backend updated. Restart proxy to apply changes.', 'info');
        loadCustomBackends();
      } catch (error) {
        console.error('Failed to toggle backend:', error);
        checkbox.checked = !checkbox.checked;
        showBackendsStatus(`Failed to toggle: ${error}`, 'error');
      }
    });
  });

  // Add event listeners for edit buttons
  container.querySelectorAll('.backend-edit').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const id = parseInt(btn.dataset.id);
      const backend = customBackends.find(b => b.id === id);
      if (backend) {
        showBackendModal(backend);
      }
    });
  });

  // Add event listeners for delete buttons
  container.querySelectorAll('.backend-delete').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = parseInt(btn.dataset.id);
      const backend = customBackends.find(b => b.id === id);
      if (confirm(`Delete backend "${backend?.name}"?`)) {
        try {
          await invoke('delete_custom_backend', { id });
          showBackendsStatus('Backend deleted. Restart proxy to apply changes.', 'success');
          loadCustomBackends();
        } catch (error) {
          showBackendsStatus(`Failed to delete: ${error}`, 'error');
        }
      }
    });
  });
}

// Show backend modal (add or edit)
function showBackendModal(backend = null) {
  const modal = document.getElementById('backend-modal');
  const title = document.getElementById('backend-modal-title');
  const nameInput = document.getElementById('backend-name');
  const urlInput = document.getElementById('backend-url');
  const dlpEnabledInput = document.getElementById('backend-dlp-enabled');
  const rateRequestsInput = document.getElementById('backend-rate-requests');
  const rateMinutesInput = document.getElementById('backend-rate-minutes');

  // Set title
  title.textContent = backend ? 'Edit Backend' : 'Add Backend';

  // Parse existing settings or use defaults
  const settings = backend ? parseSettings(backend.settings) : { dlp_enabled: true, rate_limit_requests: 0, rate_limit_minutes: 1 };

  // Reset/populate form
  document.getElementById('backend-id').value = backend ? backend.id : '';
  nameInput.value = backend ? backend.name : '';
  urlInput.value = backend ? backend.base_url : '';
  dlpEnabledInput.checked = settings.dlp_enabled;
  rateRequestsInput.value = settings.rate_limit_requests;
  rateMinutesInput.value = settings.rate_limit_minutes;

  // If editing, disable name field (changing name not allowed)
  nameInput.disabled = !!backend;

  modal.classList.add('show');

  // Focus appropriate input
  setTimeout(() => {
    if (backend) {
      urlInput.focus();
    } else {
      nameInput.focus();
    }
  }, 100);
}

// Hide backend modal
function hideBackendModal() {
  const modal = document.getElementById('backend-modal');
  modal.classList.remove('show');
  document.getElementById('backend-name').disabled = false;
}

// Save backend (add or update)
async function saveBackend() {
  const id = document.getElementById('backend-id').value;
  const name = document.getElementById('backend-name').value.trim();
  const baseUrl = document.getElementById('backend-url').value.trim();
  const dlpEnabled = document.getElementById('backend-dlp-enabled').checked;
  const rateRequests = parseInt(document.getElementById('backend-rate-requests').value) || 0;
  const rateMinutes = parseInt(document.getElementById('backend-rate-minutes').value) || 1;

  // Build settings JSON
  const settings = buildSettingsJson(dlpEnabled, rateRequests, Math.max(1, rateMinutes));

  // Validation
  if (!name) {
    alert('Please enter a backend name');
    return;
  }

  if (!baseUrl) {
    alert('Please enter a base URL');
    return;
  }

  const saveBtn = document.getElementById('save-backend-btn');
  saveBtn.disabled = true;
  saveBtn.textContent = 'Saving...';

  try {
    if (id) {
      // Update existing backend
      await invoke('update_custom_backend', {
        id: parseInt(id),
        name,
        baseUrl,
        settings
      });
      showBackendsStatus('Backend updated. Restart proxy to apply changes.', 'success');
    } else {
      // Add new backend
      await invoke('add_custom_backend', {
        name,
        baseUrl,
        settings
      });
      showBackendsStatus('Backend added. Restart proxy to apply changes.', 'success');
    }
    hideBackendModal();
    loadCustomBackends();
  } catch (error) {
    alert(`Failed to save: ${error}`);
  } finally {
    saveBtn.disabled = false;
    saveBtn.textContent = 'Save';
  }
}

// Initialize backends tab
export function initBackends() {
  // Add backend button
  const addBackendBtn = document.getElementById('add-backend-btn');
  if (addBackendBtn) {
    addBackendBtn.addEventListener('click', () => showBackendModal());
  }

  // Modal close buttons
  const closeModalBtn = document.getElementById('close-backend-modal');
  const cancelBtn = document.getElementById('cancel-backend-btn');
  if (closeModalBtn) closeModalBtn.addEventListener('click', hideBackendModal);
  if (cancelBtn) cancelBtn.addEventListener('click', hideBackendModal);

  // Modal save button
  const saveBackendBtn = document.getElementById('save-backend-btn');
  if (saveBackendBtn) {
    saveBackendBtn.addEventListener('click', saveBackend);
  }

  // Close modal on backdrop click
  const modal = document.getElementById('backend-modal');
  if (modal) {
    modal.addEventListener('click', (e) => {
      if (e.target === modal) hideBackendModal();
    });
  }

  // Close modal on Escape key
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && modal?.classList.contains('show')) {
      hideBackendModal();
    }
  });

  // Load backends when tab is clicked
  document.querySelector('[data-tab="backends"]')?.addEventListener('click', () => {
    loadCustomBackends();
  });

  // Initial load
  loadCustomBackends();
}
