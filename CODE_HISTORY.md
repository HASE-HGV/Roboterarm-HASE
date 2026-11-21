# Code History — Roboterarm HASE

> A living documentation of how this project evolved, generated from the Git
> history of the `HASE-HGV/Roboterarm-HASE` repository (branch `main`, plus
> merged feature/fix branches). All dates, counts and commit references below
> were derived from `git log` and `git shortlog`.

- **Repository:** `HASE-HGV/Roboterarm-HASE`
- **Current branch at generation:** `fix/security-hardening-issues`
- **History span:** 2025-11-14 → 2026-11-20
- **Total commits (mainline history):** 297
- **Commits across all branches:** 346
- **Active branches (local + remote):** 57
- **Files tracked:** 108
- **Source lines (Rust):** ~6,792 (`rustctl/src`) + ~710 (`rustctl/tests`)
- **Source lines (Python):** ~1,087
- **Tags:** `v1.1.0-beta`, `v.1.1.2-backup`, `backup`, `backup@18.11.16`

---

## 1. Project in one paragraph

Roboterarm HASE is an open-source 3-axis robotic arm (base rotation, shoulder,
elbow) plus a reserved auxiliary axis, driven by four A4988 stepper drivers on a
Raspberry Pi 4B+. The control software computes 3D inverse kinematics, converts
the resulting joint angles into synchronized motor step counts (Bresenham-style
interpolation), and pulses the GPIO pins. It ships with a CLI, an interactive
shell, an HTTP/JSON control API, a browser-based control page, a gpioTest
hardware utility, an offline Raspberry Pi image builder, and a Docker setup.

---

## 2. Snapshot

### 2.1 Top-level layout

| Path | Purpose |
|------|---------|
| `rustctl/` | Main control program (Rust, edition 2024) — 11 modules + 10 test files |
| `gpioTest/` | Standalone hardware test utility (Rust) |
| `image-builder/` | Builds compact offline Raspberry Pi OS images (Python + Docker) |
| `docker/` | Containerised build/runtime environment |
| `arduino-oscilloscope/` | Submodule: Arduino-based signal analysis tool |
| `pi-gpio-emulator/` | Submodule: GPIO emulation for PC development |
| `VI21-Anlagen/` | Design assets (blender renders, gearboxes) |
| `hw-controller/` (historical) | Original Python control software (removed from mainline) |
| `.github/workflows/` | CI: Rust tests, CodeQL, coverage generator |
| `README.md` | Full project documentation |

### 2.2 Current Rust modules (`rustctl/src`)

| Module | LOC | Responsibility |
|--------|----:|----------------|
| `main.rs` | 93 | Entry point, mode dispatch |
| `config.rs` | 182 | Pin mapping, motor constants, geometry |
| `kinematics.rs` | 153 | 3D inverse kinematics (spherical/cylindrical) |
| `motion.rs` | 291 | Step planning + synchronized motor execution |
| `bresenham.rs` | 45 | Bresenham line interpolation for multi-motor sync |
| `control.rs` | 32 | High-level move commands |
| `api.rs` | 354 | Command parsing / JSON request handling |
| `http_api.rs` | 736 | HTTP server, routing, web assets |
| `cli.rs` | 104 | Argument parsing for CLI modes |
| `shell.rs` | 154 | Interactive shell / raw mode |
| `net.rs` | 90 | Network helpers (host validation, overload caps) |
| `pretty.rs` | 41 | Pretty-printing for logs/output |

---

## 3. Contributors

Aggregated by commit author (note: several contributors used multiple
name/email identities; counts below are per identity as recorded by Git).

| Contributor (identity) | Commits | First commit | Last commit |
|------------------------|--------:|--------------|-------------|
| lucagurr | 93 | 2026-09-07 | 2026-11-20 |
| Luca Gurband | 58 | 2026-02-03 | 2026-11-20 |
| Catrick-cpu | 48 | 2025-11-14 | 2026-05-27 |
| LucaGurr | 40 | 2026-02-03 | 2026-11-20 |
| Catrick The Game | 39 | 2026-02-15 | 2026-11-19 |
| Patrick Streng | 30 | 2026-02-14 | 2026-11-20 |
| frozen brain | 13 | 2025-11-15 | 2026-07-05 |
| Johannes (Johannes) | 12 | 2025-11-15 | 2026-01-23 |
| github-actions[bot] | 8 | 2026-03-26 | 2026-11-20 |
| dependabot[bot] | 2 | 2026-02-15 | 2026-11-03 |
| Flxx94 | 1 | 2026-11-04 | 2026-11-04 |
| MineChampion09 | 1 | 2026-03-05 | 2026-03-05 |

