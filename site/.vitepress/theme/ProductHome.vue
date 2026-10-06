<script setup lang="ts">
import { ref } from 'vue';
import { withBase } from 'vitepress';

const copyLabel = ref('Copy commands');
const commands = 'git clone https://github.com/MikeK184/Recollect.git\ncd Recollect\n./scripts/stack.sh up --build';
async function copyCommands() {
  try {
    await navigator.clipboard.writeText(commands);
    copyLabel.value = 'Copied';
  } catch { copyLabel.value = 'Select the commands to copy'; }
}
const tools = [
  { name: 'Context7', description: 'Documentation', placement: 'Recollect service' },
  { name: 'Exa', description: 'Web research', placement: 'Recollect service' },
  { name: 'Filesystem', description: 'Office files', placement: 'Private runner' },
];
</script>

<template>
  <div class="rc-product">
    <section class="rc-hero" aria-labelledby="product-title">
      <div class="rc-hero-copy">
        <p class="rc-eyebrow">SELF-HOSTED · OPEN SOURCE</p>
        <h1 id="product-title">Your agents.<br />One shared <em>Brain.</em></h1>
        <p class="rc-lead">Keep the knowledge. Give agents the right tools.</p>
        <p class="rc-description">Shared engineering memory, governed MCP tools and private execution. Connect your coding agents to Recollect and carry context across repositories, environments and sessions.</p>
        <div class="rc-actions">
          <a class="rc-button rc-button-primary" :href="withBase('/docs/installation.html')">Get started <span aria-hidden="true">↗</span></a>
          <a class="rc-button" :href="withBase('/docs/')">Read the docs <span aria-hidden="true">→</span></a>
        </div>
        <div class="rc-hosts" aria-label="Supported coding hosts"><span>Works with</span><b>Codex</b><b>Claude Code</b><b>OpenCode</b></div>
      </div>

      <div class="rc-example">
        <div class="rc-agent-request"><span class="rc-terminal" aria-hidden="true">&gt;_</span><div><strong>Your coding agent</strong><span>Plugin or direct MCP</span></div><span class="rc-request-label">Ask · Recall · Use tools</span></div>
        <div class="rc-connector" aria-hidden="true"><span></span>Context &amp; permitted tool calls<span>↓</span></div>
        <div class="rc-brain-card">
          <div class="rc-brain-heading"><img :src="withBase('/brand/recollect-symbol.svg')" alt="" width="36" height="36" /><div><h2>Engineering Brain</h2><span>Knowledge and tools, together.</span></div><span class="rc-badge">Shared</span></div>
          <div class="rc-brain-memory"><span class="rc-eyebrow">MEMORY WITH EVIDENCE</span><div><span>Decisions &amp; runbooks</span><span class="rc-evidence">↗ Sources</span></div><div><span>Repository knowledge</span><span class="rc-evidence">↗ Revisions</span></div><div><span>Session memory</span><span class="rc-evidence">↗ Context</span></div></div>
          <div class="rc-brain-tools"><div class="rc-tools-label"><span class="rc-eyebrow">APPROVED MCP TOOLS</span><span class="rc-badge">Development</span></div><div v-for="tool in tools" :key="tool.name" class="rc-tool-row"><span class="rc-tool-icon" aria-hidden="true">◇</span><div><strong>{{ tool.name }}</strong><span>{{ tool.description }}</span></div><span class="rc-placement">{{ tool.placement }}</span></div></div>
          <div class="rc-brain-footer"><span class="rc-permission"><span aria-hidden="true">✓</span> Explicit tool access</span><span>Runs where you choose</span></div>
        </div>
        <p class="rc-caption">Workflow example · Knowledge access and tool access are separate.</p>
      </div>
    </section>

    <section class="rc-principles" aria-label="What Recollect brings together">
      <div><span class="rc-eyebrow">01 / CONTINUITY</span><h2>Keep context.</h2><p>Decisions, fixes and handovers stay connected to their evidence.</p></div>
      <div><span class="rc-eyebrow">02 / ACCESS</span><h2>Choose the tools.</h2><p>Independent permissions and environment scope guide execution.</p></div>
      <div><span class="rc-eyebrow">03 / EXECUTION</span><h2>Reach your network.</h2><p>Paired private runners bring approved tools into the same workflow.</p></div>
    </section>

    <section class="rc-workflow" aria-labelledby="workflow-title">
      <div class="rc-section-heading"><div><p class="rc-eyebrow">MEMORY THAT TRAVELS WITH THE WORK</p><h2 id="workflow-title">Connect once. Keep context.</h2></div><p>Your next session starts with the knowledge your team has already built. Automatic capture and cited recall follow your Brain's policies.</p></div>
      <figure><img :src="withBase('/visuals/recollect-agent-workflow-v3.png')" width="1672" height="941" loading="lazy" alt="Codex, Claude Code and OpenCode connect through a plugin or direct MCP to a shared Engineering Brain with decisions, repository knowledge and session memory." /><figcaption>Product illustration with example data.</figcaption></figure>
    </section>

    <section class="rc-execution" aria-labelledby="execution-title">
      <div class="rc-section-heading"><div><p class="rc-eyebrow">FROM CONTEXT TO ACTION</p><h2 id="execution-title">The right tools.<br />In the right place.</h2></div><p>Recollect checks who can call a tool and where the connection belongs. Execution happens through the central service or its assigned runner.</p></div>
      <div class="rc-workflow-grid">
        <article><div class="rc-feature-copy"><span class="rc-eyebrow">ENVIRONMENTS &amp; TOOL GROUPS</span><h3>Make access deliberate.</h3><p>Group MCP connections and grant independent Use, Manage and Share permissions. Reading a Brain does not grant permission to run its tools.</p><a :href="withBase('/docs/mcp-catalogue.html')">Explore tool access <span aria-hidden="true">→</span></a></div><img :src="withBase('/visuals/recollect-environment-access-v3.png')" width="1672" height="941" loading="lazy" alt="A Development tool group controls access to Brain-wide Exa and Context7 connections, with calls dispatched by the Recollect service." /></article>
        <article><div class="rc-feature-copy"><span class="rc-eyebrow">PRIVATE RUNNERS</span><h3>Bring private tools into reach.</h3><p>A paired device connects outbound, runs approved calls inside your network, and returns results through Recollect to your coding agent.</p><a :href="withBase('/docs/mcp-vault-and-private-runners.html')">Explore private execution <span aria-hidden="true">→</span></a></div><img :src="withBase('/visuals/recollect-private-runner-v3.png')" width="1672" height="941" loading="lazy" alt="A coding agent asks Recollect to dispatch an approved filesystem call to an Ubuntu runner inside an office network. The runner connects outbound and returns the result." /></article>
      </div>
      <p class="rc-caption">Product illustrations use example data. A central HTTP connection calls its remote MCP service; environment scope controls availability.</p>
    </section>

    <section class="rc-start" aria-labelledby="start-title">
      <div><p class="rc-eyebrow">YOUR KNOWLEDGE. YOUR INFRASTRUCTURE.</p><h2 id="start-title">Start with one Brain.</h2><p>Run Recollect on your machine or a private server. Connect your agent, choose your policies, and build from there.</p><a class="rc-button rc-button-primary" :href="withBase('/docs/installation.html')">Installation guide <span aria-hidden="true">→</span></a><p class="rc-prerequisites">Docker with Compose · Python 3.11+</p></div>
      <div class="rc-command-card"><div><span>Run locally</span><button @click="copyCommands" aria-live="polite">{{ copyLabel }}</button></div><pre><code>{{ commands }}</code></pre><p>Then open <code>http://127.0.0.1:8787</code></p></div>
    </section>
    <div class="rc-status"><span class="rc-badge">Early project</span><p>Built for individuals and trusted internal teams. Workflows have local acceptance evidence; <a href="https://github.com/MikeK184/Recollect/blob/main/docs/roadmap/execution/active/README.md">current work and limits</a> remain visible.</p></div>
  </div>
</template>
