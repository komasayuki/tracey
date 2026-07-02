(function() {
  const core = window.__TRACEY_STATIC_CORE__;

  function jsonResponse(status, body) {
    return new Response(JSON.stringify(body), {
      status,
      headers: { "Content-Type": "application/json" },
    });
  }

  function normalizeApiPath(parsedUrl) {
    let pathname = parsedUrl.pathname;
    if (window.location.protocol === "file:") {
      // Windows file URL compatibility:
      // new URL("/api/config", "file:///C:/path/index.html")
      // resolves to file:///C:/api/config, whose pathname is /C:/api/config.
      pathname = pathname.replace(/^\/[A-Za-z]:\/api\//, "/api/");
    }
    return pathname;
  }

  if (window.__TRACEY_STATIC_TESTS__) {
    window.__TRACEY_STATIC_TESTS__.normalizeApiPath = normalizeApiPath;
  }

  window.fetch = async function(input, init) {
    const url = typeof input === "string" ? input : input.url;
    const parsed = new URL(url, window.location.href);
    const apiPath = normalizeApiPath(parsed);
    if (!apiPath.startsWith("/api/")) {
      return core.originalFetch(input, init);
    }

    const manifest = await core.loadManifest();
    const params = parsed.searchParams;
    const entry = await core.loadEntryFromParams(manifest, params);

    switch (apiPath) {
      case "/api/config":
        return jsonResponse(200, manifest.config || { specs: [] });
      case "/api/health":
        return jsonResponse(200, manifest.health || { configError: null });
      case "/api/version":
        return jsonResponse(200, { version: manifest.version || 1 });
      case "/api/forward":
        if (!entry) return jsonResponse(404, { error: "Spec/impl not found", code: "not_found" });
        return jsonResponse(200, { specs: [entry.forward] });
      case "/api/reverse":
        if (!entry) return jsonResponse(404, { error: "Spec/impl not found", code: "not_found" });
        return jsonResponse(200, entry.reverse);
      case "/api/spec":
        if (!entry) return jsonResponse(404, { error: "Spec not found", code: "not_found" });
        return jsonResponse(200, entry.spec_content);
      case "/api/file": {
        if (!entry) return jsonResponse(404, { error: "Spec/impl not found", code: "not_found" });
        const file = await core.loadFile(entry, params.get("path") || "");
        if (!file) return jsonResponse(404, { error: "File not found", code: "not_found" });
        return jsonResponse(200, file.data);
      }
      case "/api/search": {
        const limit = Number(params.get("limit") || "50");
        const query = params.get("q") || "";
        return jsonResponse(200, await core.search(manifest, query, Number.isFinite(limit) ? limit : 50));
      }
      default:
        return jsonResponse(501, { error: "Static mode: endpoint not available", code: "not_implemented" });
    }
  };

  class StaticWebSocket {
    constructor(url) {
      this.url = url;
      this.readyState = 1;
      this.onopen = null;
      this.onmessage = null;
      this.onerror = null;
      this.onclose = null;
      setTimeout(async () => {
        if (this.onopen) this.onopen({ type: "open" });
        const manifest = await core.loadManifest();
        if (this.onmessage) {
          this.onmessage({ data: JSON.stringify({ type: "version", version: manifest.version || 1 }) });
        }
      }, 0);
    }

    send(_) {}

    close() {
      this.readyState = 3;
      if (this.onclose) this.onclose({ type: "close" });
    }
  }

  window.WebSocket = StaticWebSocket;
})();
