// The explanatory diagrams shared by the public site and the explainer
// artifact. Each story is complete at rest; the player adds motion. Names,
// counts and paths are example data, not a running installation.
import { glyph, icon, mark } from './icons';
import { storyControls, type Story } from './player';

export type StoryDefinition = Story & { markup: () => string };

const stateIcons = `<span class="rc-state" aria-hidden="true">${icon('clock', 14, 'rc-state-wait')}${icon('loader', 14, 'rc-state-run')}${icon('check', 14, 'rc-state-done')}${icon('x', 14, 'rc-state-fail')}</span>`;

function card(key: string, x: number, y: number, title: string, sub: string, glyphName: string, width = 150) {
  return `<g class="rc-node" data-key="${key}">
    <rect class="rc-node-box" x="${x}" y="${y}" width="${width}" height="64" rx="12"/>
    <rect class="rc-node-tile" x="${x + 12}" y="${y + 18}" width="28" height="28" rx="8"/>
    ${glyph(glyphName, x + 18, y + 24, 16, 'rc-node-glyph')}
    <text class="rc-node-title" x="${x + 50}" y="${y + 29}">${title}</text>
    <text class="rc-node-sub" x="${x + 50}" y="${y + 46}">${sub}</text>
  </g>`;
}

function brainRow(key: string, y: number, glyphName: string, label: string, value: string, slot?: string) {
  return `<g class="rc-row" data-key="${key}">
    <rect class="rc-row-bg" x="242" y="${y + 4}" width="140" height="48" rx="8"/>
    ${glyph(glyphName, 252, y + 20, 16, 'rc-row-glyph')}
    <text class="rc-row-label" x="276" y="${y + 33}">${label}</text>
    <text class="rc-row-value" x="372" y="${y + 33}" text-anchor="end"${slot ? ` data-slot="${slot}"` : ''}>${value}</text>
  </g>`;
}

