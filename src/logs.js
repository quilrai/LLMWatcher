import {
  invoke,
  logsTimeRange,
  setLogsTimeRange,
  logsBackend,
  setLogsBackend,
  currentLogs,
  setCurrentLogs,
  formatNumber,
  formatLatency,
  formatTimestamp,
  shortenModel,
  escapeHtml
} from './utils.js';

// Render message logs table
function renderLogsTable(logs) {
  setCurrentLogs(logs);

  if (logs.length === 0) {
    return `
      <div class="empty-state">
        <h3>No logs yet</h3>
        <p>Make some API requests through the proxy to see logs here.</p>
      </div>
    `;
  }

  return `
    <div class="logs-table-container">
      <table class="logs-table">
        <thead>
          <tr>
            <th>Time</th>
            <th>Backend</th>
            <th>Model</th>
            <th>Tokens</th>
            <th>Latency</th>
            <th>Request</th>
            <th>Response</th>
          </tr>
        </thead>
        <tbody>
          ${logs.map((log, index) => `
            <tr>
              <td class="col-time">${formatTimestamp(log.timestamp)}</td>
              <td class="col-backend">${log.backend}</td>
              <td class="col-model">${shortenModel(log.model)}</td>
              <td class="col-tokens">${formatNumber(log.input_tokens)} / ${formatNumber(log.output_tokens)}</td>
              <td class="col-latency">${formatLatency(log.latency_ms)}</td>
              <td class="col-json">
                <button class="json-btn" data-index="${index}" data-type="request">View</button>
              </td>
              <td class="col-json">
                <button class="json-btn" data-index="${index}" data-type="response">View</button>
              </td>
            </tr>
          `).join('')}
        </tbody>
      </table>
    </div>
  `;
}

// Show JSON modal
function showJsonModal(title, jsonStr) {
  const existing = document.getElementById('json-modal');
  if (existing) existing.remove();

  let formatted = '';
  try {
    const parsed = JSON.parse(jsonStr);
    formatted = JSON.stringify(parsed, null, 2);
  } catch {
    formatted = jsonStr || 'null';
  }

  const modal = document.createElement('div');
  modal.id = 'json-modal';
  modal.className = 'json-modal';
  modal.innerHTML = `
    <div class="json-modal-content">
      <div class="json-modal-header">
        <h3>${title}</h3>
        <button class="json-modal-close">&times;</button>
      </div>
      <pre class="json-modal-body">${escapeHtml(formatted)}</pre>
    </div>
  `;
  document.body.appendChild(modal);
  modal.querySelector('.json-modal-close').addEventListener('click', () => modal.remove());
  modal.addEventListener('click', (e) => {
    if (e.target === modal) modal.remove();
  });
}

// Load message logs
export async function loadMessageLogs() {
  const content = document.getElementById('logs-content');
  content.innerHTML = '<p class="loading">Loading...</p>';

  try {
    const logs = await invoke('get_message_logs', { timeRange: logsTimeRange, backend: logsBackend });
    content.innerHTML = renderLogsTable(logs);

    // Add click handlers for JSON buttons
    content.querySelectorAll('.json-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        const index = parseInt(btn.dataset.index);
        const type = btn.dataset.type;
        const log = currentLogs[index];
        const title = type === 'request' ? `Request #${log.id}` : `Response #${log.id}`;
        const jsonStr = type === 'request' ? log.request_body : log.response_body;
        showJsonModal(title, jsonStr);
      });
    });
  } catch (error) {
    content.innerHTML = `
      <div class="empty-state">
        <h3>Error loading logs</h3>
        <p>${error}</p>
      </div>
    `;
  }
}

// Load backends for logs tab
export async function loadLogsBackends() {
  try {
    const backends = await invoke('get_backends');
    const select = document.getElementById('logs-backend-select');
    select.innerHTML = '<option value="all">All Backends</option>';
    backends.forEach(backend => {
      const option = document.createElement('option');
      option.value = backend;
      option.textContent = backend.charAt(0).toUpperCase() + backend.slice(1);
      select.appendChild(option);
    });
  } catch (error) {
    console.error('Failed to load backends:', error);
  }
}

// Initialize logs backend filter
export function initLogsBackendFilter() {
  const select = document.getElementById('logs-backend-select');
  select.addEventListener('change', () => {
    setLogsBackend(select.value);
    loadMessageLogs();
  });
}

// Initialize logs time filter
export function initLogsTimeFilter() {
  const select = document.getElementById('logs-time-select');
  select.addEventListener('change', () => {
    setLogsTimeRange(select.value);
    loadMessageLogs();
  });
}