Roles (per README team section): **Patrick** — software, cabling, PCB;
**Luca** — organisation, Rust, web server, 3D design/printing, oscilloscope;
**Johannes** — build suite, Docker, Linux setup; **Florian** — soldering;
**Julian** — soldering, calculations & design review.

---

## 4. Development timeline

The project went through several distinct eras. Each era is described with its
date range, representative commits and what changed.

### Era 0 — Bootstrap (2025-11-14 … 2025-11-15)
- `6f0f704` Initial commit
- `8ade5b3` Create `main.py`
- `f7a10ed` / `7647f1d` First `serverconsole` script + controller
- `f0fa73e` Create `.gitignore` (Johannes)
- `fb35fc9` HTTP server base & core commands
- `59b2dd2` … `074932c` README development instructions & formatting

Outcome: a tiny Python/HTTP prototype plus documentation scaffolding. The
repository was still named `HASE-Roboterarm` and lived under `Catrick-cpu`.

### Era 1 — Python hardware controller (2025-11-17 … 2025-12-12)
- `c6eb0f3` "Added functional feature"
- `50821c2` Move controller scripts into `hw-controller/`
- `345cd6a` / `7462995` Add motor test scripts, simplify server controller
- `fa43a5e` Refactor `motortest.py` with improved CLI and simulation
- `966219c` … `f21bfb0` English option names, stepper constants, argument keys

Outcome: the `hw-controller/` package (config, gpio_setup, ik, motor, robot,
motortest) became the working Python control stack, with a stepper simulation
for PC testing.

### Era 2 — Website & multi-motor Python (2026-01-22 … 2026-02-16)
- `e9b059d` WEBSITE ADD
- `2775d46` Update `content.js`
- `e210dba` "Adding support for more motors"
- `96ac137` ADDED UNIT TEST!!! → `76966de` "Removed unittest -> to complicate"
- `5bfcf92` ADDED GPIO SIMULATION
- `b2553ff` "New script structure"
- Governance files: `CODE_OF_CONDUCT.md`, `CONTRIBUTING.md`, `LICENSE` (GPL-3),
  issue templates, dependabot config
- `v1.1.0-beta` tagged (2026-02-16)

Outcome: a browser front-end appeared, GPIO could be simulated on a PC, and the
project gained community/governance scaffolding. The first unit-test attempt was
abandoned as "too complicated" — a recurring theme that resurfaces in Era 6.

### Era 3 — Hardware, PCB & kinematics math (2026-03-05 … 2026-04-17)
- `246305f` pcb
- `37eba2d` server python
- `bac9f45` / `1dda116` new control script with webserver
- `cae5629` server update (`v.1.1.2-backup` tagged)
- `c1d4753` / `0813423` `ods` (inverse kinematics spreadsheet work)

Outcome: PCB design, the `inverseKinematics.ods` spreadsheet (basis for the
later Rust IK), and a webserver-based control script.

### Era 4 — Go HTTP server experiment (2026-01-22 … 2026-02-13, merged into main 2026-06)
- `fb35fc9` Httpserver base & core commands
- `071db82` / `261683c` httpserver code merge attempts (with a revert)
- `f5c412c` clean up!!!
- `a1b8189` Remove comments/examples from `main.rs` for line compression
- Go files: `main.go`, `server.go`, `motorController.go`, `scripting.go`,
  `logger.go`, `wrapper.sh`, `include/...`

Outcome: a Go-based HTTP server experiment (with embedded web assets) was tried
and partially landed, then largely superseded. Roughly 14 `.go` files were
added across the project's lifetime; the long-term direction moved to Rust.

### Era 5 — Rust port & motion core (2026-05-27 … 2026-05-30)
- `31daa31` moved `main.rs` to cargo project
- `0c544b8` added dependencies to cargo
- `c35246a` multi-motor support using structs per motor
- `a4293ac` IK from `inverseKinematics.ods`, improved formatting
- `01331ff` refined sleep logic + v0.9 rough Bresenham
- `0ba4ce2` added 16/1 gearbox logic, IK "should be finished now"
- `4e7bede` hardware execution offset tuned 55 µs → 83 µs after long-run test
- `b370897` updated rotation algorithm, interactive input handler
- `ea07f66` ctrlc graceful shutdown (SIGINT/SIGTERM/SIGHUP)
- `9193ce4` integrate `haseforge` library

Outcome: the project's centre of gravity moved to Rust. Inverse kinematics,
gearbox ratios, Bresenham multi-motor synchronisation and graceful shutdown all
landed within ~4 days.

