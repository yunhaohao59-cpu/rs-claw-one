let sessions = [];

async function loadSessions() {
  try {
    if (window.__TAURI__) {
      const { invoke } = window.__TAURI__.core;
      const raw = await invoke('list_sessions');
      sessions = raw.map(([id, updated]) => ({ id, updated }));
    }
  } catch (e) { console.warn('Session load fallback:', e); }
  renderSessions();
}

function renderSessions() {
  const list = document.getElementById('session-list');
  if (!list) return;
  list.innerHTML = sessions.length === 0
    ? '<div class="sidebar-section-title" style="text-transform:none;opacity:0.5">暂无会话</div>'
    : sessions.map(s => {
        const active = s.id === (window._currentSession || '');
        return `<div class="session-item${active ? ' active' : ''}" data-id="${s.id}">
          <div class="sess-title">${s.id.slice(0, 8)}</div>
          <div class="sess-time">${s.updated || ''}</div>
        </div>`;
      }).join('');

  list.querySelectorAll('.session-item').forEach(el => {
    el.addEventListener('click', () => switchToSession(el.dataset.id));
  });
}

async function switchToSession(id) {
  window._currentSession = id;
  try {
    if (window.__TAURI__) {
      await window.__TAURI__.core.invoke('switch_session', { sessionId: id });
    }
  } catch (e) { console.warn('Switch session error:', e); }
  document.getElementById('chat-title').textContent = id.slice(0, 8);
  const msgList = document.getElementById('message-list');
  msgList.innerHTML = '';
  showEmptyState(false);
  renderSessions();
}

async function newSession() {
  try {
    if (window.__TAURI__) {
      const id = await window.__TAURI__.core.invoke('new_session');
      window._currentSession = id;
      document.getElementById('chat-title').textContent = '新对话';
      document.getElementById('message-list').innerHTML = '';
      showEmptyState(true);
      await loadSessions();
    } else {
      document.getElementById('message-list').innerHTML = '';
      showEmptyState(true);
    }
  } catch (e) { console.warn('New session error:', e); }
}

function showEmptyState(show) {
  const es = document.getElementById('empty-state');
  if (es) es.classList.toggle('hidden', !show);
}

async function initSidebar() {
  document.getElementById('btn-new-chat').addEventListener('click', newSession);
  await loadSessions();
  const firstId = sessions.length > 0 ? sessions[0].id : null;
  if (firstId) await switchToSession(firstId);
  else window._currentSession = null;
}
