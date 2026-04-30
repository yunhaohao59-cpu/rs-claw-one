document.addEventListener('DOMContentLoaded', () => {
  const input = document.getElementById('msg-input');
  const sendBtn = document.getElementById('btn-send');
  const titlebar = document.getElementById('titlebar');
  const settingsOverlay = document.getElementById('settings-overlay');

  initSidebar();

  input.addEventListener('input', () => {
    sendBtn.disabled = input.value.trim() === '';
    input.style.height = 'auto';
    input.style.height = Math.min(input.scrollHeight, 200) + 'px';
  });

  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      if (!sendBtn.disabled) sendMessage();
    }
  });

  sendBtn.addEventListener('click', () => { if (!sendBtn.disabled) sendMessage(); });

  document.getElementById('btn-settings').addEventListener('click', () => {
    loadConfigToForm();
    settingsOverlay.classList.remove('hidden');
  });
  document.getElementById('btn-settings-close').addEventListener('click', () => {
    settingsOverlay.classList.add('hidden');
  });
  settingsOverlay.addEventListener('click', (e) => {
    if (e.target === settingsOverlay) settingsOverlay.classList.add('hidden');
  });

  document.querySelectorAll('.settings-nav-item').forEach(btn => {
    btn.addEventListener('click', () => {
      document.querySelectorAll('.settings-nav-item').forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      const panel = btn.dataset.panel;
      document.querySelectorAll('.settings-panel').forEach(p => p.classList.add('hidden'));
      document.getElementById('panel-' + panel).classList.remove('hidden');
    });
  });

  // Titlebar window controls — using Tauri commands (v2 compatible)
  let isMaximized = false;
  document.getElementById('btn-minimize').addEventListener('click', async () => {
    try { if (window.__TAURI__) await window.__TAURI__.core.invoke('minimize_window'); } catch(e) {}
  });
  document.getElementById('btn-maximize').addEventListener('click', async () => {
    try {
      if (window.__TAURI__) {
        if (isMaximized) { await window.__TAURI__.core.invoke('unmaximize_window'); isMaximized = false; }
        else { await window.__TAURI__.core.invoke('maximize_window'); isMaximized = true; }
      }
    } catch(e) {}
  });
  document.getElementById('btn-close').addEventListener('click', async () => {
    try { if (window.__TAURI__) await window.__TAURI__.core.invoke('close_window'); } catch(e) {}
  });

  // Settings save
  document.querySelector('#panel-api .btn-primary').addEventListener('click', async (e) => {
    e.preventDefault();
    const provider = document.getElementById('cfg-provider').value.trim() || 'deepseek';
    const apiKey = document.getElementById('cfg-apikey').value.trim();
    const model = document.getElementById('cfg-model').value.trim() || 'deepseek-chat';
    const baseUrl = document.getElementById('cfg-baseurl').value.trim() || null;
    if (!apiKey) { alert('请输入 API Key'); return; }
    try {
      if (window.__TAURI__) {
        await window.__TAURI__.core.invoke('save_config', { provider, apiKey, model, baseUrl });
        settingsOverlay.classList.add('hidden');
        alert('配置已保存。请重启应用以生效。');
      }
    } catch (e) { alert('保存失败: ' + e); }
  });

  // Check if API key is set
  checkApiKey();

  showEmptyState(true);
});

async function checkApiKey() {
  try {
    if (window.__TAURI__) {
      const has = await window.__TAURI__.core.invoke('has_api_key');
      if (!has) {
        document.getElementById('message-list').innerHTML = `
          <div class="empty-state">
            <div class="empty-logo">🔑</div>
            <h2>需要 API Key</h2>
            <p class="empty-subtitle">点击左下角 ⚙ 设置，填写 DeepSeek API Key 后重启应用</p>
            <p class="empty-hint"><a href="https://platform.deepseek.com" target="_blank" style="color:var(--accent-primary)">获取 DeepSeek API Key →</a></p>
          </div>`;
      }
    }
  } catch(e) {}
}

async function loadConfigToForm() {
  try {
    if (window.__TAURI__) {
      const cfg = await window.__TAURI__.core.invoke('get_config');
      document.getElementById('cfg-provider').value = cfg.model?.provider || 'deepseek';
      document.getElementById('cfg-apikey').value = cfg.model?.api_key || '';
      document.getElementById('cfg-model').value = cfg.model?.model || 'deepseek-chat';
      document.getElementById('cfg-baseurl').value = cfg.model?.base_url || '';
    }
  } catch(e) { console.warn('Load config failed:', e); }
}

async function sendMessage() {
  const input = document.getElementById('msg-input');
  const text = input.value.trim();
  if (!text) return;

  input.value = '';
  input.style.height = 'auto';
  document.getElementById('btn-send').disabled = true;

  addUserMessage(text);
  showTypingIndicator();

  try {
    if (window.__TAURI__) {
      const { invoke } = window.__TAURI__.core;
      const { listen } = window.__TAURI__.event;

      const unlisten = await listen('chat-event', (event) => {
        removeTypingIndicator();
        const e = event.payload;
        switch (e.type) {
          case 'text_delta':
            appendAiText(e.text || '');
            break;
          case 'tool_call':
            addToolCard(e.tool_name || 'unknown');
            break;
          case 'tool_result':
            finishToolCard(e.tool_name || '', e.tool_result || '', true);
            break;
          case 'tool_error':
            finishToolCard(e.tool_name || '', e.tool_error || '', false);
            break;
          case 'done':
            finishAiMessage();
            unlisten();
            break;
          case 'error':
            addSystemMessage('❌ ' + (e.text || 'Unknown error'));
            finishAiMessage();
            unlisten();
            break;
        }
      });

      await invoke('send_message', { message: text });
    } else {
      setTimeout(() => {
        removeTypingIndicator();
        appendAiText('(GUI 模式：请通过 Tauri 运行以连接 AI 后端)');
        finishAiMessage();
      }, 500);
    }
  } catch (e) {
    removeTypingIndicator();
    addSystemMessage('❌ 发送失败: ' + e);
  }
}
