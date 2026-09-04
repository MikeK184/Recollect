// Called only by test-ui.sh with Brain identities read from its owned database.
import { readFileSync } from "node:fs";
const database = new URL(process.env.DATABASE_URL).pathname;
if (!/^\/recollect_ui_[a-f0-9]{32}$/.test(database)) {
  throw new Error("Graph fixture cleanup requires an owned UI test database");
}
const brains = JSON.parse(readFileSync(0, "utf8"));
if (!Array.isArray(brains) || brains.some(id => typeof id !== "string" || !/^[a-f0-9-]{36}$/.test(id))) {
  throw new Error("Invalid fixture Brain identities");
}
for (const brain of brains) {
  const response = await fetch(`${process.env.NEO4J_URL.replace(/\/$/, "")}/db/neo4j/query/v2`, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      authorization: `Basic ${Buffer.from(`${process.env.NEO4J_USERNAME}:${process.env.NEO4J_PASSWORD}`).toString("base64")}`,
    },
    signal: AbortSignal.timeout(5000),
    body: JSON.stringify({
      statement: "MATCH (n) WHERE n.brain=$brain AND any(label IN labels(n) WHERE label IN ['RecollectGraphEntity','RecollectGraphGeneration','RecollectGraphBrain','RecollectGraphFence']) DETACH DELETE n RETURN count(n) AS count",
      parameters: { brain }, maxExecutionTime: 3,
    }),
  });
  const body = await response.json();
  if (!response.ok || body.errors?.length || body.data?.fields?.[0] !== "count") {
    throw new Error("Owned UI graph cleanup failed; preserve the fixture database");
  }
}
