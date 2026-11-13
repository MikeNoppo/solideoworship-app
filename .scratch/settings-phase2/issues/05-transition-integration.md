# 05: Synchronize live, projector and Settings demo animations

Blocked by: 02,03,04
Status: ready-for-agent

Integrate Scene/Playback into App live mutations including Go Live, arrows/row clicks/label shortcuts and Show/Black/Clear, using saved per-kind options. One shared frame sample drives live preview/output. Subscribe to native frame events only while animation or dialog demo exists; drop old snapshots and stop on completion. Retarget rapid inputs, preserve v1 monitor-close/move/Black/Clear and stale modal protections. Wire Settings Preview locally without changing real Live. Update README and record all checks/runtime QA. Remove interim adapters/unused keys without suppressions. Mark implemented pending review, not finally resolved until review fixes and root QA.

- [ ] All navigation paths and modes use correct settings; latest input isn't queued behind animation.
- [ ] Native frames stop at idle; preview/output/demo are consistent and snapshots bounded.
- [ ] Full tests/clippy/MSRV/format and available native QA pass; no v1 regressions.
