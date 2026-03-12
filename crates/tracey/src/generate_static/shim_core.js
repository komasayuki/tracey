(function() {
  const bootstrap = window.__TRACEY_STATIC_BOOTSTRAP__ || {};
  const MANIFEST_KEY = "__TRACEY_STATIC_MANIFEST__";
  const CHUNK_KEY = "__TRACEY_STATIC_CHUNKS__";
  const STATIC_ROUTE = bootstrap.routePath || "/";
  const STATIC_ROOT = bootstrap.staticRoot || "./tracey-static/";
  const originalFetch = window.fetch.bind(window);
  let loadedManifest = null;
  const loadedScripts = new Map();
  const loadedEntries = new Map();
  const loadedFiles = new Map();
  const STATIC_ROUTE_BODY_CLASSES = [
    "tracey-static-route-default",
    "tracey-static-route-spec",
    "tracey-static-route-coverage",
    "tracey-static-route-sources",
  ];

  function chunkStore() {
    if (!window[CHUNK_KEY]) window[CHUNK_KEY] = {};
    return window[CHUNK_KEY];
  }

  function routeBodyClass(route) {
    if ((route || "").endsWith("/sources")) return "tracey-static-route-sources";
    if ((route || "").endsWith("/coverage")) return "tracey-static-route-coverage";
    if ((route || "").endsWith("/spec")) return "tracey-static-route-spec";
    return "tracey-static-route-default";
  }

  function applyRouteBodyClass(route) {
    if (!document.body) return;
    for (const className of STATIC_ROUTE_BODY_CLASSES) {
      document.body.classList.remove(className);
    }
    document.body.classList.add(routeBodyClass(route));
  }

  function toRoutePath(url) {
    if (!url) return STATIC_ROUTE || "/";
    try {
      return new URL(url, "https://tracey.local").pathname || "/";
    } catch (_) {
      return STATIC_ROUTE || "/";
    }
  }

  function toBrowserHistoryUrl(url) {
    if (window.location.protocol !== "file:" || !url) return url;
    try {
      const parsed = new URL(url, "https://tracey.local");
      const nextRoute = parsed.pathname || "/";
      const currentRoute = window.__TRACEY_CURRENT_ROUTE__ || STATIC_ROUTE || "/";
      if (nextRoute === currentRoute) {
        return `${window.location.pathname}${parsed.search}${parsed.hash}`;
      }
    } catch (_) {
      return url;
    }
    return url;
  }

  function stateWithRoute(state, route) {
    return { ...(state || {}), __traceyRoute: route };
  }

  function resolveStaticUrl(relativePath) {
    return new URL(relativePath, new URL(STATIC_ROOT, window.location.href)).toString();
  }

  if (window.location.protocol === "file:") {
    window.__TRACEY_CURRENT_ROUTE__ = window.__TRACEY_CURRENT_ROUTE__ || STATIC_ROUTE || "/";
    applyRouteBodyClass(window.__TRACEY_CURRENT_ROUTE__);
    const historyObj = window.history;
    const rawPushState = historyObj.pushState.bind(historyObj);
    const rawReplaceState = historyObj.replaceState.bind(historyObj);

    historyObj.pushState = function(state, title, url) {
      const route = toRoutePath(url);
      window.__TRACEY_CURRENT_ROUTE__ = route;
      applyRouteBodyClass(route);
      try {
        return rawPushState(stateWithRoute(state, route), title, toBrowserHistoryUrl(url));
      } catch (_) {
        return null;
      }
    };

    historyObj.replaceState = function(state, title, url) {
      const route = toRoutePath(url);
      window.__TRACEY_CURRENT_ROUTE__ = route;
      applyRouteBodyClass(route);
      try {
        return rawReplaceState(stateWithRoute(state, route), title, toBrowserHistoryUrl(url));
      } catch (_) {
        return null;
      }
    };

    window.addEventListener("popstate", (event) => {
      const route = event.state?.__traceyRoute || window.__TRACEY_CURRENT_ROUTE__ || STATIC_ROUTE || "/";
      window.__TRACEY_CURRENT_ROUTE__ = route;
      applyRouteBodyClass(route);
    });
  }

  window.__TRACEY_EFFECTIVE_PATHNAME__ = function() {
    if (window.location.protocol !== "file:") return window.location.pathname;
    return window.__TRACEY_CURRENT_ROUTE__ || STATIC_ROUTE || "/";
  };

  function escapeHtml(value) {
    return value
      .replaceAll("&", "&amp;")
      .replaceAll("<", "&lt;")
      .replaceAll(">", "&gt;")
      .replaceAll('"', "&quot;");
  }

  function markMatch(text, query) {
    const lower = text.toLowerCase();
    const needle = query.toLowerCase();
    if (!needle) return escapeHtml(text);
    const idx = lower.indexOf(needle);
    if (idx < 0) return escapeHtml(text);
    const head = escapeHtml(text.slice(0, idx));
    const mid = escapeHtml(text.slice(idx, idx + query.length));
    const tail = escapeHtml(text.slice(idx + query.length));
    return `${head}<mark>${mid}</mark>${tail}`;
  }

  function ruleIdText(ruleId) {
    if (typeof ruleId === "string") return ruleId;
    if (!ruleId || typeof ruleId !== "object" || !ruleId.base) return "";
    return ruleId.version && ruleId.version > 1
      ? `${ruleId.base}+${ruleId.version}`
      : ruleId.base;
  }

  function normalizeManifest(raw) {
    const data = typeof raw === "string" ? JSON.parse(raw) : raw;
    const byImpl = new Map();
    for (const entry of data.entries || []) {
      byImpl.set(`${entry.spec}::${entry.impl}`, entry);
    }
    return { ...data, byImpl };
  }

  function normalizeEntry(raw) {
    const fileByPath = new Map();
    for (const file of raw.files || []) {
      fileByPath.set(file.path, file);
    }
    return { ...raw, fileByPath };
  }

  async function loadScriptChunk(relativePath) {
    const store = chunkStore();
    if (store[relativePath]) return store[relativePath];
    if (!loadedScripts.has(relativePath)) {
      loadedScripts.set(relativePath, new Promise((resolve, reject) => {
        const script = document.createElement("script");
        script.src = resolveStaticUrl(relativePath);
        script.async = true;
        script.onload = () => resolve(store[relativePath]);
        script.onerror = () => reject(new Error(`Failed to load static chunk: ${relativePath}`));
        document.head.appendChild(script);
      }));
    }
    return loadedScripts.get(relativePath);
  }

  async function loadManifest() {
    if (loadedManifest) return loadedManifest;
    const inlined = window[MANIFEST_KEY];
    if (!inlined) {
      loadedManifest = Promise.reject(new Error("Static manifest script was not loaded"));
      return loadedManifest;
    }
    loadedManifest = Promise.resolve(normalizeManifest(inlined));
    return loadedManifest;
  }

  function pickImpl(manifest, params) {
    const config = manifest.config || { specs: [] };
    const spec = params.get("spec") || config.specs?.[0]?.name || "";
    let implName = params.get("impl");
    if (!implName) {
      const specInfo = (config.specs || []).find((item) => item.name === spec);
      implName = specInfo?.implementations?.[0] || "";
    }
    return { spec, implName };
  }

  async function loadEntry(entryMeta) {
    if (!entryMeta) return null;
    const key = `${entryMeta.spec}::${entryMeta.impl}`;
    if (!loadedEntries.has(key)) {
      loadedEntries.set(key, loadScriptChunk(entryMeta.chunk).then((entry) => normalizeEntry(entry)));
    }
    return loadedEntries.get(key);
  }

  async function loadEntryFromParams(manifest, params) {
    const { spec, implName } = pickImpl(manifest, params);
    return loadEntry(manifest.byImpl.get(`${spec}::${implName}`));
  }

  async function loadFile(entry, path) {
    const fileRef = entry?.fileByPath?.get(path);
    if (!fileRef) return null;
    if (!loadedFiles.has(fileRef.chunk)) {
      loadedFiles.set(fileRef.chunk, loadScriptChunk(fileRef.chunk));
    }
    return loadedFiles.get(fileRef.chunk);
  }

  window.__TRACEY_STATIC_CORE__ = {
    originalFetch,
    loadManifest,
    loadEntryFromParams,
    loadFile,
    search: async function(manifest, query, limit) {
      if (!query || query.length < 2) {
        return { query, results: [], available: true };
      }

      const results = [];
      const needle = query.toLowerCase();
      for (const entryMeta of manifest.entries || []) {
        const entry = await loadEntry(entryMeta);
        for (const rule of entry?.forward?.rules || []) {
          const ruleId = ruleIdText(rule.id);
          const haystack = `${ruleId}\n${rule.raw}`.toLowerCase();
          if (!haystack.includes(needle)) continue;
          results.push({
            kind: "rule",
            id: ruleId,
            line: 0,
            content: rule.raw,
            highlighted: markMatch(rule.raw, query),
            score: 1.0,
          });
          if (results.length >= limit) return { query, results, available: true };
        }
      }

      for (const entryMeta of manifest.entries || []) {
        const entry = await loadEntry(entryMeta);
        for (const fileRef of entry?.files || []) {
          const file = await loadFile(entry, fileRef.path);
          const lines = file?.data?.content?.split("\n") || [];
          const matchedLine = lines.findIndex((line) => line.toLowerCase().includes(needle));
          if (matchedLine < 0) continue;
          const lineText = lines[matchedLine] || "";
          results.push({
            kind: "source",
            id: fileRef.path,
            line: matchedLine + 1,
            content: lineText,
            highlighted: markMatch(lineText, query),
            score: 1.0,
          });
          if (results.length >= limit) return { query, results, available: true };
        }
      }

      return { query, results, available: true };
    },
  };
})();
