#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "$0")" && pwd)/common.sh"

OUTPUT_DIR="$REPO_ROOT/examples/visual"
OUTPUT_HTML="$OUTPUT_DIR/index.html"

capture_cmd() {
  local repo="$1"
  shift
  (
    cd "$repo"
    "$@" 2>&1
  )
}

capture_cmd_allow_failure() {
  local repo="$1"
  shift
  local output
  local status

  set +e
  output="$(
    cd "$repo"
    "$@" 2>&1
  )"
  status=$?
  set -e

  printf '%s\n[exit %s]\n' "$output" "$status"
}

read_file() {
  local path="$1"
  if [[ -f "$path" ]]; then
    sed -n '1,220p' "$path"
  fi
}

ensure_hj_built
create_demo_repo
repo="$DEMO_REPO"

refresh_output="$(capture_cmd "$repo" "$HJ_BIN" refresh)"
detect_output="$(capture_cmd_allow_failure "$repo" "$HJ_BIN" detect)"
handoff_output="$(
  capture_cmd \
    "$repo" \
    "$HJ_BIN" \
    handoff \
    --allow-create \
    --build clean \
    --tests passing \
    --notes "Visual demo repo." \
    --log-summary "Bootstrap visual demo state"
)"

handoff_path="$(
  cd "$repo"
  "$HJ_BIN" detect
)"

cat >"$handoff_path" <<'EOF'
project: demo-app
id: demoapp
updated: 2026-04-16
items:
  - id: demoapp-1
    name: wire-status-source
    priority: P1
    status: open
    title: Wire a real status source
    description: Replace the placeholder status function with repository-backed state.
    files:
      - src/lib.rs
  - id: demoapp-2
    name: fix-handup-copy
    priority: P0
    status: blocked
    title: Fix handup copy for onboarding
    description: Waiting on a final wording pass before the onboarding path can ship.
    files:
      - README.md
      - .ctx/HANDOVER.md
  - id: demoapp-3
    priority: P2
    status: open
    title: Add sqlite smoke coverage
    description: Cover the basic closeout path against a disposable sqlite file.
    files:
      - crates/hj-sqlite/src/lib.rs
log:
  - date: 2026-04-16
    summary: Bootstrap visual demo state
    commits:
      - abc1234
EOF

handover_output="$(capture_cmd "$repo" "$HJ_BIN" handover)"
handon_output="$(capture_cmd "$repo" "$HJ_BIN" handon)"
handup_output="$(capture_cmd "$repo" "$HJ_BIN" handup --max-depth 3)"

rendered_handover="$(read_file "$repo/.ctx/HANDOVER.md")"
rendered_handup="$(read_file "$HOME/.ctx/handoffs/$(basename "$repo")/HANDUP.json")"
rendered_handoff="$(read_file "$handoff_path")"

mkdir -p "$OUTPUT_DIR"

export VISUAL_REPO="$repo"
export VISUAL_OUTPUT_HTML="$OUTPUT_HTML"
export VISUAL_DETECT_OUTPUT="$detect_output"
export VISUAL_REFRESH_OUTPUT="$refresh_output"
export VISUAL_HANDOFF_OUTPUT="$handoff_output"
export VISUAL_HANDON_OUTPUT="$handon_output"
export VISUAL_HANDOVER_OUTPUT="$handover_output"
export VISUAL_HANDUP_OUTPUT="$handup_output"
export VISUAL_RENDERED_HANDOFF="$rendered_handoff"
export VISUAL_RENDERED_HANDOVER="$rendered_handover"
export VISUAL_RENDERED_HANDUP="$rendered_handup"
export VISUAL_HANDOFF_NAME="$(basename "$handoff_path")"

python3 <<'PY'
import html
import os
from pathlib import Path

def esc(name: str) -> str:
    return html.escape(os.environ[name])

repo = os.environ["VISUAL_REPO"]
output_html = Path(os.environ["VISUAL_OUTPUT_HTML"])
handoff_name = os.environ["VISUAL_HANDOFF_NAME"]

