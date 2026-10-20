<script setup lang="ts">
import { ref } from 'vue';
import { withBase } from 'vitepress';
import StoryDiagram from './StoryDiagram.vue';
import { icon } from './diagrams/icons';

const run = 'git clone https://github.com/MikeK184/Recollect.git\ncd Recollect\n./scripts/stack.sh up --build';
const connect = 'python3 scripts/package-plugin.py\nclaude plugin marketplace add <package-directory>\nclaude plugin install recollect-memory@recollect';
const copied = ref('');
async function copy(id: string, text: string) {
  try {
    await navigator.clipboard.writeText(text);
    copied.value = id;
    setTimeout(() => { if (copied.value === id) copied.value = ''; }, 1600);
  } catch { copied.value = `${id}-failed`; }
}

const capabilities = [
  { icon: 'notebook', title: 'Memory with evidence', body: 'Decisions, fixes and handovers are learned from sessions and repositories, checked against their sources and recalled with citations.', link: '/docs/exact-lexical-recall.html', more: 'Recall with evidence' },
  { icon: 'file', title: 'Dynamic skills', body: 'Publish a skill or shared instructions once. Every connected host loads the current revision natively, when it applies.', link: '/docs/managed-skills.html', more: 'Shared skills' },
  { icon: 'unplug', title: 'Governed MCP tools', body: 'Tool groups with separate Use, Manage and Share rights decide who may run which tools, in which environment.', link: '/docs/mcp-catalogue.html', more: 'Tool groups' },
  { icon: 'shield', title: 'Private execution', body: 'A paired runner connects outbound and runs approved calls inside your network. It needs no inbound port.', link: '/docs/mcp-vault-and-private-runners.html', more: 'Private runners' },
  { icon: 'users', title: 'Agents that coordinate', body: 'Sessions on different machines announce their scope and planned files, and exchange advisory notes before their work collides.', link: '/docs/plugin.html#peer-coordination', more: 'Peer coordination' },
  { icon: 'network', title: 'Code and knowledge graphs', body: 'Explore how repositories, configuration and decisions connect, with paths that cross repository boundaries.', link: '/docs/graph-exploration.html', more: 'Explore graphs' },
];

const families = [
  { name: 'workspace', purpose: 'Scope every task', tools: ['list', 'start_task', 'set_scope', 'begin', 'inspect_task', 'close'] },
  { name: 'memory', purpose: 'Recall and contribute', tools: ['recall', 'inspect', 'contribute', 'handover', 'graph_explore', 'graph_path'] },
  { name: 'skills', purpose: 'Load shared guidance', tools: ['list', 'fetch'] },
  { name: 'source', purpose: 'Retain documents', tools: ['list', 'import', 'inspect'] },
  { name: 'coordination', purpose: 'Work alongside peers', tools: ['announce', 'peers', 'inbox', 'send', 'ack', 'close'] },
  { name: 'mcp', purpose: 'Run governed tools', tools: ['profiles', 'discover', 'call', 'status', 'cancel', 'reconcile'] },
];

const placements = [
  { icon: 'server', title: 'Recollect service', body: 'Central HTTP connections such as Context7 or Exa. Recollect makes the MCP request after its checks.' },
  { icon: 'laptop', title: 'Caller’s device', body: 'A local stdio server on the machine where the agent runs, started by the plugin’s optional runner.' },
  { icon: 'shield', title: 'Private runner', body: 'A named, paired device inside your network. It connects outbound, claims addressed calls and returns results.' },
];
</script>

