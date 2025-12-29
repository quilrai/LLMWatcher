import { invoke, getCurrentPort } from './utils.js';

// Get instructions for each tool
function getToolInstructions(tool) {
  const port = getCurrentPort();

  const instructions = {
    'claude-code': {
      title: 'Claude Code CLI',
      content: `
        <p>Run Claude Code with the proxy inline:</p>
        <code>ANTHROPIC_BASE_URL="http://localhost:${port}/claude" claude</code>

        <p style="margin-top: 24px;"><strong>Or set globally:</strong></p>

        <div class="shell-tabs">
          <button class="shell-tab active" data-shell="bash">Bash</button>
          <button class="shell-tab" data-shell="zsh">Zsh</button>
          <button class="shell-tab" data-shell="fish">Fish</button>
        </div>

        <div class="shell-tab-content active" data-shell="bash">
          <p class="shell-config-path">~/.bashrc</p>
          <code>export ANTHROPIC_BASE_URL="http://localhost:${port}/claude"</code>
          <button class="btn btn-primary shell-action-btn" data-shell="bash" data-action="set">Set</button>
        </div>

        <div class="shell-tab-content" data-shell="zsh">
          <p class="shell-config-path">~/.zshrc</p>
          <code>export ANTHROPIC_BASE_URL="http://localhost:${port}/claude"</code>
          <button class="btn btn-primary shell-action-btn" data-shell="zsh" data-action="set">Set</button>
        </div>

        <div class="shell-tab-content" data-shell="fish">
          <p class="shell-config-path">Universal variable (persists automatically)</p>
          <code>set -Ux ANTHROPIC_BASE_URL "http://localhost:${port}/claude"</code>
          <button class="btn btn-primary shell-action-btn" data-shell="fish" data-action="set">Set</button>
        </div>

        <div id="shell-set-status" class="shell-set-status"></div>
      `
    },
    'gemini': {
      title: 'Gemini CLI',
      content: `
        <p>Gemini CLI support coming soon.</p>
        <p>Instructions for configuring Gemini CLI to use this proxy will be added in a future update.</p>
      `
    },
    'codex': {
      title: 'Codex CLI',
      content: `
        <p>Run Codex CLI with the proxy inline:</p>
        <code>OPENAI_BASE_URL="http://localhost:${port}/codex" codex</code>

        <p style="margin-top: 24px;"><strong>Or set globally:</strong></p>

        <div class="shell-tabs">
          <button class="shell-tab active" data-shell="bash">Bash</button>
          <button class="shell-tab" data-shell="zsh">Zsh</button>
          <button class="shell-tab" data-shell="fish">Fish</button>
        </div>

        <div class="shell-tab-content active" data-shell="bash">
          <p class="shell-config-path">~/.bashrc</p>
          <code>export OPENAI_BASE_URL="http://localhost:${port}/codex"</code>
          <button class="btn btn-primary shell-action-btn" data-shell="bash" data-action="set">Set</button>
        </div>

        <div class="shell-tab-content" data-shell="zsh">
          <p class="shell-config-path">~/.zshrc</p>
          <code>export OPENAI_BASE_URL="http://localhost:${port}/codex"</code>
          <button class="btn btn-primary shell-action-btn" data-shell="zsh" data-action="set">Set</button>
        </div>

        <div class="shell-tab-content" data-shell="fish">
          <p class="shell-config-path">Universal variable (persists automatically)</p>
          <code>set -Ux OPENAI_BASE_URL "http://localhost:${port}/codex"</code>
          <button class="btn btn-primary shell-action-btn" data-shell="fish" data-action="set">Set</button>
        </div>

        <div id="shell-set-status" class="shell-set-status"></div>
      `
    },
    'antigravity': {
      title: 'Antigravity',
      content: `
        <p>Antigravity support coming soon.</p>
        <p>Instructions for configuring Antigravity to use this proxy will be added in a future update.</p>
      `
    },
    'cursor': {
      title: 'Cursor',
      content: `
        <p>Cursor uses the MITM proxy since it doesn't support custom base URLs.</p>

        <h4 style="margin-top: 20px;">Step 1: Install CA Certificate</h4>
        <p>Go to <strong>Settings → MITM Proxy</strong> and copy the CA certificate path.</p>

        <div class="install-steps">
          <p><strong>macOS:</strong></p>
          <code>sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain ~/.config/quilr-agent-gateway/quilr_proxy_ca.crt</code>

          <p style="margin-top: 16px;"><strong>Linux:</strong></p>
          <code>sudo cp ~/.config/quilr-agent-gateway/quilr_proxy_ca.crt /usr/local/share/ca-certificates/ && sudo update-ca-certificates</code>
        </div>

        <h4 style="margin-top: 20px;">Step 2: Set HTTP Proxy</h4>
        <p>Set the HTTP_PROXY environment variable before launching Cursor:</p>
        <code>HTTP_PROXY=http://127.0.0.1:8888 HTTPS_PROXY=http://127.0.0.1:8888 /Applications/Cursor.app/Contents/MacOS/Cursor</code>

        <p style="margin-top: 16px;"><strong>Or create a launch script:</strong></p>
        <code>#!/bin/bash
export HTTP_PROXY=http://127.0.0.1:8888
export HTTPS_PROXY=http://127.0.0.1:8888
/Applications/Cursor.app/Contents/MacOS/Cursor</code>

        <p style="margin-top: 16px; opacity: 0.7;"><em>Note: The MITM proxy intercepts traffic to api.cursor.sh, api.anthropic.com, and api.openai.com.</em></p>
      `
    },
    'vscode': {
      title: 'VS Code',
      content: `
        <p>VS Code support coming soon.</p>
        <p>Instructions for configuring VS Code extensions to use this proxy will be added in a future update.</p>
      `
    }
  };

  return instructions[tool] || { title: 'Unknown', content: '<p>No instructions available.</p>' };
}