page = f"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>hj Visual Demo</title>
  <style>
    :root {{
      --bg: #f4efe6;
      --bg-strong: #efe4d1;
      --surface: rgba(255, 251, 245, 0.88);
      --surface-strong: rgba(255, 248, 238, 0.98);
      --ink: #1e2430;
      --muted: #5b6577;
      --line: rgba(42, 58, 82, 0.14);
      --accent: #ba5a2a;
      --accent-soft: rgba(186, 90, 42, 0.12);
      --accent-2: #1f6b5c;
      --shadow: 0 20px 60px rgba(44, 31, 18, 0.12);
      --radius: 22px;
      --mono: "SFMono-Regular", "JetBrains Mono", "Cascadia Code", Menlo, monospace;
      --serif: "Iowan Old Style", "Palatino Linotype", "Book Antiqua", Georgia, serif;
      --sans: "Avenir Next", "Segoe UI", "Helvetica Neue", sans-serif;
    }}

    * {{
      box-sizing: border-box;
    }}

    body {{
      margin: 0;
      font-family: var(--sans);
      color: var(--ink);
      background:
        radial-gradient(circle at top left, rgba(186, 90, 42, 0.22), transparent 34%),
        radial-gradient(circle at top right, rgba(31, 107, 92, 0.18), transparent 28%),
        linear-gradient(180deg, #f7f1e8 0%, #f1eadf 45%, #ece4d7 100%);
    }}

    .shell {{
      width: min(1180px, calc(100vw - 32px));
      margin: 0 auto;
      padding: 32px 0 72px;
    }}

    .hero,
    .panel,
    .card {{
      background: var(--surface);
      backdrop-filter: blur(10px);
      border: 1px solid var(--line);
      border-radius: var(--radius);
      box-shadow: var(--shadow);
    }}

    .hero {{
      overflow: hidden;
      position: relative;
      padding: 32px;
    }}

    .hero::after {{
      content: "";
      position: absolute;
      inset: auto -80px -90px auto;
      width: 260px;
      height: 260px;
      border-radius: 50%;
      background: radial-gradient(circle, rgba(186, 90, 42, 0.24), transparent 70%);
      pointer-events: none;
    }}

    .eyebrow {{
      display: inline-flex;
      gap: 8px;
      align-items: center;
      padding: 7px 12px;
      border-radius: 999px;
      background: var(--accent-soft);
      color: var(--accent);
      letter-spacing: 0.08em;
      text-transform: uppercase;
      font-size: 12px;
      font-weight: 700;
    }}

    h1,
    h2,
    h3 {{
      margin: 0;
      font-family: var(--serif);
      font-weight: 700;
      line-height: 1.02;
    }}

    h1 {{
      margin-top: 18px;
      font-size: clamp(2.7rem, 6vw, 5.5rem);
      max-width: 10ch;
    }}

    .lede {{
      margin: 18px 0 0;
      max-width: 58rem;
      color: var(--muted);
      font-size: 1.05rem;
      line-height: 1.65;
    }}

    .meta {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
      gap: 14px;
      margin-top: 24px;
    }}

    .meta .chip {{
      padding: 14px 16px;
      border-radius: 18px;
      background: var(--surface-strong);
      border: 1px solid var(--line);
      font-size: 0.95rem;
    }}

    .layout {{
      display: grid;
      grid-template-columns: 320px minmax(0, 1fr);
      gap: 20px;
      margin-top: 22px;
    }}

    .panel {{
      padding: 24px;
    }}

    .flow {{
      display: grid;
      gap: 12px;
      margin-top: 18px;
    }}

    .flow-step {{
      position: relative;
      padding: 14px 14px 14px 48px;
      border-radius: 16px;
      background: var(--surface-strong);
      border: 1px solid var(--line);
    }}

    .flow-step::before {{
      content: attr(data-step);
      position: absolute;
      left: 14px;
      top: 14px;
      width: 24px;
      height: 24px;
      display: grid;
      place-items: center;
      border-radius: 50%;
      background: var(--accent);
      color: white;
      font-size: 12px;
      font-weight: 700;
    }}

    .flow-step strong {{
      display: block;
      font-size: 0.95rem;
    }}

    .flow-step span {{
      display: block;
      margin-top: 4px;
      color: var(--muted);
      font-size: 0.92rem;
      line-height: 1.45;
    }}

    .content {{
      display: grid;
      gap: 18px;
    }}

    .card {{
      overflow: hidden;
      animation: rise 420ms ease both;
    }}

    .card:nth-child(2) {{ animation-delay: 60ms; }}
    .card:nth-child(3) {{ animation-delay: 120ms; }}
    .card:nth-child(4) {{ animation-delay: 180ms; }}
    .card:nth-child(5) {{ animation-delay: 240ms; }}
    .card:nth-child(6) {{ animation-delay: 300ms; }}

    @keyframes rise {{
      from {{
        opacity: 0;
        transform: translateY(18px);
      }}
      to {{
        opacity: 1;
        transform: translateY(0);
      }}
    }}

    .card-header {{
      display: flex;
      justify-content: space-between;
      gap: 16px;
      align-items: end;
      padding: 20px 22px 0;
    }}

    .card-header p {{
      margin: 10px 0 0;
      color: var(--muted);
      line-height: 1.55;
    }}

    .badge {{
      white-space: nowrap;
      align-self: start;
      padding: 8px 10px;
      border-radius: 999px;
      background: rgba(31, 107, 92, 0.12);
      color: var(--accent-2);
      font-weight: 700;
      font-size: 12px;
      letter-spacing: 0.04em;
      text-transform: uppercase;
    }}

    .grid {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
      gap: 16px;
      padding: 20px 22px 24px;
    }}

    .terminal,
    .artifact {{
      min-height: 100%;
      border-radius: 18px;
      overflow: hidden;
      border: 1px solid rgba(23, 28, 38, 0.12);
      background: #10151d;
      color: #d6dfef;
    }}

    .artifact {{
      background: #fbf8f2;
      color: var(--ink);
    }}

    .window-bar {{
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
      padding: 10px 14px;
      font-family: var(--mono);
      font-size: 12px;
      border-bottom: 1px solid rgba(255, 255, 255, 0.08);
      background: rgba(255, 255, 255, 0.05);
    }}

    .artifact .window-bar {{
      border-bottom: 1px solid rgba(23, 28, 38, 0.08);
      background: rgba(186, 90, 42, 0.07);
    }}

    pre {{
      margin: 0;
      padding: 16px;
      overflow-x: auto;
      font-family: var(--mono);
      font-size: 12.5px;
      line-height: 1.55;
      white-space: pre-wrap;
      word-break: break-word;
    }}

    .footer {{
      margin-top: 22px;
      padding: 18px 22px;
      color: var(--muted);
      font-size: 0.94rem;
      line-height: 1.6;
    }}

    code {{
      font-family: var(--mono);
      font-size: 0.94em;
    }}

    @media (max-width: 920px) {{
      .layout {{
        grid-template-columns: 1fr;
      }}
    }}
  </style>
