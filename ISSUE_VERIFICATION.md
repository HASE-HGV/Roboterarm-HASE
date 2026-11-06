# Issue verification overview

Verification of the automatically generated issues filed against `HASE-HGV/Roboterarm-HASE`
(issues #28–#51). Each issue was checked against `origin/main` at commit `0903d16`, not against
an older local checkout. Simulation builds were used unless noted; hardware-only behaviour could
not be run here.

Checks were done on 2026-11-06. Test baseline on `origin/main`: `cargo test` passes,
544 tests across 11 targets, 0 failures.

## Verdicts

| # | Title (short) | Verdict | Notes |
|---|---|---|---|
| 28 | NaN joint angles become zero steps | Partial (corrected) | Only the text input path accepts NaN. JSON routes cannot carry NaN. |
| 29 | Huge angles saturate to i64::MAX steps | Confirmed | Planner overflow above ~2^62 only matters in hardware builds. |
| 30 | Web preview uses a different IK branch | Confirmed | Controller picks the branch with the larger shoulder angle; for x > 0 that is always the negative elbow. |
| 31 | Shutdown waits for in-flight move | Partial | `/quit` returns immediately; only process exit waits. Ctrl+C already stops a move in hardware builds. The real gap is that Ctrl+C can't stop the server after the first move. |
| 32 | Duplicate IPs in banner | Confirmed | Non-adjacent duplicates survive `dedup_by`. |
| 33 | Body `command` overrides route | Confirmed | `POST /args` with `"command":"raw"` runs a raw move. |
| 34 | Expose last pose / constants (feature) | Confirmed (premise) | No pose is stored; the page hard-codes constants. Start fields are already optional. |
| 35 | Ctrl+C move reports success | Partial | Interrupted move returns `Ok`, so status is `done`. `--cli` prints nothing on success; `--raw` is also affected. Runtime needs a Pi. |
| 36 | Ctrl+C stops working after first move | Confirmed (code level) | Handler is installed after GPIO init. The site loop reads a shutdown flag, but only from `/quit`. Runtime needs a Pi. |
| 37 | gpioTest u64 underflow | Confirmed | Every input 0–134 underflows, not only 50–134. Reproduction needs a Pi because `gpio_init` runs first. |
| 38 | encodethesecond.py subfolder and collisions | Confirmed | Real ffmpeg was not run; a fake ffmpeg reproduced the messages. |
| 39 | Default image: root/root, SSH, port 2000 | Confirmed | sshd change applies only with Guacamole enabled and username `root`. Already disclosed in the README and config docs. |
| 40 | No per-joint angle limits (feature) | Confirmed | `/raw` accepts 500° and −1e9°. |
| 41 | No acceleration ramp (feature) | Confirmed | README rates this Medium; "main cause" is loose wording. |
| 42 | Config file for gear ratio/timing (feature) | Confirmed (premise) | Pin constants exist only in hardware builds. |
| 43 | DNS rebinding bypasses origin check | Confirmed (server side) | Matching Origin/Host passes. Ordinary cross-site pages get 403. Browser-side rebinding was not tested. |
| 44 | Control API on all interfaces | Confirmed | Default `0.0.0.0:8080`, no authentication. |
| 45 | Guacamole hash readable on image | Confirmed | Mode `0644`. "Every user" depends on the `/etc/guacamole` directory mode, which the code doesn't set. |
| 46 | Unbounded threads (DoS) | Confirmed (core) | 300 idle connections → 301 threads; `/status` still answered. Slow sends hold a thread longer than 5 s. 2000-connection case not tested. |
| 47 | Committed unencrypted SSH key | Confirmed | Cipher `none` is inside the base64 body, not the header line. Key contents were not printed. |
| 48 | Help text suggests build-time env var | Confirmed | Only runtime `RUSTCTL_SITE_ADDR` is read. The suggested `sudo` form was not tested. |
| 49 | CI never builds hardware feature | Confirmed | `cargo check --features hardware` passes on Linux. |
| 50 | Docker build has no GPIO feature | Confirmed | README already documents the limitation. |
| 51 | ccw_positive accepts `2` | False alarm (closed) | Fixed on `origin/main`; the bug existed only in an older local checkout. |

## What is still unverified

- Runtime Ctrl+C behaviour after the first real hardware move (#31, #35, #36, #37): needs a Raspberry Pi with GPIO.
- Browser-side DNS rebinding (#43): needs a browser and a controlled DNS name.
- Scale of the connection-exhaustion attack (#46): 2000 connections not tested.
- Real ffmpeg output (#38): a fake ffmpeg was used.
- Whether `sudo` passes `RUSTCTL_SITE_ADDR` through (#48): depends on sudoers on the Pi.
- Whether the base Raspberry Pi OS overrides the sshd settings (#39): no image was built.

## Method

1. Each issue was read with `gh issue view`.
2. Cited code was read from `origin/main` with `git show`.
3. Behaviour was reproduced in a copy of `rustctl` built from `git archive origin/main`.
4. Each verdict was reviewed against the evidence before it was recorded here.

Issue #51 was closed as not planned. Issue deletion was not possible with the available
GitHub permissions.
