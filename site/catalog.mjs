export const groups = [
  { text: 'Start here', items: [
    ['installation', 'Install Recollect', 'docs/runbooks/installation.md'],
    ['agent-memory-tools', 'Connect your agent', 'docs/runbooks/agent-memory-tools.md'],
    ['plugin', 'Plugin installation', 'plugins/recollect/README.md'],
    ['desktop-experience', 'Find your way around', 'docs/runbooks/desktop-experience.md'],
  ] },
  { text: 'Knowledge & memory', items: [
    ['evidence-collections', 'Sources & collections', 'docs/runbooks/evidence-collections.md'],
    ['workspace-scope', 'Repositories & environments', 'docs/runbooks/workspace-scope.md'],
    ['repository-publication', 'Publish repository evidence', 'docs/runbooks/repository-publication.md'],
    ['session-capture', 'Automatic session capture', 'docs/runbooks/session-capture.md'],
    ['provider-learning', 'AI policy & learning', 'docs/runbooks/provider-learning.md'],
    ['exact-lexical-recall', 'Recall with evidence', 'docs/runbooks/exact-lexical-recall.md'],
    ['semantic-recall', 'Semantic search', 'docs/runbooks/semantic-recall.md'],
    ['memory-investigation', 'Investigate knowledge', 'docs/runbooks/memory-investigation.md'],
    ['review-and-corrections', 'Corrections & review', 'docs/runbooks/review-and-corrections.md'],
    ['procedures-and-handovers', 'Procedures & handovers', 'docs/runbooks/procedures-and-handovers.md'],
  ] },
  { text: 'Tools & private execution', items: [
    ['mcp-catalogue', 'Connections & tool groups', 'docs/runbooks/mcp-catalogue.md'],
    ['mcp-runtime', 'Tool calls & outcomes', 'docs/runbooks/mcp-runtime.md'],
    ['mcp-vault-and-private-runners', 'Private runners & credentials', 'docs/runbooks/mcp-vault-and-private-runners.md'],
  ] },
  { text: 'Graphs', items: [
    ['graph-exploration', 'Explore graphs', 'docs/runbooks/graph-exploration.md'],
    ['graph-cross-repository-views', 'Cross-repository views', 'docs/runbooks/graph-cross-repository-views.md'],
    ['graph-recall', 'Graph recall', 'docs/runbooks/graph-recall.md'],
    ['graph-analytics', 'Graph analytics', 'docs/runbooks/graph-analytics.md'],
  ] },
  { text: 'Operate & develop', items: [
    ['team-access', 'People & access', 'docs/runbooks/team-access.md'],
    ['device-pairing', 'Pair a device', 'docs/runbooks/device-pairing.md'],
    ['retention-and-erasure', 'Privacy & erasure', 'docs/runbooks/retention-and-erasure.md'],
    ['recovery', 'Backup & recovery', 'docs/runbooks/recovery.md'],
    ['local-development', 'Local development', 'docs/runbooks/local-development.md'],
    ['public-memory-benchmark', 'Evaluation & limits', 'docs/runbooks/public-memory-benchmark.md'],
  ] },
];
export const pages = groups.flatMap(group => group.items.map(([slug, title, source]) => ({ slug, title, source })));
