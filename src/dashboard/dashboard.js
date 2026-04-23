'use strict';

function toast(msg, type) {
  type = type || 'success';
  var c = document.getElementById('toasts');
  var el = document.createElement('div');
  el.className = 'toast ' + type;
  el.textContent = msg;
  c.appendChild(el);
  setTimeout(function() { el.remove(); }, 3500);
}

function toggleDetail(el) {
  el.classList.toggle('open');
  el.nextElementSibling.classList.toggle('open');
}

async function fetchData() {
  try {
    const [statusRes, configRes] = await Promise.all([
      fetch('/api/status'),
      fetch('/api/config')
    ]);
    if (!statusRes.ok || !configRes.ok) {
      if (statusRes.status === 503 || configRes.status === 503) {
        setBanner('error', 'Service is not running', '');
      }
      return;
    }
    const status = await statusRes.json();
    const config = await configRes.json();
    render(status, config);
  } catch (e) {
    setBanner('error', 'Cannot reach service', '');
  }
}

function render(status, config) {
  document.getElementById('version').textContent = 'v' + (config.version || '?');

  var hasSuccess = !!status.last_update;
  var domains = config.domains || [];
  var updatedSet = {};
  if (status.updated_domains) {
    status.updated_domains.forEach(function(dd) { updatedSet[dd] = true; });
  }

  if (hasSuccess) {
    var d = new Date(status.last_update);
    var ago = timeAgo(d);
    var failed = domains.filter(function(dd) { return !updatedSet[dd]; });
    if (failed.length > 0) {
      setBanner('warn', 'Last update ' + ago + ' \u2014 ' + failed.length + ' domain(s) not updated', formatDate(d));
    } else {
      setBanner('ok', 'Service running \u2014 last update ' + ago, formatDate(d));
    }
  } else {
    setBanner('warn', 'Service running \u2014 no updates yet', '');
  }

  document.getElementById('interval').textContent = config.interval || '\u2014';
  document.getElementById('ipv6').textContent = config.ipv6 ? 'Enabled' : 'Disabled';
  document.getElementById('token').textContent = config.token_set ? '\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022' : 'Not set';

  document.getElementById('domainCount').textContent = domains.length + ' / 5';

  var list = document.getElementById('domainList');
  if (domains.length === 0) {
    list.innerHTML = '<span class="no-domains">No domains configured</span>';
  } else {
    list.innerHTML = domains.map(function(d) {
      var failed = hasSuccess && !updatedSet[d];
      return '<li' + (failed ? ' class="domain-failed"' : '') + '>' +
        escHtml(d) + '<span class="suffix">.duckdns.org</span>' +
        (failed ? '<span class="domain-error" title="Not in last successful update">\u26a0</span>' : '') +
        '</li>';
    }).join('');
  }
}

function setBanner(cls, text, time) {
  const b = document.getElementById('statusBanner');
  b.className = 'status-banner ' + cls;
  document.getElementById('statusText').innerHTML = '<strong>' + escHtml(text) + '</strong>';
  document.getElementById('statusTime').textContent = time;
}

async function forceUpdate() {
  const btn = document.getElementById('btnUpdate');
  btn.classList.add('loading');
  btn.disabled = true;
  try {
    const res = await fetch('/api/update', { method: 'POST' });
    const data = await res.json();
    if (data.ok) {
      toast('Update succeeded');
      fetchData();
    } else {
      toast(data.error || 'Update failed', 'error');
    }
  } catch (e) {
    toast('Request failed', 'error');
  } finally {
    btn.classList.remove('loading');
    btn.disabled = false;
  }
}

function escHtml(s) {
  const d = document.createElement('div');
  d.textContent = s;
  return d.innerHTML;
}

function formatDate(d) {
  return d.getFullYear() + '-' +
    String(d.getMonth() + 1).padStart(2, '0') + '-' +
    String(d.getDate()).padStart(2, '0') + ' ' +
    String(d.getHours()).padStart(2, '0') + ':' +
    String(d.getMinutes()).padStart(2, '0') + ':' +
    String(d.getSeconds()).padStart(2, '0');
}

function timeAgo(d) {
  const s = Math.floor((Date.now() - d.getTime()) / 1000);
  if (s < 60) return s + 's ago';
  if (s < 3600) return Math.floor(s / 60) + 'm ago';
  if (s < 86400) return Math.floor(s / 3600) + 'h ago';
  return Math.floor(s / 86400) + 'd ago';
}

fetchData();
setInterval(fetchData, 10000);