</head>
<body>
  <main class="shell">
    <section class="hero">
      <div class="eyebrow">Standalone walkthrough</div>
      <h1>hj, rendered as a working flow.</h1>
      <p class="lede">
        This page was generated from a disposable demo repo using the local binaries from this checkout.
        It shows the core path from discovery to closeout artifacts, with real command output and real files.
      </p>
      <div class="meta">
        <div class="chip"><strong>Regenerate</strong><br><code>./examples/visual-demo.sh</code></div>
        <div class="chip"><strong>Repo root</strong><br><code>{html.escape(repo)}</code></div>
        <div class="chip"><strong>Output page</strong><br><code>{html.escape(str(output_html))}</code></div>
      </div>
    </section>

    <section class="layout">
      <aside class="panel">
        <div class="eyebrow">Flow</div>
        <h2 style="margin-top: 16px; font-size: 2rem;">Command path</h2>
        <div class="flow">
          <div class="flow-step" data-step="1">
            <strong>Detect</strong>
            <span>Resolve the active handoff location inside the disposable repo.</span>
          </div>
          <div class="flow-step" data-step="2">
            <strong>Refresh</strong>
            <span>Create the managed <code>.ctx</code> layout and ignore rules.</span>
          </div>
          <div class="flow-step" data-step="3">
            <strong>Handoff</strong>
            <span>Persist state, markdown renders, and closeout metadata.</span>
          </div>
          <div class="flow-step" data-step="4">
            <strong>Handon</strong>
            <span>Wake up into triage grouped by priority and status.</span>
          </div>
          <div class="flow-step" data-step="5">
            <strong>Handover</strong>
            <span>Generate the compact markdown summary for the next session.</span>
          </div>
          <div class="flow-step" data-step="6">
            <strong>Handup</strong>
            <span>Survey the repo tree and emit a machine-readable checkpoint.</span>
          </div>
        </div>
      </aside>

      <div class="content">
        <section class="card">
          <div class="card-header">
            <div>
              <div class="eyebrow">Step 1</div>
              <h3 style="margin-top: 14px; font-size: 2rem;">Detect and refresh</h3>
              <p>The first two commands establish where state lives and ensure the repo has managed scaffolding. Before the first handoff file exists, <code>hj detect</code> still prints the managed path and exits <code>2</code> to signal that the file is not there yet.</p>
            </div>
            <div class="badge">Project bootstrap</div>
          </div>
          <div class="grid">
            <div class="terminal">
              <div class="window-bar">Terminal<div><code>hj detect</code></div></div>
              <pre>{esc("VISUAL_DETECT_OUTPUT")}</pre>
            </div>
            <div class="terminal">
              <div class="window-bar">Terminal<div><code>hj refresh</code></div></div>
              <pre>{esc("VISUAL_REFRESH_OUTPUT")}</pre>
            </div>
          </div>
        </section>

        <section class="card">
          <div class="card-header">
            <div>
              <div class="eyebrow">Step 2</div>
              <h3 style="margin-top: 14px; font-size: 2rem;">Close out with handoff</h3>
              <p>The demo bootstraps the handoff files, then injects a few open and blocked items so the later views are interesting.</p>
            </div>
            <div class="badge">State write</div>
          </div>
          <div class="grid">
            <div class="terminal">
              <div class="window-bar">Terminal<div><code>hj handoff --allow-create ...</code></div></div>
              <pre>{esc("VISUAL_HANDOFF_OUTPUT")}</pre>
            </div>
            <div class="artifact">
              <div class="window-bar">Artifact<div><code>{html.escape(handoff_name)}</code></div></div>
              <pre>{esc("VISUAL_RENDERED_HANDOFF")}</pre>
            </div>
          </div>
        </section>

        <section class="card">
          <div class="card-header">
            <div>
              <div class="eyebrow">Step 3</div>
              <h3 style="margin-top: 14px; font-size: 2rem;">Wake into triage</h3>
              <p><code>hj handon</code> shows the actionable queue the next time someone picks the repo back up.</p>
            </div>
            <div class="badge">Operator view</div>
          </div>
          <div class="grid">
            <div class="terminal">
              <div class="window-bar">Terminal<div><code>hj handon</code></div></div>
              <pre>{esc("VISUAL_HANDON_OUTPUT")}</pre>
            </div>
          </div>
        </section>

        <section class="card">
          <div class="card-header">
            <div>
              <div class="eyebrow">Step 4</div>
              <h3 style="margin-top: 14px; font-size: 2rem;">Generate the handover brief</h3>
              <p><code>hj handover</code> distills the current state into a compact markdown summary meant for the next person or next session.</p>
            </div>
            <div class="badge">Compact summary</div>
          </div>
          <div class="grid">
            <div class="terminal">
              <div class="window-bar">Terminal<div><code>hj handover</code></div></div>
              <pre>{esc("VISUAL_HANDOVER_OUTPUT")}</pre>
            </div>
            <div class="artifact">
              <div class="window-bar">Artifact<div><code>.ctx/HANDOVER.md</code></div></div>
              <pre>{esc("VISUAL_RENDERED_HANDOVER")}</pre>
            </div>
          </div>
        </section>

        <section class="card">
          <div class="card-header">
            <div>
              <div class="eyebrow">Step 5</div>
              <h3 style="margin-top: 14px; font-size: 2rem;">Survey with handup</h3>
              <p><code>hj handup</code> gives a broader, machine-readable checkpoint for repo scanning and orchestration.</p>
            </div>
            <div class="badge">Tree survey</div>
          </div>
          <div class="grid">
            <div class="terminal">
              <div class="window-bar">Terminal<div><code>hj handup --max-depth 3</code></div></div>
              <pre>{esc("VISUAL_HANDUP_OUTPUT")}</pre>
            </div>
            <div class="artifact">
              <div class="window-bar">Artifact<div><code>HANDUP.json</code></div></div>
              <pre>{esc("VISUAL_RENDERED_HANDUP")}</pre>
            </div>
          </div>
        </section>

        <section class="panel footer">
          The remaining command-specific demos still live under <code>examples/commands/</code>, including
          <code>reconcile</code>, <code>audit</code>, <code>install</code>, and <code>update</code>.
          This visual page focuses on the core local workflow that works without external services.
        </section>
      </div>
    </section>
  </main>
</body>
</html>
"""

output_html.write_text(page, encoding="utf-8")
PY

printf 'generated %s\n' "$OUTPUT_HTML"
