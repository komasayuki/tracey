// 直接要件を持つ見出しだけに outline の coverage arc を表示する。
export function shouldShowOutlineCoverage(entry) {
  return entry.coverage.total > 0;
}

export function isOutlineCoverageComplete(entry, aggregated) {
  return (
    shouldShowOutlineCoverage(entry) &&
    aggregated.implCount === aggregated.implTotal &&
    aggregated.verifyCount === aggregated.verifyTotal
  );
}

export function isOutlineCoverageIncomplete(entry, aggregated) {
  return shouldShowOutlineCoverage(entry) && !isOutlineCoverageComplete(entry, aggregated);
}
