# Roadmap

Start with the [current product status and remaining features](epics/index.md).
It separates implemented behavior, unfinished authorized work and proposed
follow-ups. Product implementation and its acceptance are tracked there;
documentation maintenance has its own delivery record.

- [Epics](epics/index.md) own capabilities and slice identities.
- [Active packs](execution/active/README.md) specify unfinished risky slices.
- [Execution lifecycle](execution/README.md) says when a pack is required;
  every other slice is a row in its epic with acceptance and validation inline.
- Shipped packs were removed from the tree on 2026-10-19; they remain in
  [Git history](https://github.com/MikeK184/Recollect/tree/873156174f421da0574952a5e533fb6a6133fab0/docs/roadmap/execution/archive).
- [Desktop design system](../foundation/design-system.md) governs the
  current light desktop presentation; [desktop operation](../runbooks/desktop-experience.md)
  describes the implemented navigation.

Accepted [ADRs](../adr/README.md), [contracts](../contracts/README.md) and
[foundations](../foundation/README.md) govern behavior. Roadmap priority does not
replace a missing contract or prove runtime acceptance.