### Era 6 — 3D rendering, gpioTest & packaging (2026-06-01 … 2026-07-21)
- `1fdb7e6` / `aeb7144` / `0876d0c` animation & video assets
- `4f8899e` added `gpioTest`
- `697889a` … `5ab1739` CLI refactors (args → shell mode, cylindrical coords,
  fixed-timing CLI modes)
- `94e6592` … `d842cdb` delay & GPIO emergency fixes
- `3e95a3f` / `099614a` added `pi-gpio-emulator` submodule
- `d8ddc60` initial Docker implementation
- `395bbc2` added image build suite
- `backup` tag (2026-10-23 image-builder feature)

Outcome: rendering/animation assets, a separate hardware-test binary, Docker
support, and the start of the offline image builder. The Rust CLI gained
multiple modes (args, shell, raw, API).

### Era 7 — API, modularity & web control page (2026-09-07 … 2026-09-25)
- `6f8f455` / `7b27b26` optimization + tests
- `d65a3a1` first API implementation
- `539490b` refactor API to JSON requests/responses; add test command
- `5ea0580` "beginning of rewrite"
- `bc6e418` modularise `rustctl` into separate modules
- `c494688` / `5b9bd17` Merge rewrite into api
- `344f208` / `60b33ac` / `0d663c3` test coverage details in README
- `a9d1caf` delete `.blend` files; `0bfc146` update `index.html`
- `d8ddc60` Dockerfile; `f1cfec4` merge docker

Outcome: the monolithic `main.rs` was split into the module layout that still
exists today (`api`, `http_api`, `kinematics`, `motion`, `bresenham`, …), the
API became JSON-based, and the web control page gained a WebGL preview and
desktop-window theming.

### Era 8 — Tests, CI & quality (2026-09-16 … 2026-11-06)
- `334f847` expanded test portfolio
- `a12702f` refactor CLI/HTTP API I/O handling + command execution tests
- `5624a50` increase rustctl test coverage
- `79099bc` "cargo deny warnings to enforce clean code, THIS PUSH CURRENTLY
  DOES NOT COMPILE!!!"
- `acda1c0` all cargo errors resolved (promoted warnings)
- `c5510ee` fixed dead imports
- `edef727` add GitHub Actions workflow for Rust
- `3fddd22` / `55c20d6` enable tests for the main binary + coverage tests
- `81980e5` "completed test suite and lack of sleep 0_o"
- `b8a5433` more unit tests for dark spots
- `0903d16` test results file

Outcome: a serious test/CI push — the test suite grew to ~710 lines, warnings
were promoted to errors, and the project gained Rust + CodeQL workflows. The
commit messages honestly record a painful compile-fix session.

### Era 9 — Security & robustness hardening (2026-11-06 … 2026-11-20)
A concentrated burst (29 commits on 2026-11-06 alone) of fix PRs:

| PR | Commit | Fix |
|----|--------|-----|
| #56 | `c94e650` | gpioTest delay calculation underflow |
| #57 | `e7e1872` | report end of input clearly in `--cli` prompts |
| #58 | `f86a20e` | keep shell/raw modes running after non-UTF-8 input |
| #59 | `764350e` | compile hardware feature on Linux (CI) |
| #60 | `cd285a0` | reject JSON command that does not match the route |
| #61 | `f86a20e`/`f86a…` | back off & rate-limit output when `accept()` fails |
| #62 | `4277582` | correct README setup steps & file tree |
| #63 | `df4d3c2` | reject chunked request bodies with a clear error |
| #64 | `ad12e83` | refuse joint angles too large to plan safely |
| #64b | `993a49c` | refuse joint angles too large to plan safely |
| — | `eea40cd` (HEAD) | validate request hosts & cap HTTP overload |

Plus `49e938e` stop panicking on transient `accept()` failures, `6b8eb32`
preemptive exiting in `motion.rs`, `057fffe` unauthenticated control API
loopback-only, and `7d1def3` panic on preemptive stop.

Outcome: the HTTP server and motion planner became defensive (host validation,
overload caps, chunked-body rejection, safe angle limits, loopback-only control
API, no panic on transient accept failures).

### Era 10 — Documentation, stats & gallery (2026-11-04 … 2026-11-07)
- `2ef9329` / `904ecf5` issue verification overview + bug fixes
- `504c7ec` / `357610f` star history chart + statistics gallery in README
- `094ca7f` / `d82e761` live project stats in README
- `7935c14` dynamic test-count badge; `2c1358f` rename to "unit tests"
- `93473f1` removed "Patrick's AI slop" from README
- `711ec15` Mermaid diagram of function paths across source files
- `f86c996` iterators in place of for loops + jog-mode fix
- `7342ab5` replaced remaining for loops with iterators