const system: StoryDefinition = {
  id: 'system',
  label: 'How Recollect works',
  rest: 'Three coding hosts share one Brain. Memory flows in from sessions and back out as cited context; tools run where you placed them.',
  reset: { mem: '412' },
  final: { mem: '413' },
  steps: [
    { id: 'capture', label: 'Capture', duration: 2000, on: ['claude', 'a1'], packets: [{ path: 'a1', delay: 150 }], beats: [{ t: 1250, on: 'row-mem' }],
      caption: 'Claude Code finishes a fix. The plugin captures the session events your policy permits.' },
    { id: 'learn', label: 'Learn', duration: 2200, on: ['brain', 'row-mem'], beats: [{ t: 0, flag: 'learning' }, { t: 1400, unflag: 'learning' }, { t: 1400, slot: 'mem', text: '413' }, { t: 1400, flag: 'learned' }],
      caption: 'Recollect checks what the evidence supports and learns the decision, linked to its source.' },
    { id: 'recall', label: 'Recall', duration: 2100, on: ['codex', 'a0', 'row-mem'], packets: [{ path: 'a0', reverse: true, delay: 150 }],
      caption: 'Next morning, Codex receives that decision as cited context before its first prompt.' },
    { id: 'skills', label: 'Skills', duration: 2400, on: ['row-skills', 'a0', 'a1', 'a2'], packets: [{ path: 'a0', reverse: true, delay: 150 }, { path: 'a1', reverse: true, delay: 280 }, { path: 'a2', reverse: true, delay: 410 }], beats: [{ t: 1250, on: 'codex' }, { t: 1380, on: 'claude' }, { t: 1510, on: 'opencode' }],
      caption: 'A skill published once in the Brain syncs into every host’s native skills folder.' },
    { id: 'tools', label: 'Tools', duration: 4300, on: ['opencode', 'a2'], packets: [{ path: 'a2', delay: 100 }, { path: 't2', delay: 1350 }, { path: 't2', reverse: true, delay: 2450, duration: 900 }, { path: 'a2', reverse: true, delay: 3300, duration: 900 }], beats: [{ t: 1150, on: 'row-tools' }, { t: 1150, flag: 'checked' }, { t: 1350, on: 't2' }, { t: 2350, on: 'runner' }],
      caption: 'OpenCode calls an approved MCP tool. Recollect checks the Use grant and dispatches the call to a private runner inside your network.' },
  ],
  markup: () => `<div class="rc-story rc-system" data-story="system" data-step="rest">
    <div class="rc-canvas">
      <svg class="rc-drawing" viewBox="0 0 624 432" role="img" aria-labelledby="rc-system-title rc-system-desc">
        <title id="rc-system-title">Recollect connects coding agents to a shared Brain and to governed tools</title>
        <desc id="rc-system-desc">Codex, Claude Code and OpenCode connect to one Brain holding memory, skills and tools. The Brain dispatches approved tool calls to the Recollect service, the caller's device or a private runner inside your network.</desc>
        <rect class="rc-zone" x="445" y="316" width="177" height="96" rx="16"/>
        <text class="rc-zone-label" x="533" y="404" text-anchor="middle">YOUR NETWORK</text>
        <path class="rc-edge" data-path="a0" data-key="a0" d="M156 76 C195 76 195 176 234 176"/>
        <path class="rc-edge" data-path="a1" data-key="a1" d="M156 216 C195 216 195 216 234 216"/>
        <path class="rc-edge" data-path="a2" data-key="a2" d="M156 356 C195 356 195 256 234 256"/>
        <path class="rc-edge" data-path="t0" data-key="t0" d="M390 176 C421 176 421 76 452 76"/>
        <path class="rc-edge" data-path="t1" data-key="t1" d="M390 216 C421 216 421 216 452 216"/>
        <path class="rc-edge" data-path="t2" data-key="t2" d="M390 256 C421 256 421 356 452 356"/>
        ${card('codex', 6, 44, 'Codex', 'plugin', 'terminal')}
        ${card('claude', 6, 184, 'Claude Code', 'plugin', 'terminal')}
        ${card('opencode', 6, 324, 'OpenCode', 'direct MCP', 'terminal')}
        <g class="rc-node rc-brain" data-key="brain">
          <rect class="rc-node-box" x="234" y="96" width="156" height="240" rx="14"/>
          ${mark(248, 110, 28)}
          <text class="rc-node-title" x="286" y="123">Brain</text>
          <text class="rc-node-sub" x="286" y="140">platform-team</text>
          <line class="rc-divider" x1="234" x2="390" y1="154" y2="154"/>
          ${brainRow('row-mem', 158, 'notebook', 'Memory', '413', 'mem')}
          ${brainRow('row-skills', 214, 'file', 'Skills', '6')}
          ${brainRow('row-tools', 270, 'unplug', 'Tools', '9')}
          <g class="rc-when-learning">${glyph('sparkles', 331, 179, 14, 'rc-learning-glyph')}</g>
          <g class="rc-when-checked"><rect class="rc-mini-badge" x="313" y="289" width="44" height="18" rx="9"/><text class="rc-mini-badge-text" x="335" y="301.5" text-anchor="middle">Use ✓</text></g>
        </g>
        ${card('service', 452, 44, 'Recollect service', 'Context7 · Exa', 'server', 168)}
        ${card('device', 452, 184, 'Caller’s device', 'local stdio', 'laptop', 168)}
        ${card('runner', 452, 324, 'Private runner', 'office-ubuntu', 'shield', 168)}
      </svg>
    </div>
    ${storyControls(system)}
  </div>`,
};

function stage(key: string, label: string, title: string, body: string, meta: string) {
  return `<div class="rc-stage" data-key="${key}" role="listitem">
    <div class="rc-stage-head"><span class="rc-mono-label">${label}</span>${stateIcons}</div>
    <h3>${title}</h3>
    <p>${body}</p>
    <span class="rc-stage-meta">${meta}</span>
  </div>`;
}
const link = (key: string) => `<div class="rc-link" aria-hidden="true"><svg width="40" height="12" viewBox="0 0 40 12"><path class="rc-edge" data-path="${key}" data-key="${key}" d="M2 6 L38 6"/></svg></div>`;

