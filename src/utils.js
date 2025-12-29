// Tauri API
export const { invoke } = window.__TAURI__.core;

// ============ Shared State ============

// Store chart instances for cleanup
export let charts = {};

export function setCharts(newCharts) {
  charts = newCharts;
}

// Current time range filter
export let currentTimeRange = '1h';

export function setCurrentTimeRange(value) {
  currentTimeRange = value;
}

// Current backend filter
export let currentBackend = 'all';

export function setCurrentBackend(value) {
  currentBackend = value;
}

// Logs tab filters
export let logsTimeRange = '1h';

export function setLogsTimeRange(value) {
  logsTimeRange = value;
}

export let logsBackend = 'all';

export function setLogsBackend(value) {
  logsBackend = value;
}

// Store logs data for modal access
export let currentLogs = [];

export function setCurrentLogs(logs) {
  currentLogs = logs;
}

// Current proxy port
let currentPort = 8008;

export function getCurrentPort() {
  return currentPort;
}

export function setCurrentPort(port) {
  currentPort = port;
}

// ============ Color Palette ============

export const colors = {
  primary: '#6366f1',
  secondary: '#22c55e',
  warning: '#f59e0b',
  pink: '#ec4899',
  blue: '#3b82f6',
  purple: '#8b5cf6',
};

// ============ Utility Functions ============

// Format number with K/M suffix
export function formatNumber(num) {
  if (num >= 1000000) return (num / 1000000).toFixed(1) + 'M';
  if (num >= 1000) return (num / 1000).toFixed(1) + 'K';
  return num.toLocaleString();
}

// Format latency
export function formatLatency(ms) {
  if (ms >= 1000) return (ms / 1000).toFixed(2) + 's';
  return Math.round(ms) + 'ms';
}

// Shorten model name
export function shortenModel(model) {
  const match = model.match(/claude-(\w+)-(\d+-\d+)/);
  return match ? `${match[1]}-${match[2]}` : model;
}

// Escape HTML for safe display
export function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

// Format timestamp for display
export function formatTimestamp(ts) {
  const date = new Date(ts);
  return date.toLocaleString();
}

// ============ Tab Switching ============

export function initTabs() {
  const navItems = document.querySelectorAll('.nav-item');
  navItems.forEach(item => {
    item.addEventListener('click', () => {
      const tabId = item.dataset.tab;
      navItems.forEach(nav => nav.classList.remove('active'));
      item.classList.add('active');
      document.querySelectorAll('.tab-content').forEach(tab => tab.classList.remove('active'));
      document.getElementById(`${tabId}-tab`).classList.add('active');
    });
  });
}
