let sessions = [];

async function loadSessions() {
  try {
    if (window.__TAURI__) {
      const raw = await window.__TAURI__.core.invoke('list_sessions');
      sessions = raw.map(([id, name, updated]) => ({ id, name, updated }));
    }
  } catch (e) { console.warn('Session load fallback:', e); }
  renderSessions();
}

function formatTime(isoStr) {
  if (!isoStr) return '';
  try {
    const d = new Date(isoStr.replace(' ', 'T'));
    if (isNaN(d.getTime())) return isoStr;
    const now = new Date();
    const diff = now - d;
    if (diff < 60000) return '刚刚';
    if (diff < 3600000) return Math.floor(diff / 60000) + ' 分钟前';
    if (diff < 86400000) return d.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' });
    if (diff < 172800000) return '昨天 ' + d.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' });
    return d.toLocaleDateString('zh-CN') + ' ' + d.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' });
  } catch { return isoStr; }
}

function sessionDisplayName(s) {
  if (s.name && s.name !== s.id && s.name.length > 20) return s.name.slice(0, 18) + '...';
  if (s.name && s.name !== s.id) return s.name;
  return '新对话';
}

function renderSessions() {
  const list = document.getElementById('session-list');
  if (!list) return;
  const cur = window._currentSession || '';
  list.innerHTML = sessions.length === 0
    ? '<div class="sidebar-section-title" style="text-transform:none;opacity:0.5">暂无会话</div>'
    : sessions.map(s => {
        const active = s.id === cur;
        const disp = sessionDisplayName(s);
        return `<div class="session-item${active ? ' active' : ''}" data-id="${s.id}" title="${s.name || s.id}">
          <div class="sess-title">${escapeHtml(disp)}</div>
          <div class="sess-time">${formatTime(s.updated)}</div>
          <div class="sess-actions">
            <button class="sess-act-btn" data-action="rename" data-id="${s.id}" title="重命名">✏</button>
            <button class="sess-act-btn sess-del-btn" data-action="delete" data-id="${s.id}" title="删除">✕</button>
          </div>
        </div>`;
      }).join('');

  list.querySelectorAll('.session-item').forEach(el => {
    el.addEventListener('click', (e) => {
      if (e.target.closest('.sess-act-btn')) return;
      switchToSession(el.dataset.id);
    });
  });

  list.querySelectorAll('.sess-act-btn[data-action="rename"]').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const sid = btn.dataset.id;
      const s = sessions.find(x => x.id === sid);
      const newName = prompt('重命名会话：', s ? sessionDisplayName(s) : '');
      if (newName && newName.trim()) {
        try {
          if (window.__TAURI__) await window.__TAURI__.core.invoke('rename_session', { sessionId: sid, name: newName.trim() });
          s.name = newName.trim();
          renderSessions();
        } catch (err) { console.warn('Rename failed:', err); }
      }
    });
  });

  list.querySelectorAll('.sess-act-btn[data-action="delete"]').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const sid = btn.dataset.id;
      if (!confirm('删除此会话？不可恢复。')) return;
      try {
        if (window.__TAURI__) await window.__TAURI__.core.invoke('delete_session', { sessionId: sid });
        sessions = sessions.filter(x => x.id !== sid);
        if (window._currentSession === sid) {
          window._currentSession = sessions.length > 0 ? sessions[0].id : null;
          if (window._currentSession) await switchToSession(window._currentSession);
          else { document.getElementById('message-list').innerHTML = ''; showEmptyState(true); }
        }
        renderSessions();
      } catch (err) { console.warn('Delete failed:', err); }
    });
  });
}

async function switchToSession(id) {
  window._currentSession = id;
  try {
    if (window.__TAURI__) await window.__TAURI__.core.invoke('switch_session', { sessionId: id });
  } catch (e) { console.warn('Switch session error:', e); }
  document.getElementById('chat-title').textContent = '加载中...';
  document.getElementById('message-list').innerHTML = '';
  showEmptyState(false);
  renderSessions();
  document.getElementById('chat-title').textContent = sessions.find(s => s.id === id) ? sessionDisplayName(sessions.find(s => s.id === id)) : id.slice(0, 8);
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
