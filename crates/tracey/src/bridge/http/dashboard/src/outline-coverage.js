// 直接要件を持つ見出しだけに outline の coverage arc を表示する。
export function shouldShowOutlineCoverage(entry) {
  return entry.coverage.total > 0;
}

// 要件 prefix に応じて、必要な側の arc だけを表示する。
export function shouldShowOutlineImplCoverage(entry) {
  return entry.coverage.implTotal > 0;
}

export function shouldShowOutlineVerifyCoverage(entry) {
  return entry.coverage.verifyTotal > 0;
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
