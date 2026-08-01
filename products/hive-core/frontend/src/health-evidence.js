export function observedValue(observation) {
  return observation?.state === "observed" ? observation.value : null;
}

export function observationReason(observation, fallback = "Not observed.") {
  return observation?.state === "observed" ? "" : observation?.reason || fallback;
}

export function observationLabel(observation) {
  switch (observation?.state) {
    case "observed":
      return "observed";
    case "failed":
      return "failed";
    case "not_observed":
      return "not observed";
    case "not_applicable":
      return "not applicable";
    default:
      return "unknown evidence state";
  }
}

export function healthEvidence(snapshot = {}) {
  const endpoint = observedValue(snapshot.health_endpoint);
  return {
    endpoint,
    startup: observedValue(snapshot.startup_checks),
    capabilities: observedValue(snapshot.capabilities),
    runs: observedValue(snapshot.runs),
    version: observedValue(snapshot.version),
    databaseOk: observedValue(snapshot.database_ok),
  };
}
