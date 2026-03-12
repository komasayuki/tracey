import test from "node:test";
import assert from "node:assert/strict";

import {
  isOutlineCoverageComplete,
  isOutlineCoverageIncomplete,
  shouldShowOutlineCoverage,
} from "../src/outline-coverage.js";

test("non-requirement heading does not show outline coverage arc", () => {
  const entry = {
    coverage: { total: 0 },
    aggregated: { total: 1, implCount: 1, implTotal: 1, verifyCount: 1, verifyTotal: 1 },
  };

  assert.equal(shouldShowOutlineCoverage(entry), false);
  assert.equal(isOutlineCoverageComplete(entry, entry.aggregated), false);
  assert.equal(isOutlineCoverageIncomplete(entry, entry.aggregated), false);
});

test("requirement heading shows outline coverage state", () => {
  const entry = {
    coverage: { total: 1 },
    aggregated: { total: 1, implCount: 1, implTotal: 1, verifyCount: 0, verifyTotal: 1 },
  };

  assert.equal(shouldShowOutlineCoverage(entry), true);
  assert.equal(isOutlineCoverageComplete(entry, entry.aggregated), false);
  assert.equal(isOutlineCoverageIncomplete(entry, entry.aggregated), true);
});
