function ruleId(rule) {
  if (typeof rule === "string") {
    return rule;
  }
  const { base, version } = rule.id;
  return version === 1 ? base : `${base}+${version}`;
}

export function ruleNeedsImpl(rule) {
  return !ruleId(rule).startsWith("verify.");
}

export function ruleNeedsVerify(rule) {
  return !ruleId(rule).startsWith("impl.");
}

export function ruleMissingImpl(rule) {
  return ruleNeedsImpl(rule) && rule.implRefs.length === 0;
}

export function ruleMissingVerify(rule) {
  return ruleNeedsVerify(rule) && rule.verifyRefs.length === 0;
}

export function coveragePercent(count, total) {
  return total === 0 ? 100 : Math.round((count / total) * 100);
}

export function summarizeRules(rules) {
  const implTotal = rules.filter((rule) => ruleNeedsImpl(rule)).length;
  const verifyTotal = rules.filter((rule) => ruleNeedsVerify(rule)).length;
  const impl = rules.filter((rule) => ruleNeedsImpl(rule) && rule.implRefs.length > 0).length;
  const verify = rules.filter(
    (rule) => ruleNeedsVerify(rule) && rule.verifyRefs.length > 0,
  ).length;

  return {
    total: rules.length,
    impl,
    implTotal,
    verify,
    verifyTotal,
    implPct: coveragePercent(impl, implTotal),
    verifyPct: coveragePercent(verify, verifyTotal),
  };
}