Outcome: README became a rich, auto-updating dashboard (badges, stats, gallery,
Mermaid diagrams) and the codebase shifted toward iterator-based control flow.

---

## 5. Architecture evolution

```
2025-11  Python prototype (main.py, serverconsole*)        ─┐
2026-01  + browser front-end (content.js, index.html)        │
2026-02  + GPIO simulation, multi-motor, governance          │  Python era
2026-03  + PCB, inverseKinematics.ods, webserver script       │
2026-04  (backup tag)                                        │
         ─────────────────────────────────────────────────────┘
2026-01  Go HTTP server experiment (main.go, server.go, ...)  ─┐ Go experiment
2026-02  (merged, then superseded)                            ┘
2026-05  Rust port: main.rs → cargo project, IK, gearbox,     ─┐
         Bresenham sync, graceful shutdown, haseforge             │
2026-06  + gpioTest, pi-gpio-emulator submodule, Docker,       │ Rust era
         image-builder, rendering assets                        │
2026-09  + JSON API, modularisation, web control page (WebGL)  │
2026-10  + tests/CI, deny-warnings, coverage                   │
2026-11  + security hardening, README stats/gallery             ┘
```

Key structural milestones:
- **Modularisation** (`bc6e418`, 2026-09-16): split monolithic `main.rs` into
  `api`, `bresenham`, `cli`, `config`, `control`, `http_api`, `kinematics`,
  `motion`, `net`, `pretty`, `shell`.
- **API** (`d65a3a1` → `539490b`): placeholder → JSON request/response API.
- **Hardware feature gating** (`bfafcd9`, 2026-09-10): Pi-only features no
  longer auto-enable on every Linux machine; build with `--features hardware`.

---

## 6. Subsystem history

### 6.1 Inverse kinematics
- Spreadsheet prototype: `inverseKinematics.ods` (2026-03).
- Ported to Rust: `a4293ac` (2026-05-28) "added IK from inverseKinematics.ods".
- Gearbox: `0ba4ce2` (2026-05-29) 16/1 gearbox logic; IK "should be finished now".
- Safety: `993a49c`/`ad12e83` (2026-11-06) refuse joint angles too large to plan.

### 6.2 Motion & synchronisation
- `01331ff` v0.9 rough Bresenham (2026-05-28).
- `4e7bede` execution offset tuned from 55 µs to 83 µs after long-run test.
- `6b8eb32` / `7d1def3` preemptive stop handling in `motion.rs`.
- `f86c996` fixed steps not being sent continuously on key hold in jog mode.

### 6.3 HTTP server
- Go experiment: `fb35fc9` … `f5c412c`.
- Rust HTTP API module `http_api.rs` (736 LOC) is now the largest module.
- Hardening: loopback-only control API (`057fffe`), host validation + overload
  caps (`eea40cd`), chunked-body rejection (`df4d3c2`), accept-loop backoff
  (`f86a20e`), non-panic on transient accept failures (`49e938e`).

### 6.4 Web control page
- Added 2026-01-23 (`e9b059d`).
- Keyboard jog controls (`9ae5cfb`, 2026-10-23).
- Desktop window theme + server quit control (`6de3c4f`, 2026-10-19).
- macOS/Windows 10 themes (`a05431a`, 2026-10-19).
- WebGL preview (`83fc921`, 2026-10-21).

