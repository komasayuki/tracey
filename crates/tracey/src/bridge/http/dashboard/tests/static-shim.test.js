import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const shimApiPath = fileURLToPath(
  new URL("../../../../generate_static/shim_api.js", import.meta.url),
);
const shimApiSource = readFileSync(shimApiPath, "utf8");

function installShim(baseHref) {
  const location = new URL(baseHref);
  const originalFetchCalls = [];
  const testHooks = {};
  const window = {
    location: {
      href: baseHref,
      protocol: location.protocol,
      pathname: location.pathname,
    },
    __TRACEY_STATIC_TESTS__: testHooks,
    __TRACEY_STATIC_CORE__: {
      originalFetch: async (input, init) => {
        originalFetchCalls.push({ input, init });
        return new Response("fallback", { status: 299 });
      },
      loadManifest: async () => ({
        config: { specs: [] },
        health: { configError: null },
        version: 1,
        entries: [],
        byImpl: new Map(),
      }),
      loadEntryFromParams: async () => null,
      loadFile: async () => null,
      search: async () => ({ query: "", results: [], available: true }),
    },
  };

  vm.runInNewContext(
    shimApiSource,
    { window, Response, URL, setTimeout, clearTimeout },
    { filename: shimApiPath },
  );

  return { window, testHooks, originalFetchCalls };
}

function normalizeApiPath(baseHref, input) {
  const { window, testHooks } = installShim(baseHref);
  return testHooks.normalizeApiPath(new URL(input, window.location.href));
}

test("static shim normalizes API paths across file URL variants", () => {
  const cases = [
    {
      base: "file:///Users/foo/report/index.html",
      input: "/api/config",
      expected: "/api/config",
    },
    {
      base: "file:///C:/Users/foo/report/index.html",
      input: "/api/config",
      expected: "/api/config",
    },
    {
      base: "file:///D:/report/index.html",
      input: "/api/health",
      expected: "/api/health",
    },
    {
      base: "http://localhost:8000/index.html",
      input: "/api/config",
      expected: "/api/config",
    },
  ];

  for (const { base, input, expected } of cases) {
    assert.equal(normalizeApiPath(base, input), expected, `${base} ${input}`);
  }
});

test("static shim keeps non-api paths on the original fetch path", async () => {
  const { window, originalFetchCalls } = installShim(
    "file:///C:/Users/foo/report/index.html",
  );

  const response = await window.fetch("/assets/app.js");

  assert.equal(response.status, 299);
  assert.equal(originalFetchCalls.length, 1);
  assert.equal(originalFetchCalls[0].input, "/assets/app.js");
});

test("static shim serves Windows file API requests without fallback fetch", async () => {
  for (const { base, input } of [
    { base: "file:///C:/Users/foo/report/index.html", input: "/api/config" },
    { base: "file:///D:/report/index.html", input: "/api/health" },
  ]) {
    const { window, originalFetchCalls } = installShim(base);

    const response = await window.fetch(input);

    assert.equal(response.status, 200);
    assert.equal(originalFetchCalls.length, 0);
  }
});
