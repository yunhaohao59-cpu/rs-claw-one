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

  // Titlebar window controls
  let isMaximized = false;
  document.getElementById('btn-minimize').addEventListener('click', async () => {
    if (window.__TAURI__) await window.__TAURI__.window.getCurrent().minimize();
  });
  document.getElementById('btn-maximize').addEventListener('click', async () => {
    if (window.__TAURI__) {
      const win = window.__TAURI__.window.getCurrent();
      if (isMaximized) { await win.unmaximize(); isMaximized = false; }
      else { await win.maximize(); isMaximized = true; }
    }
  });
  document.getElementById('btn-close').addEventListener('click', async () => {
    if (window.__TAURI__) await window.__TAURI__.window.getCurrent().close();
  });

  showEmptyState(true);
});

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
