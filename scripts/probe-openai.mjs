// Opt-in provider preflight. Only fixed synthetic text leaves this process.
// Run from the repo root: node --env-file=.env scripts/probe-openai.mjs
const key = process.env.OPENAI_API_KEY?.trim();
if (!key) {
  console.error("OPENAI_API_KEY is missing. Set it in the ignored .env file.");
  process.exit(1);
}

const endpoint = "https://api.openai.com/v1";
async function post(path, body) {
  const response = await fetch(`${endpoint}${path}`, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${key}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify(body),
    redirect: "error",
    signal: AbortSignal.timeout(30_000),
  });
  if (!response.ok) {
    // Provider error messages can quote credentials; never print their bodies.
    const body = await response.json().catch(() => null);
    const verification = /organization must be verified/i.test(
      body?.error?.message ?? "",
    );
    throw new Error(
      `OpenAI ${path} returned HTTP ${response.status}${verification ? " (organization verification required)" : ""}`,
    );
  }
  return response.json();
}

const probes = await Promise.allSettled([
  (async () => {
    const result = await post("/embeddings", {
      model: "text-embedding-3-large",
      input: ["Recollect synthetic connection test."],
      encoding_format: "float",
    });
    const vector = result.data?.[0]?.embedding;
    const valid =
      result.data?.length === 1 &&
      result.data[0].index === 0 &&
      Array.isArray(vector) &&
      vector.length === 3072 &&
      vector.every(Number.isFinite);
    if (!valid)
      throw new Error("Embedding probe returned an unexpected vector shape");
    return {
      purpose: "embedding",
      model: result.model,
      dimensions: vector.length,
      total_tokens: result.usage?.total_tokens,
      verified: true,
    };
  })(),
  (async () => {
    const result = await post("/responses", {
      model: "gpt-5.6-luna",
      store: false,
      reasoning: { effort: "none" },
      max_output_tokens: 512,
      input:
        "Extract the service and environment from this synthetic sentence: The service Amber runs in the test environment.",
      text: {
        format: {
          type: "json_schema",
          name: "synthetic_connection_probe",
          strict: true,
          schema: {
            type: "object",
            properties: {
              service: { type: "string" },
              environment: { type: "string" },
            },
            required: ["service", "environment"],
            additionalProperties: false,
          },
        },
      },
    });
    if (result.status !== "completed")
      throw new Error("Structured response probe did not complete");
    const output = result.output
      ?.flatMap((item) => item.content ?? [])
      .filter((item) => item.type === "output_text")
      .map((item) => item.text)
      .join("");
    let parsed;
    try {
      parsed = JSON.parse(output);
    } catch {
      throw new Error("Structured response probe did not return JSON");
    }
    if (
      parsed.service !== "Amber" ||
      parsed.environment !== "test" ||
      Object.keys(parsed).length !== 2
    )
      throw new Error(
        "Structured response probe did not extract the synthetic facts",
      );
    return {
      purpose: "structured_extraction",
      model: result.model,
      total_tokens: result.usage?.total_tokens,
      verified: true,
    };
  })(),
]);

const results = probes.map((probe, index) =>
  probe.status === "fulfilled"
    ? probe.value
    : {
        purpose: index === 0 ? "embedding" : "structured_extraction",
        verified: false,
        // Our explicit errors contain no payload; network errors stay generic.
        error:
          probe.reason instanceof Error &&
          /^(OpenAI |Embedding probe|Structured response probe)/.test(
            probe.reason.message,
          )
            ? probe.reason.message
            : "Provider connection failed or timed out",
      },
);
console.log(
  JSON.stringify(
    {
      observed_at: new Date().toISOString(),
      endpoint,
      synthetic_input_only: true,
      results,
    },
    null,
    2,
  ),
);
if (results.some((result) => !result.verified)) process.exitCode = 1;
