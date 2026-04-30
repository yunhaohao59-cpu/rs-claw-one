function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

function formatTime(date) {
  const h = String(date.getHours()).padStart(2, '0');
  const m = String(date.getMinutes()).padStart(2, '0');
  return `${h}:${m}`;
}

function truncate(text, len) {
  return text.length > len ? text.slice(0, len) + '...' : text;
}

function renderMarkdown(text) {
  if (typeof marked === 'undefined') return escapeHtml(text);
  marked.setOptions({ breaks: true, gfm: true });
  let html = marked.parse(text);
  if (typeof hljs !== 'undefined') {
    html = html.replace(/<pre><code class="language-(\w+)">([\s\S]*?)<\/code><\/pre>/g, (_, lang, code) => {
      try {
        const highlighted = hljs.highlight(code, { language: lang || 'plaintext' }).value;
        return `<pre data-lang="${lang}"><code class="hljs language-${lang}">${highlighted}</code></pre>`;
      } catch { return `<pre><code>${escapeHtml(code)}</code></pre>`; }
    });
  }
  return html;
}