const loop: StoryDefinition = {
  id: 'loop',
  label: 'The memory loop',
  rest: 'One loop runs on every prompt: capture, learn, recall, act. Every step keeps its evidence.',
  restDone: ['s1', 's2', 's3', 's4', 'l1', 'l2', 'l3', 'ctx1', 'ctx2', 'ctx3'],
  steps: [
    { id: 'capture', label: 'Capture', duration: 1800, on: ['s1'], done: ['s1'],
      caption: 'The session’s permitted events enter a durable queue, so nothing is lost when the network drops.' },
    { id: 'learn', label: 'Learn', duration: 2000, on: ['s2', 'l1'], done: ['s2', 'l1'], packets: [{ path: 'l1', duration: 600 }],
      caption: 'Recollect extracts candidates and checks each one against its source before it becomes usable memory.' },
    { id: 'recall', label: 'Recall', duration: 2600, on: ['s3', 'l2'], done: ['s3', 'l2'], packets: [{ path: 'l2', duration: 600 }], beats: [{ t: 700, done: 'ctx1' }, { t: 950, done: 'ctx2' }, { t: 1200, done: 'ctx3' }],
      caption: 'Before the next prompt, the plugin adds the memories that apply to this task, each with its citation.' },
    { id: 'act', label: 'Act', duration: 2200, on: ['s4', 'l3'], done: ['s4', 'l3'], packets: [{ path: 'l3', duration: 600 }],
      caption: 'The agent works with that context. Tool outcomes are captured as new evidence and the loop continues.' },
  ],
  markup: () => `<div class="rc-story rc-loop" data-story="loop" data-step="rest">
    <div class="rc-stages" role="list" aria-label="The memory loop">
      ${stage('s1', 'Capture', 'Sessions become evidence', 'The plugin captures permitted prompts, tool observations and imported documents through a durable delivery queue.', 'capture policy')}
      ${link('l1')}
      ${stage('s2', 'Learn', 'Evidence becomes memory', 'Claims, decisions, procedures and handovers pass a separate support check and keep their sources.', 'support-checked')}
      ${link('l2')}
      ${stage('s3', 'Recall', 'Memory returns as context', 'Exact, keyword, semantic and graph recall select what applies to the task’s scope, with citations.', '8 KiB per prompt')}
      ${link('l3')}
      ${stage('s4', 'Act', 'Agents act on it', 'Skills load when needed and approved MCP calls run where you placed them. Outcomes become new evidence.', 'governed tools')}
    </div>
    <div class="rc-terminal" role="img" aria-label="Example: a Claude Code prompt receives three cited memories from Recollect">
      <div class="rc-terminal-bar"><span>claude</span><span class="rc-terminal-path">~/platform</span><span class="rc-terminal-status">example data</span></div>
      <div class="rc-terminal-body">
        <div class="rc-term-line rc-term-prompt"><span class="rc-term-caret">›</span> Why does staging read secrets from a different Vault path?</div>
        <div class="rc-term-line rc-term-muted">recollect · context for this prompt · 3 memories · 2.1 KiB</div>
        <div class="rc-term-line rc-ctx" data-key="ctx1"><span class="rc-kind rc-kind-decision">decision</span><span class="rc-term-text">Staging reads kv/staging; production reads kv/prod</span><span class="rc-cite">infra/vault.tf@a91c2e3</span></div>
        <div class="rc-term-line rc-ctx" data-key="ctx2"><span class="rc-kind rc-kind-procedure">procedure</span><span class="rc-term-text">Rotate the staging AppRole secret</span><span class="rc-cite">runbooks/vault.md</span></div>
        <div class="rc-term-line rc-ctx" data-key="ctx3"><span class="rc-kind rc-kind-handover">handover</span><span class="rc-term-text">Vault migration: staging done, production pending</span><span class="rc-cite">session · 14 Oct</span></div>
      </div>
    </div>
    ${storyControls(loop)}
  </div>`,
};

function host(key: string, name: string, path: string, reload: string) {
  return `<div class="rc-host" data-key="${key}">
    <span class="rc-tile">${icon('terminal', 16)}</span>
    <div class="rc-host-copy"><strong>${name}</strong><code>${path}</code></div>
    <div class="rc-host-side"><span class="rc-host-state">${stateIcons}<span data-slot="${key}">r4 loaded</span></span><span class="rc-host-reload">${reload}</span></div>
  </div>`;
}
const fan = (prefix: string, width = 72) => `<svg class="rc-fan" width="${width}" height="228" viewBox="0 0 ${width} 228" aria-hidden="true">
  <path class="rc-edge" data-path="${prefix}0" data-key="${prefix}0" d="M0 114 C${width / 2} 114 ${width / 2} 34 ${width} 34"/>
  <path class="rc-edge" data-path="${prefix}1" data-key="${prefix}1" d="M0 114 C${width / 2} 114 ${width / 2} 114 ${width} 114"/>
  <path class="rc-edge" data-path="${prefix}2" data-key="${prefix}2" d="M0 114 C${width / 2} 114 ${width / 2} 194 ${width} 194"/>
</svg>`;