### 6.5 gpioTest
- Added 2026-06-19 (`4f8899e`).
- Pin choice + hardware overhead (`43535b0`).
- Delay underflow fix (`c94e650`, PR #56).

### 6.6 Image builder
- Started 2026-10-16 (`395bbc2`).
- Compact offline Raspberry Pi OS images (`d5e692f`, 2026-10-23).
- `image-builder/` contains Python CLI, Dockerfile, compose, configs.

### 6.7 Docker
- Initial implementation `d8ddc60` (2026-09-25), merged via PR #26.
- `3f0cd59` final Dockerfile fix.

### 6.8 CI / quality
- Workflows: `rust.yml`, `codeql.yml`, `coveragegenerator.yml`.
- Dependabot for Cargo and GitHub Actions.
- Warnings promoted to errors (`79099bc` → `acda1c0`).

---

## 7. Branching & pull-request workflow

The project uses feature branches with `Merge pull request #N` merges. Notable
integration points:

- `rewrite` → `api` → `main` (2026-09-16)
- `tests` → `main` (PR #25, 2026-09-16)
- `docker` → `main` (PR #26, 2026-09-30)
- `Flxx94/patch-1` (PR #27, 2026-11-04)
- Security/fix batch PRs #56–#66 (2026-11-06 … 2026-11-07)
- Stats/gallery PRs #65, #66 (2026-11-07)

Branch naming conventions observed: `fix/*`, `docs/*`, `ci/*`, `feat`-ish
descriptive names, plus personal forks (`Catrick-cpu/patch-*`).

---

## 8. Activity statistics

### 8.1 Commits per month

| Month | Commits |
|-------|--------:|
| 2025-11 | 22 |
| 2025-12 | 3 |
| 2026-01 | 10 |
| 2026-02 | 37 |
| 2026-03 | 12 |
| 2026-04 | 1 |
| 2026-05 | 29 |
| 2026-06 | 26 |
| 2026-07 | 12 |
| 2026-09 | 45 |
| 2026-10 | 24 |
| 2026-11 | 76 |

### 8.2 Commits by weekday

| Day | Commits |
|-----|--------:|
| Monday | 30 |
| Tuesday | 30 |
| Wednesday | 47 |
| Thursday | 46 |
| Friday | 89 |
| Saturday | 45 |
| Sunday | 10 |

### 8.3 Busiest days

| Date | Commits |
|------|--------:|
| 2026-11-06 | 29 |
| 2026-09-16 | 14 |
| 2026-02-14 | 14 |
| 2026-05-28 | 13 |
| 2026-05-27 | 11 |
| 2026-11-19 | 10 |
| 2026-11-17 | 10 |
| 2025-11-15 | 10 |

### 8.4 File types added (cumulative)

| Extension | Files added |
|-----------|------------:|
| `.rs` | 39 |
| `.py` | 31 |
| `.go` | 14 |
| `.js` | 252 |
| `.html` | 8 |
| `.sh` | 8 |

### 8.5 Tags

| Tag | Commit | Date | Note |
|-----|--------|------|------|
| `v1.1.0-beta` | `0ea513f` | 2026-02-16 | License update |
| `v.1.1.2-backup` | `cae5629` | 2026-04-17 | Server update |
| `backup` | `d5e692f` | 2026-10-23 | Image-builder feature |
| `backup@18.11.16` | — | 2026-11-16 | Backup snapshot |

---

## 9. Notable commits & honest retrospectives

A few commit messages deserve to be quoted verbatim — they document real
friction points better than any summary could:

- `76966de` (2026-02-14) **"Removed unittest -> to complicate"** — the first
  attempt at unit testing was abandoned.
- `79099bc` (2026-10-21) **"made cargo deny warnings to enforce clean code,
  THIS PUSH CURRENTLY DOES NOT COMPILE!!!"** — followed by `acda1c0`
  "all cargo errors resolved (promoted warnings)".
- `a32eacf` (2026-11-06) **"fixed testing bug (vscode lsp heult immernoch
  aber scheiss drauf, cargo ist zufrieden oder so -_-)"** — tooling pain.
- `81980e5` (2026-11-06) **"completed test suite and lack of sleep 0_o"**.
- `93473f1` (2026-11-07) **"removed patricks ai slop from readme"**.
- `0b8f223` (2026-01-22) **"Keine Ahnung was das ist"** ("no idea what this is").
- `1dda116` (2026-03-26) **"!"** — minimal commit message.
- `69c15a5` (2026-09-23) **"*"** — same pattern.
- `e4ccde8` (2026-01-23) **"Revert 'httpserver code mergen'"** — a merge was
  attempted, reverted, then re-attempted (`071db82`).

These are valuable signals: the project grew organically, sometimes messily,
and the later eras (Era 8/9) explicitly paid down that debt with tests, CI and
hardening.

---

## 10. How this document was generated

This file is a manual synthesis of the Git history, but every fact (dates,
counts, commit hashes, authors, file lists) was obtained with standard Git
commands, e.g.:

```bash
git log --reverse --format="%h|%ad|%an|%s" --date=short
git shortlog -sne --all
git log --format="%ad" --date=format:"%Y-%m" | sort | uniq -c
git log --merges --format="%ad|%s" --date=short
git log --diff-filter=A --name-only --format="" | sort | uniq -c
git log --format="%ad" --date=format:"%A" | sort | uniq -c
```

To regenerate or extend this document, re-run those commands against the
current `main` and update the tables. The raw data (commit graph, branches,
tags) is the single source of truth; this Markdown is a snapshot.

---

*Generated from the Git history of `Roboterarm-HASE` on 2026-11-20.*
