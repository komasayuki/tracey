use eyre::{Result, eyre};

const INSERT_POINT: &str = "<script type=\"module\" crossorigin src=\"/assets/index.js\"></script>";

pub(super) fn inject_static_shim(
    index_html: &str,
    static_data_path: &str,
    static_data_js_path: &str,
    route_path: &str,
) -> Result<String> {
    let script = build_shim_script(static_data_path, route_path);
    let data_script = format!("<script src=\"{static_data_js_path}\"></script>\n");
    if let Some(pos) = index_html.find(INSERT_POINT) {
        let mut out =
            String::with_capacity(index_html.len() + data_script.len() + script.len() + 32);
        out.push_str(&index_html[..pos]);
        out.push_str(&data_script);
        out.push_str(&script);
        out.push_str(&index_html[pos..]);
        return Ok(out);
    }
    Err(eyre!(
        "Failed to inject static shim: module script tag not found"
    ))
}

fn build_shim_script(static_data_path: &str, route_path: &str) -> String {
    format!(
        r#"<script>
(function() {{
  const DATA_PATH = '{static_data_path}';
  const DATA_KEY = '__TRACEY_STATIC_SNAPSHOT__';
  const STATIC_ROUTE = '{route_path}';
  const originalFetch = window.fetch.bind(window);
  let loaded = null;

  function toRoutePath(url) {{
    if (!url) return STATIC_ROUTE || '/';
    try {{
      return new URL(url, 'https://tracey.local').pathname || '/';
    }} catch (_) {{
      return STATIC_ROUTE || '/';
    }}
  }}

  if (window.location.protocol === 'file:') {{
    window.__TRACEY_CURRENT_ROUTE__ = window.__TRACEY_CURRENT_ROUTE__ || STATIC_ROUTE || '/';
    const historyObj = window.history;
    const rawPushState = historyObj.pushState.bind(historyObj);
    const rawReplaceState = historyObj.replaceState.bind(historyObj);
    historyObj.pushState = function(state, title, url) {{
      window.__TRACEY_CURRENT_ROUTE__ = toRoutePath(url);
      try {{
        return rawPushState(state, title, url);
      }} catch (_) {{
        return null;
      }}
    }};
    historyObj.replaceState = function(state, title, url) {{
      window.__TRACEY_CURRENT_ROUTE__ = toRoutePath(url);
      try {{
        return rawReplaceState(state, title, url);
      }} catch (_) {{
        return null;
      }}
    }};
  }}

  window.__TRACEY_EFFECTIVE_PATHNAME__ = function() {{
    if (window.location.protocol !== 'file:') return window.location.pathname;
    return window.__TRACEY_CURRENT_ROUTE__ || STATIC_ROUTE || '/';
  }};

  function escapeHtml(value) {{
    return value
      .replaceAll('&', '&amp;')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;')
      .replaceAll('"', '&quot;');
  }}

  function markMatch(text, query) {{
    const lower = text.toLowerCase();
    const needle = query.toLowerCase();
    if (!needle) return escapeHtml(text);
    const idx = lower.indexOf(needle);
    if (idx < 0) return escapeHtml(text);
    const head = escapeHtml(text.slice(0, idx));
    const mid = escapeHtml(text.slice(idx, idx + query.length));
    const tail = escapeHtml(text.slice(idx + query.length));
    return `${{head}}<mark>${{mid}}</mark>${{tail}}`;
  }}

  function normalizeSnapshot(raw) {{
    const data = typeof raw === 'string' ? JSON.parse(raw) : raw;
    const byImpl = new Map();
    for (const entry of data.entries || []) {{
      byImpl.set(`${{entry.spec}}::${{entry.impl}}`, entry);
    }}
    return {{ ...data, byImpl }};
  }}

  async function loadSnapshot() {{
    if (loaded) return loaded;
    const inlined = window[DATA_KEY];
    if (inlined) {{
      loaded = Promise.resolve(normalizeSnapshot(inlined));
      return loaded;
    }}

    // フォールバック: http(s) 配信時にJSONを直接読む。
    loaded = originalFetch(DATA_PATH, {{ cache: 'no-store' }})
      .then((res) => res.json())
      .then((raw) => normalizeSnapshot(raw));
    return loaded;
  }}

  function pickImpl(snapshot, params) {{
    const config = snapshot.config || {{ specs: [] }};
    const spec = params.get('spec') || config.specs?.[0]?.name || '';
    let implName = params.get('impl');
    if (!implName) {{
      const specInfo = (config.specs || []).find((s) => s.name === spec);
      implName = specInfo?.implementations?.[0] || '';
    }}
    return {{ spec, implName }};
  }}

  function search(snapshot, query, limit) {{
    if (!query || query.length < 2) {{
      return {{ query, results: [], available: true }};
    }}

    const results = [];
    for (const rule of snapshot.search_rules || []) {{
      const haystack = `${{rule.id}}\n${{rule.raw}}`.toLowerCase();
      if (haystack.includes(query.toLowerCase())) {{
        results.push({{
          kind: 'rule',
          id: rule.id,
          line: 0,
          content: rule.raw,
          highlighted: markMatch(rule.raw, query),
          score: 1.0,
        }});
      }}
      if (results.length >= limit) break;
    }}

    if (results.length < limit) {{
      for (const source of snapshot.search_sources || []) {{
        const lines = source.content.split('\n');
        let matchedLine = null;
        for (let i = 0; i < lines.length; i++) {{
          if (lines[i].toLowerCase().includes(query.toLowerCase())) {{
            matchedLine = i + 1;
            break;
          }}
        }}
        if (!matchedLine) continue;

        const lineText = lines[matchedLine - 1] || '';
        results.push({{
          kind: 'source',
          id: source.path,
          line: matchedLine,
          content: lineText,
          highlighted: markMatch(lineText, query),
          score: 1.0,
        }});
        if (results.length >= limit) break;
      }}
    }}

    return {{ query, results, available: true }};
  }}

  function jsonResponse(status, body) {{
    return new Response(JSON.stringify(body), {{
      status,
      headers: {{ 'Content-Type': 'application/json' }},
    }});
  }}

  window.fetch = async function(input, init) {{
    const url = typeof input === 'string' ? input : input.url;
    const parsed = new URL(url, window.location.href);
    if (!parsed.pathname.startsWith('/api/')) {{
      return originalFetch(input, init);
    }}

    const snapshot = await loadSnapshot();
    const params = parsed.searchParams;
    const {{ spec, implName }} = pickImpl(snapshot, params);
    const entry = snapshot.byImpl.get(`${{spec}}::${{implName}}`);

    switch (parsed.pathname) {{
      case '/api/config':
        return jsonResponse(200, snapshot.config || {{ specs: [] }});
      case '/api/health':
        return jsonResponse(200, snapshot.health || {{ configError: null }});
      case '/api/version':
        return jsonResponse(200, {{ version: snapshot.version || 1 }});
      case '/api/forward':
        if (!entry) return jsonResponse(404, {{ error: 'Spec/impl not found', code: 'not_found' }});
        return jsonResponse(200, {{ specs: [entry.forward] }});
      case '/api/reverse':
        if (!entry) return jsonResponse(404, {{ error: 'Spec/impl not found', code: 'not_found' }});
        return jsonResponse(200, entry.reverse);
      case '/api/spec':
        if (!entry) return jsonResponse(404, {{ error: 'Spec not found', code: 'not_found' }});
        return jsonResponse(200, entry.spec_content);
      case '/api/file': {{
        if (!entry) return jsonResponse(404, {{ error: 'Spec/impl not found', code: 'not_found' }});
        const path = params.get('path') || '';
        const file = (entry.files || []).find((f) => f.path === path);
        if (!file) return jsonResponse(404, {{ error: 'File not found', code: 'not_found' }});
        return jsonResponse(200, file.data);
      }}
      case '/api/search': {{
        const q = params.get('q') || '';
        const limit = Number(params.get('limit') || '50');
        return jsonResponse(200, search(snapshot, q, Number.isFinite(limit) ? limit : 50));
      }}
      default:
        return jsonResponse(501, {{ error: 'Static mode: endpoint not available', code: 'not_implemented' }});
    }}
  }};

  class StaticWebSocket {{
    constructor(url) {{
      this.url = url;
      this.readyState = 1;
      this.onopen = null;
      this.onmessage = null;
      this.onerror = null;
      this.onclose = null;
      setTimeout(async () => {{
        if (this.onopen) this.onopen({{ type: 'open' }});
        const snapshot = await loadSnapshot();
        if (this.onmessage) {{
          this.onmessage({{ data: JSON.stringify({{ type: 'version', version: snapshot.version || 1 }}) }});
        }}
      }}, 0);
    }}

    send(_) {{}}

    close() {{
      this.readyState = 3;
      if (this.onclose) this.onclose({{ type: 'close' }});
    }}
  }}

  window.WebSocket = StaticWebSocket;
}})();
</script>
"#
    )
}