// Update a button to show Set or Remove
function updateButtonState(btn, isSet) {
  if (isSet) {
    btn.textContent = 'Remove';
    btn.dataset.action = 'remove';
    btn.classList.remove('btn-primary');
    btn.classList.add('btn-danger');
  } else {
    btn.textContent = 'Set';
    btn.dataset.action = 'set';
    btn.classList.remove('btn-danger');
    btn.classList.add('btn-primary');
  }
}

// Track currently active tool
let currentTool = 'claude-code';

// Check shell env status and update button states
async function updateShellButtonStates(tool) {
  const shells = ['bash', 'zsh', 'fish'];

  for (const shell of shells) {
    try {
      const isSet = await invoke('check_shell_env', { shell, tool });
      const btn = document.querySelector(`.shell-action-btn[data-shell="${shell}"]`);
      const tab = document.querySelector(`.shell-tab[data-shell="${shell}"]`);

      if (btn) {
        updateButtonState(btn, isSet);
      }

      // Add/remove indicator on tab
      if (tab) {
        if (isSet) {
          tab.classList.add('is-set');
        } else {
          tab.classList.remove('is-set');
        }
      }
    } catch (error) {
      // Shell might not be installed, leave button as "Set"
      console.log(`Could not check ${shell}: ${error}`);
    }
  }
}

// Handle shell action (set or remove)
async function handleShellAction(btn) {
  const shell = btn.dataset.shell;
  const action = btn.dataset.action;
  const statusDiv = document.getElementById('shell-set-status');
  const tool = currentTool;

  btn.disabled = true;
  btn.textContent = action === 'set' ? 'Setting...' : 'Removing...';

  try {
    let result;
    if (action === 'set') {
      result = await invoke('set_shell_env', { shell, tool });
    } else {
      result = await invoke('remove_shell_env', { shell, tool });
    }

    // Show success
    btn.textContent = 'Done!';
    btn.classList.remove('btn-primary', 'btn-danger');
    btn.classList.add('btn-success');

    if (statusDiv) {
      statusDiv.textContent = result;
      statusDiv.className = 'shell-set-status show success';
    }

    // Update button and tab state after success
    setTimeout(() => {
      btn.classList.remove('btn-success');
      btn.disabled = false;
      // Toggle the action
      const newIsSet = action === 'set';
      updateButtonState(btn, newIsSet);

      // Update tab indicator
      const tab = document.querySelector(`.shell-tab[data-shell="${shell}"]`);
      if (tab) {
        if (newIsSet) {
          tab.classList.add('is-set');
        } else {
          tab.classList.remove('is-set');
        }
      }
    }, 1500);
  } catch (error) {
    btn.textContent = 'Failed';
    btn.classList.remove('btn-primary', 'btn-danger');
    btn.classList.add('btn-error');

    if (statusDiv) {
      statusDiv.textContent = error;
      statusDiv.className = 'shell-set-status show error';
    }

    // Reset button after 3 seconds
    setTimeout(() => {
      btn.classList.remove('btn-error');
      btn.disabled = false;
      updateButtonState(btn, action === 'remove'); // Restore original state
    }, 3000);
  }
}

// Show instructions for selected tool
async function showToolInstructions(tool) {
  const instructionsDiv = document.getElementById('howto-instructions');
  const buttons = document.querySelectorAll('.howto-tool-btn');

  // Update active button
  buttons.forEach(btn => {
    if (btn.dataset.tool === tool) {
      btn.classList.add('active');
    } else {
      btn.classList.remove('active');
    }
  });

  // Show instructions
  const info = getToolInstructions(tool);
  instructionsDiv.innerHTML = `
    <h3>${info.title}</h3>
    ${info.content}
  `;

  // Add click handlers for shell action buttons
  instructionsDiv.querySelectorAll('.shell-action-btn').forEach(btn => {
    btn.addEventListener('click', () => handleShellAction(btn));
  });

  // Add click handlers for shell tabs
  instructionsDiv.querySelectorAll('.shell-tab').forEach(tab => {
    tab.addEventListener('click', () => {
      const shell = tab.dataset.shell;

      // Update active tab
      instructionsDiv.querySelectorAll('.shell-tab').forEach(t => t.classList.remove('active'));
      tab.classList.add('active');

      // Update active content
      instructionsDiv.querySelectorAll('.shell-tab-content').forEach(c => c.classList.remove('active'));
      instructionsDiv.querySelector(`.shell-tab-content[data-shell="${shell}"]`).classList.add('active');
    });
  });

  // Check and update button states for tools with shell buttons
  if (tool === 'claude-code' || tool === 'codex') {
    currentTool = tool;
    await updateShellButtonStates(tool);
  }
}

// Initialize How to use tab
export function initHowTo() {
  const buttons = document.querySelectorAll('.howto-tool-btn');
  buttons.forEach(btn => {
    btn.addEventListener('click', () => {
      showToolInstructions(btn.dataset.tool);
    });
  });
}
