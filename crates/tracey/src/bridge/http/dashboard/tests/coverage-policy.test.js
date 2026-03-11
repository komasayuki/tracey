import test from "node:test";
import assert from "node:assert/strict";

import {
  ruleMissingImpl,
  ruleMissingVerify,
  summarizeRules,
} from "../src/coverage-policy.js";

function rule(id, implRefs = [], verifyRefs = []) {
  return {
    id: { base: id },
    implRefs,
    verifyRefs,
  };
}

test("verify-prefixed rule is excluded from impl-missing filter", () => {
  assert.equal(ruleMissingImpl(rule("verify.gateway.start")), false);
});

test("impl-prefixed rule is excluded from verify-missing filter", () => {
  assert.equal(ruleMissingVerify(rule("impl.gateway.start", [{ file: "src/lib.rs", line: 1 }])), false);
});

test("coverage summary uses separate impl and verify totals", () => {
  const stats = summarizeRules([
    rule("impl.gateway.start", [{ file: "src/lib.rs", line: 1 }]),
    rule("verify.gateway.start", [], [{ file: "src/lib.rs", line: 2 }]),
    rule("req.gateway.start", [{ file: "src/lib.rs", line: 3 }]),
  ]);

  assert.equal(stats.total, 3);
  assert.equal(stats.implTotal, 2);
  assert.equal(stats.verifyTotal, 2);
  assert.equal(stats.impl, 2);
  assert.equal(stats.verify, 1);
  assert.equal(stats.implPct, 100);
  assert.equal(stats.verifyPct, 50);
});
