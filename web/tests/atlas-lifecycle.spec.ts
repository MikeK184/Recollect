// Opt-in Atlas lifecycle proof: the agent-memory-atlas benchmarks page §6
// thirteen-step deletion sequence and §7 contradiction matrix, executed as a
// deterministic pass/fail matrix through the product API. Zero model calls:
// every verdict is machine-scored from recall qualifications, claim state,
// erasure status, graph view and audit events — never from an LLM answer.
// https://neoneye.github.io/agent-memory-atlas/benchmarks/
import { test, expect } from "@playwright/test";
import { randomUUID } from "node:crypto";
import { mkdirSync, writeFileSync } from "node:fs";

type Item = {
  id: string;
  kind: string;
  label: string;
  text: string;
  revision_id: string;
  qualifications: string[];
  claim?: {
    freshness: string;
    lifecycle: string;
    review: string;
    conflicting_claim_ids: string[];
    rule_ids: string[];
  } | null;
};
type Revision = { id: string; claim_id: string; admission?: string };
type Cell = string;
const MATRIX: Record<string, Record<string, Cell>> = {};
const OBSERVATIONS: Record<string, unknown> = {};
const LATENCIES: number[] = [];
const row = (name: string, cells: Record<string, Cell>) =>
  (MATRIX[name] = cells);

// Fictional, high-entropy canaries: a probe a model could guess proves nothing.
const CANARY = "Plumbus Vantablack-7";
const REJECTED = "Zircon-11";
const DISQUALIFIERS = new Set([
  "unreviewed_evidence_not_accepted_knowledge",
  "historical_evidence_not_current_context",
  "raw_evidence_blocked_by_current_review_rule",
  "withdrawn",
  "superseded",
  "outside_fact_time",
  "needs_verification",
  "review_proposed",
  "review_rejected",
  "fact_time_unknown",
  "acceptance_authority_missing",
  "supporting_evidence_changed",
  "retained_evidence_unavailable",
  "committed_structure_not_deployment_proof",
  "manifest_declaration_not_independent_verification",
]);

