# 0021: Grouped spatial graph navigation

Status: accepted

## Decision

The user's 2026-10-18 instruction to implement the complete researched graph plan
reopens 3D presentation. Add lazy `react-force-graph-3d`/Three.js over the same
qualified bounded nodes, edges, selection and evidence inspection as 2D. Use
stable, named topic groups with explicit expansion, readable selection and finite
layout settling. Retain Cytoscape 2D and accessible HTML controls as alternatives.
The existing cream/ink/sage visual language remains (amended 2026-10-19 by
[ADR 0025](0025-two-theme-desktop-redesign.md): both renderers take their
colours from the Light and Ink design tokens).

PostgreSQL owns identity, topics, scope and evidence. Neo4j/GDS owns declared
native graph computation. Three.js and layout physics only position the current
authorized selection; visual proximity never creates a relationship or topic.
No graph database replacement or new extraction runtime is selected.

## Why

The earlier sphere display did not provide usable organization. Source-backed
topics, controlled expansion and a stable camera make the spatial view useful
without requiring an entire Brain to be loaded in the browser.

## Consequences

The [grouped 3D contract](../contracts/graph-grouped-3d-navigation.md) governs
fallback, bounds, accessibility, stability and performance proof. Dependency
installation is not browser compatibility or performance proof. Graph reliability
and automatic organization keep their independent full acceptance requirements.

## Supersession

Supersedes the renderer-only flat-graph correction in
[ADR 0017](0017-desktop-knowledge-and-ask-experience.md#flat-graph-correction--2026-10-03)
and [graph exploration](../contracts/graph-exploration.md#flat-graph-correction--2026-10-03).
All canonical qualification, contribution, path and validity rules remain.
