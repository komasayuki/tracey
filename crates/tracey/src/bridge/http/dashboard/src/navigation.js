// SPA 遷移と同一ドキュメント内アンカー移動を切り分ける。

function effectivePathname() {
  if (typeof window.__TRACEY_EFFECTIVE_PATHNAME__ === "function") {
    return window.__TRACEY_EFFECTIVE_PATHNAME__();
  }
  return window.location.pathname;
}

export function resolveDashboardLink(href) {
  if (!href) return null;

  if (href.startsWith("#")) {
    return {
      kind: "same-document",
      target: `${effectivePathname()}${window.location.search}${href}`,
    };
  }

  let url;
  try {
    url = new URL(href, window.location.href);
  } catch {
    return null;
  }

  if (url.origin !== window.location.origin) {
    return null;
  }

  if (url.pathname === effectivePathname() && url.search === window.location.search && url.hash) {
    return {
      kind: "same-document",
      target: `${effectivePathname()}${window.location.search}${url.hash}`,
    };
  }

  return {
    kind: "spa",
    target: url.pathname + url.search + url.hash,
  };
}