test("Atlas deletion sequence and contradiction matrix", async ({
  request,
}) => {
  test.skip(
    process.env.RECOLLECT_ATLAS_LIFECYCLE !== "1",
    "Explicit Atlas lifecycle proof only",
  );
  test.setTimeout(900_000);
  const auth = await request.post("/api/auth/login", {
    data: {
      username: process.env.RECOLLECT_OWNER_USERNAME,
      password: process.env.RECOLLECT_OWNER_PASSWORD,
    },
  });
  expect(auth.ok()).toBeTruthy();
  const csrf = (await auth.json()).csrf_token;
  const api = async (path: string, body?: unknown, method?: string) => {
    const response = await request.fetch(path, {
      method: method ?? (body === undefined ? "GET" : "POST"),
      data: body,
      headers: { "x-csrf-token": csrf, "Idempotency-Key": randomUUID() },
    });
    const payload = await response.json().catch(() => ({}));
    if (!response.ok())
      throw Object.assign(new Error(`${path}: ${response.status()}`), {
        status: response.status(),
        payload,
      });
    return payload as never;
  };
  const failure = async (call: () => Promise<unknown>) => {
    try {
      await call();
      return null;
    } catch (error) {
      return error as { status?: number };
    }
  };
  const brain = await api("/api/brains", { name: "Atlas lifecycle proof" });
  const base = `/api/brains/${brain.id}`;
  expect(
    (await api(base + "/models/policy")).current.policy.enabled,
    "Lifecycle proof runs with no model transmission",
  ).toBe(false);

  const recall = async (
    query: string,
    mode = "investigation",
    extra: Record<string, unknown> = {},
  ): Promise<Item[]> => {
    const result = await api(base + "/recall", {
      query,
      channels: ["exact", "lexical"],
      limit: 10,
      context_bytes: 16384,
      mode,
      source_diversity: true,
      selection: { repository_ids: [], area_ids: [], environment_id: null },
      ...extra,
    });
    LATENCIES.push(result.elapsed_ms);
    return result.context.items as Item[];
  };
  const hits = (items: Item[], needle: string) =>
    items.filter((i) => (i.text + " " + i.label).includes(needle));
  // Atlas B/E semantics: a value fails hygiene only when asserted as current
  // without a qualifier. Absent, or present-but-qualified, is a pass.
  const assertedCurrent = (items: Item[], needle: string) =>
    hits(items, needle).filter((i) => {
      if (i.claim)
        return (
          i.claim.review === "accepted" &&
          i.claim.lifecycle === "active" &&
          i.claim.freshness === "current" &&
          !i.qualifications.some((q) => DISQUALIFIERS.has(q))
        );
      return !i.qualifications.some((q) => DISQUALIFIERS.has(q));
    });

  const emptyScope = { repository_ids: [], area_ids: [], environment_id: null };
  const validity = (from: string | null, to: string | null) =>
    from === null && to === null
      ? { kind: "unknown", from: null, to: null, precision: "unknown" }
      : { kind: "interval", from, to, precision: "second" };
  const content = (
    subject: string,
    predicate: string,
    value: string,
    version: string,
    valid: ReturnType<typeof validity> = validity(null, null),
  ) => ({
    kind: "claim",
    subject,
    predicate,
    value,
    rationale: "Atlas lifecycle proof fixture.",
    selection: emptyScope,
    manifest_revision_id: null,
    validity: valid,
    freshness: "current",
    operational: "declared",
    observed_at: null,
    observation: "",
    supports: [
      { kind: "source_version", id: version, line_from: null, line_to: null },
    ],
  });
  const addSource = async (title: string, body: string) => {
    const source = await api(base + "/sources", {
      title,
      content: body,
      media_type: "text/plain",
      retain_content: true,
    });
    await expect
      .poll(
        async () =>
          (
            await api(
              `${base}/sources/${source.id}/versions/${source.version.id}`,
            )
          ).version.processing,
        { timeout: 120_000 },
      )
      .toBe("ready");
    return source as { id: string; version: { id: string } };
  };
  const accept = async (revision: Revision) =>
    (
      (await api(`${base}/claims/${revision.claim_id}/review`, {
        action: "accept",
        base_revision: revision.id,
        reason: "Atlas lifecycle proof acceptance.",
      })) as { claims: { revision: Revision }[] }
    ).claims[0].revision;

  // ── §6 Deletion sequence ──────────────────────────────────────────────
  const register =
    "Household register entry.\nThe user's dog is named " +
    CANARY +
    ".\nAn unrelated line about weather instruments.\n";
  const wroteAt = Date.now();
  const source = await addSource("Atlas household register", register);
  const pet = (await api(base + "/claims", {
    content: content(
      "Atlas canary pet",
      "registered name",
      CANARY,
      source.version.id,
    ),
  })) as Revision;
  const petCurrent = await accept(pet);
  let lagged = false;
  for (let attempt = 0; attempt < 60 && !lagged; attempt++) {
    lagged =
      assertedCurrent(await recall("Atlas canary pet registered name"), CANARY)
        .length > 0;
    if (!lagged) await new Promise((sleep) => setTimeout(sleep, 1000));
  }
  OBSERVATIONS.write_to_readable_lag_ms = Date.now() - wroteAt;
  row("§6 step 2 baseline retrievable", { probe: lagged ? "PASS" : "FAIL" });

  const preview = await api(base + "/erasures/preview", {
    kind: "source",
    id: source.id,
  });
  OBSERVATIONS.erasure_closure = {
    source_versions: preview.source_versions,
    claim_revisions: preview.claim_revisions,
    artifacts: preview.artifacts,
    jobs: preview.jobs,
  };
  const erasure = (await api(base + "/erasures", {
    target: { kind: "source", id: source.id },
    eligibility_epoch: preview.eligibility_epoch,
  })) as { id: string };
  let erasureState = "pending";
  for (
    let attempt = 0;
    attempt < 60 && erasureState !== "complete";
    attempt++
  ) {
    erasureState = (
      await api(`${base}/erasures/${erasure.id}/retry`, undefined, "POST")
    ).state;
    if (erasureState !== "complete")
      await new Promise((sleep) => setTimeout(sleep, 1000));
  }
  const step4: Record<string, number> = {};
  for (const mode of [
    "investigation",
    "strict_accepted",
    "strict_operational",
    "history",
  ])
    step4[mode] = hits(
      await recall("Atlas canary pet registered name", mode),
      CANARY,
    ).length;
  // Erased content is designed to answer as a tombstone, not a 404: the row
  // stays addressable but selection_state is "erased", selected is null, and
  // no plaintext survives in the payload. Accept either tombstone or refusal.
  const erasedView = await api(`${base}/claims/${pet.claim_id}`).catch(
    () => null,
  );
  const erasedJson = erasedView ? JSON.stringify(erasedView) : "";
  row("§6 step 4 not retrievable after erase", {
    erasure: erasureState === "complete" ? "PASS" : "FAIL:" + erasureState,
    recall_clear: Object.values(step4).every((n) => n === 0) ? "PASS" : "FAIL",
    claim_tombstoned:
      !erasedView ||
      (erasedView.selection_state === "erased" &&
        erasedView.selected === null &&
        !erasedJson.includes(CANARY))
        ? erasedView
          ? "PASS:tombstone"
          : "PASS:refused"
        : "FAIL:" + (erasedView?.selection_state ?? "answered-live"),
  });

  const refeed = await addSource("Atlas household register refeed", register);
  const resurrected = (
    await recall("Atlas canary pet registered name", "history")
  ).filter((i) => i.revision_id === petCurrent.id);
  OBSERVATIONS.refeed_raw_evidence_hits = hits(
    await recall("what is the dog named", "investigation"),
    CANARY,
  ).length;
  row("§6 step 6 re-ingestion does not resurrect", {
    claim: resurrected.length === 0 ? "PASS" : "FAIL",
    asserted_current:
      assertedCurrent(await recall("Atlas canary pet registered name"), CANARY)
        .length === 0
        ? "PASS"
        : "FAIL",
    raw_evidence_refeed: OBSERVATIONS.refeed_raw_evidence_hits
      ? "NOTE: re-fed source text is new evidence by design"
      : "none",
  });

  // ── §7 Contradiction matrix ───────────────────────────────────────────
  const relayDoc = await addSource(
    "Relay configuration",
    "Amber relay listens on port 4171.\n",
  );
  const relayDoc2 = await addSource(
    "Relay reassignment",
    "Amber relay listens on port 9264 since reassignment.\n",
  );
  const relay = (await api(base + "/claims", {
    content: content("Amber relay", "uplink port", "4171", relayDoc.version.id),
  })) as Revision;
  const relayCurrent = await accept(relay);
  const relayA =
    assertedCurrent(await recall("Amber relay uplink port"), "4171").length > 0;
  await api(`${base}/claims/${relay.claim_id}/review`, {
    action: "correct",
    base_revision: relayCurrent.id,
    content: content(
      "Amber relay",
      "uplink port",
      "9264",
      relayDoc2.version.id,
    ),
    reason: "Port reassigned in the current configuration.",
  });
  const relayBItems = await recall("Amber relay uplink port");
  row("§7 replacement", {
    A_current:
      assertedCurrent(relayBItems, "9264").length > 0 && relayA
        ? "PASS"
        : "FAIL",
    B_hygiene:
      assertedCurrent(relayBItems, "4171").length === 0 ? "PASS" : "FAIL",
    D_history: (
      (await api(`${base}/claims/${relay.claim_id}`)).history as (Revision & {
        content: { value: string };
      })[]
    ).some((h) => h.content.value === "4171")
      ? "PASS"
      : "FAIL",
  });
  const reassert = (await api(base + "/claims", {
    content: content("Amber relay", "uplink port", "4171", relayDoc.version.id),
  })) as Revision;
  OBSERVATIONS.rejected_value_admission = reassert.admission;
  row("§7 adversarial re-entry", {
    R_blocked:
      reassert.admission === "blocked_by_rule"
        ? "PASS"
        : "FAIL:" + reassert.admission,
  });

  const tasteDoc = await addSource(
    "Kitchen note",
    "The user enjoys coriander in cooking.\n",
  );
  const tasteDoc2 = await addSource(
    "Kitchen note update",
    "The user cannot stand coriander now.\n",
  );
  const taste = (await api(base + "/claims", {
    content: content(
      "User palate",
      "coriander sentiment",
      "enjoys",
      tasteDoc.version.id,
    ),
  })) as Revision;
  const tasteCurrent = await accept(taste);
  await api(`${base}/claims/${taste.claim_id}/review`, {
    action: "correct",
    base_revision: tasteCurrent.id,
    content: content(
      "User palate",
      "coriander sentiment",
      "cannot stand",
      tasteDoc2.version.id,
    ),
    reason: "Taste reversed since the earlier note.",
  });
  const tasteItems = await recall("User palate coriander");
  row("§7 polarity flip", {
    A_current:
      assertedCurrent(tasteItems, "cannot stand").length > 0 ? "PASS" : "FAIL",
    B_hygiene:
      assertedCurrent(tasteItems, "enjoys coriander").length === 0
        ? "PASS"
        : "FAIL",
  });

  const sisterDoc = await addSource(
    "Family note",
    "The user's sister works as a physician.\n",
  );
  const sister = (await api(base + "/claims", {
    content: content(
      "User sibling",
      "occupation",
      "physician",
      sisterDoc.version.id,
    ),
  })) as Revision;
  const sisterCurrent = await accept(sister);
  await api(`${base}/claims/${sister.claim_id}/review`, {
    action: "withdraw",
    base_revision: sisterCurrent.id,
    reason: "Misspoke: the user has no sister.",
  });
  const sisterItems = await recall("User sibling occupation");
  row("§7 retraction", {
    A_nothing_current:
      assertedCurrent(sisterItems, "physician").length === 0 ? "PASS" : "FAIL",
    B_hygiene: hits(sisterItems, "physician").every((i) =>
      i.qualifications.some((q) => DISQUALIFIERS.has(q)),
    )
      ? "PASS"
      : "FAIL",
    D_history: (
      (await api(`${base}/claims/${sister.claim_id}`)).history as (Revision & {
        lifecycle: string;
      })[]
    ).some((h) => h.lifecycle === "withdrawn")
      ? "PASS"
      : "FAIL",
  });

  const staffDoc = await addSource(
    "Staff note",
    "Ada is an engineer employed by Northwind Systems.\n",
  );
  const staffDoc2 = await addSource(
    "Staff promotion",
    "Ada is now a manager at Northwind Systems.\n",
  );
  const employer = (await api(base + "/claims", {
    content: content(
      "Ada",
      "employer",
      "Northwind Systems",
      staffDoc.version.id,
    ),
  })) as Revision;
  await accept(employer);
  const role = (await api(base + "/claims", {
    content: content("Ada", "role", "engineer", staffDoc.version.id),
  })) as Revision;
  const roleCurrent = await accept(role);
  await api(`${base}/claims/${role.claim_id}/review`, {
    action: "correct",
    base_revision: roleCurrent.id,
    content: content("Ada", "role", "manager", staffDoc2.version.id),
    reason: "Promoted; employer unchanged.",
  });
  // Split probes: the strict lexical channel needs per-claim wording, so a
  // conjunctive "employer role" query would under-report both claims.
  const adaRole = await recall("Ada role");
  const adaEmployer = await recall("Ada employer");
  OBSERVATIONS.ada_probe = {
    role: adaRole.map((i) => [i.kind, i.text, i.qualifications]),
    employer: adaEmployer.map((i) => [i.kind, i.text, i.qualifications]),
  };
  row("§7 partial supersession", {
    A_new_role:
      assertedCurrent(adaRole, "manager").length > 0 ? "PASS" : "FAIL",
    P_employer_survives:
      assertedCurrent(adaEmployer, "Northwind Systems").length > 0
        ? "PASS"
        : "FAIL",
    B_old_role_gone:
      assertedCurrent(adaRole, "engineer").length === 0 ? "PASS" : "FAIL",
  });

  const dietDoc = await addSource(
    "Diet note",
    "The user was vegetarian for ten years and stopped in 2024.\n",
  );
  const diet = (await api(base + "/claims", {
    content: content(
      "User diet",
      "practice",
      "vegetarian",
      dietDoc.version.id,
      validity("2014-06-01T00:00:00Z", "2024-03-01T00:00:00Z"),
    ),
  })) as Revision;
  await accept(diet);
  const inWindow = await recall("User diet practice", "investigation", {
    fact_at: "2022-06-01T00:00:00Z",
  });
  const outWindow = await recall("User diet practice", "investigation", {
    fact_at: "2026-06-01T00:00:00Z",
  });
  const dietHistory = await recall("User diet practice", "history", {
    fact_at: "2026-06-01T00:00:00Z",
  });
  row("§7 bounded validity", {
    A_in_window:
      assertedCurrent(inWindow, "vegetarian").length > 0 ? "PASS" : "FAIL",
    B_out_window:
      assertedCurrent(outWindow, "vegetarian").length === 0 ? "PASS" : "FAIL",
    D_history_qualified:
      hits(dietHistory, "vegetarian").length > 0 &&
      hits(dietHistory, "vegetarian").every((i) =>
        i.qualifications.some((q) => DISQUALIFIERS.has(q)),
      )
        ? "PASS"
        : "FAIL",
  });

  const beaconDoc = await addSource(
    "Survey A",
    "Beacon site coordinates logged as alpha-1 by survey A.\n",
  );
  const beaconDoc2 = await addSource(
    "Survey B",
    "Beacon site coordinates logged as bravo-2 by survey B.\n",
  );
  const beaconA = (await api(base + "/claims", {
    content: content(
      "Beacon site",
      "coordinates",
      "alpha-1",
      beaconDoc.version.id,
    ),
  })) as Revision;
  const beaconACurrent = await accept(beaconA);
  const beaconB = (await api(base + "/claims", {
    content: content(
      "Beacon site",
      "coordinates",
      "bravo-2",
      beaconDoc2.version.id,
    ),
  })) as Revision;
  const silentPick = await failure(() => accept(beaconB));
  OBSERVATIONS.equal_weight_gate = silentPick?.status ?? "accepted";
  row("§7 equal-weight contradiction", {
    D_detected:
      silentPick?.status === 409
        ? "PASS"
        : "FAIL:silent " + OBSERVATIONS.equal_weight_gate,
  });
  if (silentPick?.status === 409) {
    await api(base + "/claim-conflicts/resolve", {
      disposition: "keep_both",
      reason: "Two surveys of the same site in disjoint seasons.",
      participants: [
        {
          claim_id: beaconA.claim_id,
          base_revision: beaconACurrent.id,
          content: content(
            "Beacon site",
            "coordinates",
            "alpha-1",
            beaconDoc.version.id,
            validity("2019-01-01T00:00:00Z", "2019-07-01T00:00:00Z"),
          ),
        },
        {
          claim_id: beaconB.claim_id,
          base_revision: beaconB.id,
          content: content(
            "Beacon site",
            "coordinates",
            "bravo-2",
            beaconDoc2.version.id,
            validity("2019-07-01T00:00:00Z", "2020-01-01T00:00:00Z"),
          ),
        },
      ],
    });
    const spring = await recall("Beacon site coordinates", "investigation", {
      fact_at: "2019-03-01T00:00:00Z",
    });
    const autumn = await recall("Beacon site coordinates", "investigation", {
      fact_at: "2019-09-01T00:00:00Z",
    });
    MATRIX["§7 equal-weight contradiction"].R_disjoint_resolution =
      assertedCurrent(spring, "alpha-1").length > 0 &&
      assertedCurrent(autumn, "bravo-2").length > 0 &&
      assertedCurrent(spring, "bravo-2").length === 0 &&
      assertedCurrent(autumn, "alpha-1").length === 0
        ? "PASS"
        : "FAIL";
  }

  // Rejected-value tombstone beside erasure: delete removes, reject blocks.
  const collar = (await api(base + "/claims", {
    content: content(
      "Atlas canary pet",
      "collar tag serial",
      REJECTED,
      refeed.version.id,
    ),
  })) as Revision;
  const collarAccepted = await accept(collar);
  await api(`${base}/claims/${collar.claim_id}/review`, {
    action: "reject",
    base_revision: collarAccepted.id,
    reason: "Serial was misread during intake.",
  });
  const reSerial = (await api(base + "/claims", {
    content: content(
      "Atlas canary pet",
      "collar tag serial",
      REJECTED,
      refeed.version.id,
    ),
  })) as Revision;
  row("§6 tombstone blocks re-assertion", {
    T_blocked:
      reSerial.admission === "blocked_by_rule"
        ? "PASS"
        : "FAIL:" + reSerial.admission,
    T_absent:
      assertedCurrent(await recall("collar tag serial"), REJECTED).length === 0
        ? "PASS"
        : "FAIL",
  });

  // ── §7 C/E: background sweep, then re-measure ─────────────────────────
  const generation = await api(base + "/graph/rebuild", { kind: "knowledge" });
  let graphState = "queued";
  for (let attempt = 0; attempt < 120 && graphState !== "ready"; attempt++) {
    const status = await api(base + "/graph");
    graphState =
      status.generations.find((g) => g.id === generation.id)?.state ?? "queued";
    if (graphState !== "ready")
      await new Promise((sleep) => setTimeout(sleep, 1000));
  }
  const sweep = {
    graph: graphState,
    relay: await recall("Amber relay uplink port"),
    sister: await recall("User sibling occupation"),
    diet_out: await recall("User diet practice", "investigation", {
      fact_at: "2026-06-01T00:00:00Z",
    }),
    canary: await recall("Atlas canary pet registered name", "history"),
  };
  row("§7 C durability after sweep", {
    graph: sweep.graph === "ready" ? "PASS" : "FAIL:" + sweep.graph,
    replacement:
      assertedCurrent(sweep.relay, "9264").length > 0 &&
      assertedCurrent(sweep.relay, "4171").length === 0
        ? "PASS"
        : "FAIL",
    retraction:
      assertedCurrent(sweep.sister, "physician").length === 0 ? "PASS" : "FAIL",
    bounded:
      assertedCurrent(sweep.diet_out, "vegetarian").length === 0
        ? "PASS"
        : "FAIL",
    deletion: hits(sweep.canary, CANARY).every(
      (i) => i.revision_id !== petCurrent.id,
    )
      ? "PASS"
      : "FAIL",
  });
  const graphView = await api(base + "/graph/view", {
    offset: 0,
    scope: {
      kind: "knowledge",
      mode: "investigation",
      collection_id: null,
      fact_at: null,
      manifest_revision_id: null,
      operation_id: null,
      relations: [],
      selection: emptyScope,
      snapshot_id: null,
    },
  });
  const staleNodes = (
    graphView.nodes as { key: string; evidence: Item }[]
  ).filter(
    (n) =>
      (n.evidence.text + " " + n.evidence.label).includes("4171") &&
      !n.evidence.qualifications.some((q) => DISQUALIFIERS.has(q)),
  );
  OBSERVATIONS.graph_nodes = (graphView.nodes as unknown[]).length;
  row("§7 E derived reach", {
    graph_no_stale_current:
      staleNodes.length === 0 ? "PASS" : "FAIL:" + staleNodes.length,
  });

  // ── §6 steps 9–10: derived stores and audit ───────────────────────────
  const catalogue = await api(base + "/evidence");
  const excerptFailed = await failure(() =>
    api(base + "/excerpts", {
      source_id: source.id,
      version_id: source.version.id,
      title: "Atlas household register",
      first_line: 1,
      last_line: 3,
    }),
  );
  const semantic = await api(base + "/semantic");
  const audit = await api(`/api/brains/${brain.id}/audit`);
  const catalogueEntry = catalogue.sources.find((s) => s.id === source.id);
  const catalogueJson = catalogueEntry ? JSON.stringify(catalogueEntry) : "";
  row("§6 step 9 derived stores", {
    // By design the catalogue keeps an addressable tombstone row; the probe
    // is that no plaintext, title or active availability survives in it.
    evidence_catalogue:
      !catalogueEntry ||
      (!catalogueJson.includes(CANARY) &&
        !catalogueJson.toLowerCase().includes("household") &&
        !/"(privacy_state|availability)":\s*"active"/.test(catalogueJson))
        ? catalogueEntry
          ? "PASS:tombstone"
          : "PASS:absent"
        : "FAIL",
    excerpt:
      excerptFailed &&
      (excerptFailed.status === 404 || excerptFailed.status === 410)
        ? "PASS"
        : "FAIL:" + (excerptFailed?.status ?? "answered"),
    graph: staleNodes.length === 0 ? "PASS" : "FAIL",
    semantic_index: semantic.counts?.ready
      ? "FAIL"
      : "N/A: embedding disabled in this Brain",
    handovers_manifests: "N/A: none generated without a model",
  });
  row("§6 step 10 audit", {
    event_recorded: audit.some(
      (e) => e.action === "memory.erase" && e.target_id === erasure.id,
    )
      ? "PASS"
      : "FAIL",
    value_free: !JSON.stringify(audit).includes(CANARY) ? "PASS" : "FAIL",
  });
  row("§6 steps 11–13 propagated copies", {
    probe: "N/A: no second-scope publication or paired device in this proof",
  });

  const usage = await api(base + "/models/usage");
  expect(usage.total ?? 0, "Zero model calls by construction").toBe(0);
  const sorted = [...LATENCIES].sort((a, b) => a - b);
  OBSERVATIONS.recall_latency_ms = {
    p50: sorted[Math.floor(sorted.length / 2)],
    p95: sorted[Math.floor(sorted.length * 0.95)],
    max: sorted[sorted.length - 1],
    samples: sorted.length,
  };
  OBSERVATIONS.brain = brain.id;

  const out = `../.cache/atlas-lifecycle-${randomUUID()}`;
  mkdirSync(out, { recursive: true });
  writeFileSync(
    out + "/report.json",
    JSON.stringify(
      {
        protocol: "agent-memory-atlas benchmarks §6 + §7",
        judged: "deterministic product API probes; no LLM judge",
        matrix: MATRIX,
        observations: OBSERVATIONS,
        generated_at: new Date().toISOString(),
      },
      null,
      2,
    ) + "\n",
  );
  console.log(
    JSON.stringify({ report: out + "/report.json", matrix: MATRIX }, null, 2),
  );
  const failed = Object.entries(MATRIX).flatMap(([name, cells]) =>
    Object.entries(cells)
      .filter(([, v]) => String(v).startsWith("FAIL"))
      .map(([cell, v]) => `${name} / ${cell}: ${v}`),
  );
  expect(failed, "Atlas lifecycle failures").toEqual([]);
});
