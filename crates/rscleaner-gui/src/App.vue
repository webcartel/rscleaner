<script setup lang="ts">
const navigation = [
  { label: 'Overview', icon: '▦' },
  { label: 'Cleanup', icon: '⌫', active: true },
  { label: 'Space', icon: '◔' },
  { label: 'Registry', icon: '▤' },
  { label: 'Rules', icon: '◇' },
  { label: 'Reports', icon: '▥' },
  { label: 'Settings', icon: '⚙' },
]
</script>

<template>
  <div class="app-shell">
    <header class="titlebar">
      <div class="brand-mark">R</div>
      <div class="brand-name">RS<span>Cleaner</span></div>
      <div class="title-divider"></div>
      <div class="current-section">Cleanup</div>
    </header>

    <div class="workspace">
      <aside class="sidebar">
        <div class="nav-caption">WORKSPACE</div>
        <nav class="nav">
          <a
            v-for="item in navigation"
            :key="item.label"
            class="nav-item"
            :class="{ active: item.active }"
            href="#"
          >
            <span class="nav-icon">{{ item.icon }}</span>
            {{ item.label }}
          </a>
        </nav>
        <div class="sidebar-bottom">
          <div class="engine-state"><span class="live-dot"></span>Engine ready</div>
          <div class="version">RSCLEANER <span style="float: right">0.1.0</span></div>
        </div>
      </aside>

      <main class="main">
        <div class="page-heading">
          <div class="eyebrow">RSCLEANER / WINDOWS 10 · 22H2</div>
          <h1>Cleanup</h1>
          <p>Review what RSCleaner will remove. Nothing runs until you confirm.</p>
        </div>

        <div class="metric-grid">
          <div class="metric-card">
            <div class="metric-label"><span class="metric-icon">◆</span>CAN FREE</div>
            <div class="metric-value">6.3<span>GB</span></div>
            <div class="metric-foot"><strong>42,180</strong> files · 12 rules</div>
          </div>
          <div class="metric-card">
            <div class="metric-label"><span class="metric-icon">◈</span>SELECTED</div>
            <div class="metric-value">5.4<span>GB</span></div>
            <div class="metric-foot"><strong>38,204</strong> files · 9 rules</div>
          </div>
          <div class="metric-card">
            <div class="metric-label"><span class="metric-icon">◇</span>RISK MIX</div>
            <div class="metric-value">7·3·2</div>
            <div class="metric-foot">safe · moderate · dangerous</div>
          </div>
        </div>

        <div class="placeholder-card">
          The M0 shell proves the Tauri pipeline, fonts, tokens, and window budgets.
          The Cleanup table, detail panel, and action bar arrive in M8.
        </div>
      </main>
    </div>
  </div>
</template>