<template>
  <div class="rc-product">
    <section class="rc-hero" aria-labelledby="product-title">
      <div class="rc-hero-copy">
        <a class="rc-announce" :href="withBase('/docs/managed-skills.html')"><span class="rc-announce-tag">New</span>Dynamic skills for Codex, Claude Code and OpenCode<span v-html="icon('arrow', 14)" /></a>
        <h1 id="product-title">Every coding agent starts where the last one stopped.</h1>
        <p class="rc-lead">Recollect is a self-hosted Brain for your coding agents. It remembers what they learn, gives every session cited context, keeps your team’s skills in sync and decides which MCP tools an agent may run, and where.</p>
        <div class="rc-actions">
          <a class="rc-button rc-button-primary" :href="withBase('/docs/installation.html')">Get started<span v-html="icon('arrow', 16)" /></a>
          <a class="rc-button" href="#how-it-works">See how it works</a>
        </div>
        <div class="rc-hosts" aria-label="Supported coding hosts"><span>Works with</span><b>Codex</b><b>Claude Code</b><b>OpenCode</b></div>
      </div>
      <div class="rc-hero-visual"><StoryDiagram name="system" /></div>
    </section>

    <section class="rc-capabilities" aria-label="What Recollect does">
      <a v-for="item in capabilities" :key="item.title" class="rc-capability" :href="withBase(item.link)">
        <span class="rc-tile" v-html="icon(item.icon, 16)" />
        <h2>{{ item.title }}</h2>
        <p>{{ item.body }}</p>
        <span class="rc-more">{{ item.more }}<span v-html="icon('arrow', 14)" /></span>
      </a>
    </section>

    <section id="how-it-works" class="rc-section" aria-labelledby="loop-title">
      <header class="rc-section-head">
        <span class="rc-mono-label">How it works</span>
        <h2 id="loop-title">One loop on every prompt.</h2>
        <p>Install the plugin once. From then on, each session feeds the Brain and each new prompt starts with what the team already knows, under your capture, retention and model policies.</p>
      </header>
      <StoryDiagram name="loop" />
    </section>

    <section class="rc-section" aria-labelledby="skills-title">
      <header class="rc-section-head">
        <span class="rc-mono-label">Dynamic skills</span>
        <h2 id="skills-title">Publish a skill once. Every host has it next session.</h2>
        <p>Brain managers publish skills and shared instructions in the desktop app. The plugin installs them as native skills in Claude Code, Codex and OpenCode, reloads the host, and removes temporary copies when the task ends.</p>
      </header>
      <StoryDiagram name="skills" />
      <div class="rc-facts">
        <div><span class="rc-mono-label">Per Brain or environment</span><p>Brain-wide guidance applies everywhere; environment guidance adds to it only when that environment is selected.</p></div>
        <div><span class="rc-mono-label">Loaded on demand</span><p>Agents see compact metadata first and fetch a body or reference only when a skill applies.</p></div>
        <div><span class="rc-mono-label">Safe by construction</span><p>Markdown and text only, with safe paths and secret detection. Guidance never grants a tool or overrides local rules.</p></div>
      </div>
    </section>

    <section class="rc-section" aria-labelledby="tools-title">
      <header class="rc-section-head">
        <span class="rc-mono-label">Governed MCP tools</span>
        <h2 id="tools-title">Agents ask. Recollect checks. Tools run where you choose.</h2>
        <p>Connect an MCP server once, add it to a tool group and grant people Use. Reading a Brain never grants permission to run its tools, and administrators do not inherit Use.</p>
      </header>
      <StoryDiagram name="tools" />
      <div class="rc-placements">
        <div v-for="item in placements" :key="item.title" class="rc-placement">
          <span class="rc-tile" v-html="icon(item.icon, 16)" />
          <div><h3>{{ item.title }}</h3><p>{{ item.body }}</p></div>
        </div>
      </div>
    </section>

    <section class="rc-section" aria-labelledby="surface-title">
      <header class="rc-section-head">
        <span class="rc-mono-label">One MCP endpoint</span>
        <h2 id="surface-title">The tools your agent sees.</h2>
        <p>The plugin and direct MCP clients reach the same scoped tools at <code>/api/brains/&lt;brain&gt;/mcp/agent</code>. Every call is bound to an immutable task scope, so a model cannot switch Brains or widen its own access.</p>
      </header>
      <div class="rc-families">
        <div v-for="family in families" :key="family.name" class="rc-family">
          <div class="rc-family-head"><code>{{ family.name }}.*</code><span>{{ family.purpose }}</span></div>
          <ul><li v-for="tool in family.tools" :key="tool"><code>{{ tool }}</code></li></ul>
        </div>
      </div>
    </section>

    <section class="rc-section rc-start" aria-labelledby="start-title">
      <header class="rc-section-head">
        <span class="rc-mono-label">Get started</span>
        <h2 id="start-title">Running in three steps.</h2>
        <p>Everything runs on your machine or a private server: PostgreSQL with pgvector, Neo4j, and the Rust API and worker behind the desktop app.</p>
      </header>
      <ol class="rc-steps">
        <li>
          <span class="rc-step-index">1</span>
          <h3>Run Recollect</h3>
          <p>Needs Docker with Compose and Python 3.11 or newer. Then open <code>http://127.0.0.1:8787</code>; the first run writes your login to <code>.env</code>.</p>
          <div class="rc-command"><div class="rc-command-bar"><span>shell</span><button type="button" @click="copy('run', run)" aria-live="polite">{{ copied === 'run' ? 'Copied' : copied === 'run-failed' ? 'Select to copy' : 'Copy' }}</button></div><pre><code>{{ run }}</code></pre></div>
        </li>
        <li>
          <span class="rc-step-index">2</span>
          <h3>Connect your agent</h3>
          <p>Open <b>Agents → Connect agent</b>. Build and install the plugin, then run the <code>recollect-connect</code> skill and approve the device code in your browser.</p>
          <div class="rc-command"><div class="rc-command-bar"><span>Claude Code</span><button type="button" @click="copy('connect', connect)" aria-live="polite">{{ copied === 'connect' ? 'Copied' : copied === 'connect-failed' ? 'Select to copy' : 'Copy' }}</button></div><pre><code>{{ connect }}</code></pre></div>
        </li>
        <li>
          <span class="rc-step-index">3</span>
          <h3>Work as usual</h3>
          <p>Start Codex, Claude Code or OpenCode normally. Cited context arrives before each prompt; add skills and tool groups whenever you need them.</p>
          <div class="rc-next">
            <a :href="withBase('/docs/agent-memory-tools.html')">Connect Codex or OpenCode<span v-html="icon('arrow', 14)" /></a>
            <a :href="withBase('/docs/provider-learning.html')">Turn on learning<span v-html="icon('arrow', 14)" /></a>
            <a :href="withBase('/docs/managed-skills.html')">Publish a skill<span v-html="icon('arrow', 14)" /></a>
            <a :href="withBase('/docs/mcp-catalogue.html')">Add a tool group<span v-html="icon('arrow', 14)" /></a>
          </div>
        </li>
      </ol>
    </section>

    <div class="rc-status">
      <span class="rc-pill rc-pill-warning">Early project</span>
      <p>Built for individuals and trusted internal teams. Workflows have local acceptance evidence; <a href="https://github.com/MikeK184/Recollect/blob/main/docs/roadmap/epics/index.md">current work and limits</a> stay visible. Diagrams use example data.</p>
    </div>
  </div>
</template>
