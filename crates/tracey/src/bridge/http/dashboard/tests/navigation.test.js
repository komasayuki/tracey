import test from "node:test";
import assert from "node:assert/strict";

import { resolveDashboardLink } from "../src/navigation.js";

function withWindow(location, effectivePathname, fn) {
  global.window = {
    location,
    __TRACEY_EFFECTIVE_PATHNAME__: effectivePathname ? () => effectivePathname : undefined,
  };
  try {
    fn();
  } finally {
    delete global.window;
  }
}

test("hash-only link stays on current logical spec page in static export", () => {
  withWindow(
    {
      href: "file:///tmp/site/index.html#/ignored",
      origin: "null",
      pathname: "/tmp/site/index.html",
      search: "",
    },
    "/mcp_gateway/rust/spec",
    () => {
      assert.deepEqual(resolveDashboardLink("#rreqgatewaycalltoolwithelicitation"), {
        kind: "same-document",
        target: "/mcp_gateway/rust/spec#rreqgatewaycalltoolwithelicitation",
      });
    },
  );
});

test("same-page absolute url with hash stays on current spec page", () => {
  withWindow(
    {
      href: "file:///tmp/site/index.html",
      origin: "null",
      pathname: "/tmp/site/index.html",
      search: "",
    },
    "/mcp_gateway/rust/spec",
    () => {
      assert.deepEqual(
        resolveDashboardLink("/mcp_gateway/rust/spec#rreqgatewaycalltoolwithelicitation"),
        {
          kind: "same-document",
          target: "/mcp_gateway/rust/spec#rreqgatewaycalltoolwithelicitation",
        },
      );
    },
  );
});

test("different same-origin page remains spa navigation", () => {
  withWindow(
    {
      href: "http://127.0.0.1:3000/mcp_gateway/rust/spec",
      origin: "http://127.0.0.1:3000",
      pathname: "/mcp_gateway/rust/spec",
      search: "",
    },
    null,
    () => {
      assert.deepEqual(resolveDashboardLink("/mcp_gateway/rust/sources/src/lib.rs:10"), {
        kind: "spa",
        target: "/mcp_gateway/rust/sources/src/lib.rs:10",
      });
    },
  );
});

test("external link is not intercepted", () => {
  withWindow(
    {
      href: "http://127.0.0.1:3000/mcp_gateway/rust/spec",
      origin: "http://127.0.0.1:3000",
      pathname: "/mcp_gateway/rust/spec",
      search: "",
    },
    null,
    () => {
      assert.equal(resolveDashboardLink("https://example.com/docs"), null);
    },
  );
});