const skills: StoryDefinition = {
  id: 'skills',
  label: 'Dynamic skills',
  rest: 'Publish a skill once. The plugin keeps every connected host’s native skills in step with the Brain.',
  restDone: ['h0', 'h1', 'h2'],
  reset: { rev: 'r3', publish: 'Publish revision', h0: 'r3', h1: 'r3', h2: 'r3' },
  final: { rev: 'r4', publish: 'Published', h0: 'r4 loaded', h1: 'r4 loaded', h2: 'r4 loaded' },
  steps: [
    { id: 'publish', label: 'Publish', duration: 1900, on: ['card', 'publish'], beats: [{ t: 0, slot: 'publish', text: 'Publishing…' }, { t: 1000, slot: 'rev', text: 'r4' }, { t: 1000, slot: 'publish', text: 'Published' }, { t: 1000, off: 'publish' }],
      caption: 'A Brain manager publishes revision 4 in Agents → Skills. Bundles hold Markdown and text only: no scripts, no permission overrides.' },
    { id: 'sync', label: 'Sync', duration: 2600, on: ['f0', 'f1', 'f2'], packets: [{ path: 'f0', delay: 100, duration: 900 }, { path: 'f1', delay: 220, duration: 900 }, { path: 'f2', delay: 340, duration: 900 }],
      beats: [{ t: 150, on: 'h0' }, { t: 270, on: 'h1' }, { t: 390, on: 'h2' }, { t: 1050, off: 'h0', done: 'h0', slot: 'h0', text: 'r4 loaded' }, { t: 1170, off: 'h1', done: 'h1', slot: 'h1', text: 'r4 loaded' }, { t: 1290, off: 'h2', done: 'h2', slot: 'h2', text: 'r4 loaded' }],
      caption: 'At the next task boundary the plugin syncs the bundle into each host’s native skills folder and asks the host to reload.' },
    { id: 'activate', label: 'Activate', duration: 2600, beats: [{ t: 300, flag: 'fetch' }],
      caption: 'Hosts list only metadata until a skill applies. Then skills.fetch loads the body and any referenced file under the current scope.' },
  ],
  markup: () => `<div class="rc-story rc-skills" data-story="skills" data-step="rest">
    <div class="rc-flow">
      <div class="rc-skill-card" data-key="card">
        <div class="rc-skill-head"><span class="rc-tile">${icon('file', 16)}</span><div><strong>vault-inspection</strong><span>Agents → Skills · Brain-wide</span></div><span class="rc-rev" data-slot="rev">r4</span></div>
        <pre class="rc-skill-body"><span class="rc-fm">---</span>
<span class="rc-fm-key">name:</span> vault-inspection
<span class="rc-fm-key">description:</span> Inspect Vault configuration
  with attributable evidence.
<span class="rc-fm">---</span>
Read the repository instructions first.
Separate declared configuration from
observed runtime behavior.</pre>
        <div class="rc-skill-foot"><span class="rc-pill">skill · 2 files</span><span class="rc-button-sm" data-key="publish"><span data-slot="publish">Published</span></span></div>
      </div>
      ${fan('f')}
      <div class="rc-hosts">
        ${host('h0', 'Claude Code', '.claude/skills/vault-inspection', 'reload_skills')}
        ${host('h1', 'Codex', '.agents/skills/vault-inspection', 'file watch')}
        ${host('h2', 'OpenCode', '.opencode/skills/vault-inspection', 'ctx.skill.reload()')}
      </div>
    </div>
    <div class="rc-fetch-note">${icon('sparkles', 14)}<span><code>skills.list</code> returns metadata · <code>skills.fetch</code> loads SKILL.md and references/evidence.md on demand</span></div>
    ${storyControls(skills)}
  </div>`,
};

