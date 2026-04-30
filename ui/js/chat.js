let currentAiMsg = null;
let currentToolCard = null;
let isStreaming = false;

function addUserMessage(text) {
  const list = document.getElementById('message-list');
  showEmptyState(false);
  const div = document.createElement('div');
  div.className = 'msg msg-user fade-up';
  div.innerHTML = `<div class="msg-text">${escapeHtml(text)}</div><div class="msg-meta">${formatTime(new Date())} ✓</div>`;
  list.appendChild(div);
  list.scrollTop = list.scrollHeight;
}

function startAiMessage() {
  const list = document.getElementById('message-list');
  showEmptyState(false);
  currentAiMsg = document.createElement('div');
  currentAiMsg.className = 'msg msg-ai fade-up';
  currentAiMsg.innerHTML = '<div class="msg-text"></div><div class="msg-meta"></div>';
  list.appendChild(currentAiMsg);
  list.scrollTop = list.scrollHeight;
}

function appendAiText(text) {
  if (!currentAiMsg) startAiMessage();
  const textDiv = currentAiMsg.querySelector('.msg-text');
  textDiv.innerHTML += renderMarkdown(text);
  const list = document.getElementById('message-list');
  list.scrollTop = list.scrollHeight;
}

function finishAiMessage() {
  if (!currentAiMsg) return;
  const meta = currentAiMsg.querySelector('.msg-meta');
  meta.innerHTML = `${formatTime(new Date())} <button onclick="copyMessage(this)" title="复制">📋</button>`;
  currentAiMsg = null;
}

function addToolCard(name) {
  const list = document.getElementById('message-list');
  showEmptyState(false);
  currentToolCard = document.createElement('div');
  currentToolCard.className = 'tool-card expanded';
  currentToolCard.innerHTML = `
    <div class="tool-card-header">
      <span class="tool-name">🔧 ${escapeHtml(name)}</span>
      <span class="tool-status running"><span class="spinner">⏳</span> 执行中...</span>
    </div>
    <div class="tool-body"></div>`;
  list.appendChild(currentToolCard);
  list.scrollTop = list.scrollHeight;

  currentToolCard.querySelector('.tool-card-header').addEventListener('click', () => {
    currentToolCard.classList.toggle('expanded');
  });
}

function finishToolCard(name, result, ok) {
  if (!currentToolCard) return;
  const status = currentToolCard.querySelector('.tool-status');
  if (ok) {
    status.className = 'tool-status ok';
    status.textContent = '✅ 完成';
  } else {
    status.className = 'tool-status err';
    status.textContent = '❌ 失败';
  }
  const body = currentToolCard.querySelector('.tool-body');
  body.innerHTML = `
    <div class="tool-section"><div class="tool-section-label">结果</div>
    <div class="tool-section-content">${escapeHtml(truncate(result, 2000))}</div></div>`;
  currentToolCard = null;
}

function addSystemMessage(text) {
  const list = document.getElementById('message-list');
  const div = document.createElement('div');
  div.className = 'msg-system';
  div.textContent = text;
  list.appendChild(div);
  list.scrollTop = list.scrollHeight;
}

function copyMessage(btn) {
  const msgDiv = btn.closest('.msg-ai');
  if (!msgDiv) return;
  const text = msgDiv.querySelector('.msg-text').textContent;
  navigator.clipboard.writeText(text).then(() => {
    btn.textContent = '✓';
    setTimeout(() => { btn.textContent = '📋'; }, 1500);
  });
}

function showTypingIndicator() {
  const list = document.getElementById('message-list');
  const ti = document.createElement('div');
  ti.className = 'msg msg-ai fade-up';
  ti.id = 'typing-indicator';
  ti.innerHTML = '<div class="typing-indicator"><span></span><span></span><span></span></div>';
  list.appendChild(ti);
  list.scrollTop = list.scrollHeight;
}

function removeTypingIndicator() {
  const ti = document.getElementById('typing-indicator');
  if (ti) ti.remove();
}