function check(key: string, title: string, sub: string, slot?: string) {
  return `<div class="rc-check" data-key="${key}">
    <div><strong>${title}</strong><span${slot ? ` data-slot="${slot}"` : ''}>${sub}</span></div>${stateIcons}
  </div>`;
}
function target(key: string, glyphName: string, title: string, sub: string) {
  return `<div class="rc-target" data-key="${key}"><span class="rc-tile">${icon(glyphName, 16)}</span><div><strong>${title}</strong><span>${sub}</span></div></div>`;
}

const tools: StoryDefinition = {
  id: 'tools',
  label: 'Governed MCP tools',
  rest: 'Every managed tool call passes the same checks, then runs on the executor you assigned to that connection.',
  restDone: ['k1', 'k2', 'k3'],
  reset: { tool: 'filesystem.read_file', caller: 'alex · OpenCode', result: 'waiting…', grant: 'Alex · Office files group' },
  final: { tool: 'filesystem.read_file', caller: 'alex · OpenCode', result: '3 files · captured', grant: 'Alex · Office files group' },
  steps: [
    { id: 'request', label: 'Request', duration: 1500, on: ['req', 'c0'], packets: [{ path: 'c0', delay: 100, duration: 800 }],
      caption: 'OpenCode asks Recollect to run filesystem.read_file in the Development environment.' },
    { id: 'check', label: 'Check', duration: 2000, on: ['panel'], beats: [{ t: 0, on: 'k1' }, { t: 500, off: 'k1', done: 'k1', on: 'k2' }, { t: 1000, off: 'k2', done: 'k2', on: 'k3' }, { t: 1500, off: 'k3', done: 'k3' }],
      caption: 'Recollect checks the caller’s Use grant, the environment scope and the approved tool definition.' },
    { id: 'dispatch', label: 'Dispatch', duration: 1900, on: ['g2', 'runner'], packets: [{ path: 'g2', delay: 100, duration: 1000 }],
      caption: 'This connection runs on a private runner. The runner connects outbound, claims the call and runs it inside your network.' },
    { id: 'result', label: 'Result', duration: 2200, on: ['g2', 'c0', 'req'], packets: [{ path: 'g2', reverse: true, duration: 800 }, { path: 'c0', reverse: true, delay: 800, duration: 600 }], beats: [{ t: 1400, slot: 'result', text: '3 files · captured' }],
      caption: 'The result returns through Recollect to the agent and is captured as evidence of what actually happened.' },
    { id: 'deny', label: 'Deny', duration: 3000, on: ['req', 'c0', 'panel'], packets: [{ path: 'c0', delay: 300, duration: 800 }],
      beats: [{ t: 0, slot: 'tool', text: 'exa.search' }, { t: 0, slot: 'caller', text: 'sam · Codex (reader)' }, { t: 0, slot: 'result', text: 'waiting…' }, { t: 0, slot: 'grant', text: 'No Use grant' }, { t: 1100, flag: 'denied' }, { t: 1700, slot: 'result', text: 'denied · nothing ran' }],
      caption: 'A reader asks for a tool without a Use grant. Brain access never implies tool use, so nothing runs.' },
  ],
  markup: () => `<div class="rc-story rc-tools" data-story="tools" data-step="rest">
    <div class="rc-flow">
      <div class="rc-request" data-key="req">
        <div class="rc-terminal-bar"><span>opencode</span><span class="rc-terminal-status">mcp.call</span></div>
        <dl class="rc-request-body">
          <div><dt>tool</dt><dd data-slot="tool">filesystem.read_file</dd></div>
          <div><dt>caller</dt><dd data-slot="caller">alex · OpenCode</dd></div>
          <div><dt>env</dt><dd>Development</dd></div>
          <div><dt>result</dt><dd class="rc-result" data-slot="result">3 files · captured</dd></div>
        </dl>
      </div>
      <svg class="rc-fan rc-fan-single" width="56" height="228" viewBox="0 0 56 228" aria-hidden="true"><path class="rc-edge" data-path="c0" data-key="c0" d="M0 114 C28 114 28 114 56 114"/></svg>
      <div class="rc-checks" data-key="panel">
        <span class="rc-mono-label">Recollect checks</span>
        ${check('k1', 'Use grant', 'Alex · Office files group', 'grant')}
        ${check('k2', 'Environment', 'Development')}
        ${check('k3', 'Approved tool', 'read_file · pinned schema')}
      </div>
      ${fan('g')}
      <div class="rc-targets">
        ${target('service', 'server', 'Recollect service', 'Context7 · Exa')}
        ${target('device', 'laptop', 'Caller’s device', 'local stdio')}
        ${target('runner', 'shield', 'Private runner', 'office-ubuntu · outbound only')}
      </div>
    </div>
    ${storyControls(tools)}
  </div>`,
};

function session(key: string, who: string, where: string, goal: string, files: [string, string], notice: string) {
  return `<div class="rc-session" data-key="${key}">
    <div class="rc-session-head"><span class="rc-tile">${icon('terminal', 16)}</span><div><strong>${who}</strong><span>${where}</span></div></div>
    <p class="rc-session-goal"><span class="rc-mono-label">Goal</span>${goal}</p>
    <ul class="rc-session-files"><li class="rc-file-overlap" data-key="f-${key}"><code>${files[0]}</code></li><li><code>${files[1]}</code></li></ul>
    <p class="rc-session-note" data-key="${key}-note">${notice}</p>
  </div>`;
}

const coordination: StoryDefinition = {
  id: 'coordination',
  label: 'Peer coordination',
  rest: 'Sessions on different machines share their scope and planned files through the Brain, so overlapping work surfaces before it collides.',
  restDone: ['f-mike', 'f-paul', 'mike-note', 'paul-note', 'overlap'],
  steps: [
    { id: 'announce', label: 'Announce', duration: 2200, on: ['mike', 'paul', 'p0', 'p1'], packets: [{ path: 'p0', delay: 100 }, { path: 'p1', reverse: true, delay: 250 }],
      caption: 'Each session announces its goal and scope. Edit hooks add the repository paths it plans to change; no file contents are uploaded.' },
    { id: 'overlap', label: 'Overlap', duration: 2200, on: ['hub'], beats: [{ t: 300, done: 'f-mike' }, { t: 300, done: 'f-paul' }, { t: 600, done: 'overlap' }],
      caption: 'The Brain finds both sessions planning infra/vault.tf in the same registered repository and explains the overlap.' },
    { id: 'notify', label: 'Notify', duration: 2300, on: ['p1', 'paul'], packets: [{ path: 'p1', delay: 100 }], beats: [{ t: 1150, done: 'paul-note' }],
      caption: 'At Paul’s next prompt, a hook shows an attributed notice naming the account, coding host, device and environment.' },
    { id: 'agree', label: 'Agree', duration: 2800, on: ['p0', 'p1', 'mike'], packets: [{ path: 'p1', reverse: true, delay: 100 }, { path: 'p0', reverse: true, delay: 1100 }], beats: [{ t: 2100, done: 'mike-note' }],
      caption: 'Paul proposes to take production only and Mike accepts. Agreements are advisory: they grant no tool or file-write permission.' },
  ],
  markup: () => `<div class="rc-story rc-coordination" data-story="coordination" data-step="rest">
    <div class="rc-flow">
      ${session('mike', 'Mike · Claude Code', 'macOS · Development', 'Rotate the staging AppRole', ['infra/vault.tf', 'runbooks/vault.md'], 'Agreed: Paul takes production')}
      <svg class="rc-fan rc-fan-single" width="56" height="228" viewBox="0 0 56 228" aria-hidden="true"><path class="rc-edge" data-path="p0" data-key="p0" d="M0 114 C28 114 28 114 56 114"/></svg>
      <div class="rc-hub" data-key="hub">
        <span class="rc-mono-label">Brain · coordination</span>
        <strong>2 active sessions</strong>
        <div class="rc-hub-overlap" data-key="overlap"><span>Overlap</span><code>infra/vault.tf</code><span>same repository, planned edits</span></div>
      </div>
      <svg class="rc-fan rc-fan-single" width="56" height="228" viewBox="0 0 56 228" aria-hidden="true"><path class="rc-edge" data-path="p1" data-key="p1" d="M0 114 C28 114 28 114 56 114"/></svg>
      ${session('paul', 'Paul · Codex', 'Ubuntu · Development', 'Migrate production secrets', ['infra/vault.tf', 'infra/prod.tfvars'], 'Mike plans to edit infra/vault.tf')}
    </div>
    ${storyControls(coordination)}
  </div>`,
};

export const stories = { system, loop, skills, tools, coordination };
export type StoryName = keyof typeof stories;
