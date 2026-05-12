#!/usr/bin/env nu

# Generates examples/visual/index.html from a disposable demo repo.
# Usage: nu examples/visual-demo.nu

let examples_root = ($env.CURRENT_FILE | path dirname)
let repo_root = ($examples_root | path join "..") | path expand
let hj_bin = $repo_root | path join "target" "debug" "hj"
let output_dir = $examples_root | path join "visual"
let output_html = $output_dir | path join "index.html"

# Build hj
print "building hj..."
do {
    ^cargo build --manifest-path ($repo_root | path join "Cargo.toml") -p hjx --bins
} | complete | ignore

# Create a disposable demo repo with a fake HOME so hj state stays isolated
let demo_root = (^mktemp -d | str trim)
let demo_home = $demo_root | path join "home"
let repo = $demo_root | path join "demo-app"

mkdir $demo_home
mkdir ($repo | path join "src")

# Cargo.toml
["[package]" "name = \"demo-app\"" "version = \"0.1.0\"" "edition = \"2024\"" ""]
| str join (char newline)
| save ($repo | path join "Cargo.toml")

# src/lib.rs
["pub fn status() -> &'static str {" "    // TODO: wire a real status source" "    \"ok\"" "}" ""]
| str join (char newline)
| save ($repo | path join "src" "lib.rs")

^git init -q $repo
^git -C $repo config user.name "hj examples"
^git -C $repo config user.email "examples@local"
^git -C $repo add .
^git -C $repo commit -qm "init demo repo"

# Helper: run hj in the demo repo under the isolated home
def capture [subcmd: list<string>] {
    do { with-env { HOME: $demo_home } { ^$hj_bin ...$subcmd } } | complete
}

def capture_out [subcmd: list<string>] {
    let r = capture $subcmd
    $r.stdout + $r.stderr
}

def capture_allow_fail [subcmd: list<string>] {
    let r = capture $subcmd
    if $r.exit_code == 0 {
        $r.stdout
    } else {
        $"($r.stdout)($r.stderr)\n[exit ($r.exit_code)]"
    }
}

# --- Collect outputs ---
let refresh_output = capture_out ["refresh"]
let detect_output  = capture_allow_fail ["detect"]

let handoff_output = capture_out [
    "handoff" "--allow-create"
    "--build" "clean"
    "--tests" "passing"
    "--notes" "Visual demo repo."
    "--log-summary" "Bootstrap visual demo state"
]

# Inject richer demo state directly into the handoff file
let handoff_path = (with-env { HOME: $demo_home } { ^$hj_bin detect } | complete).stdout | str trim

[
    "project: demo-app"
    "id: demoapp"
    "updated: 2026-04-16"
    "items:"
    "  - id: demoapp-1"
    "    name: wire-status-source"
    "    priority: P1"
    "    status: open"
    "    title: Wire a real status source"
    "    description: Replace the placeholder status function with repository-backed state."
    "    files:"
    "      - src/lib.rs"
    "  - id: demoapp-2"
    "    name: fix-handup-copy"
    "    priority: P0"
    "    status: blocked"
    "    title: Fix handup copy for onboarding"
    "    description: Waiting on a final wording pass before the onboarding path can ship."
    "    files:"
    "      - README.md"
    "      - .ctx/HANDOVER.md"
    "  - id: demoapp-3"
    "    priority: P2"
    "    status: open"
    "    title: Add sqlite smoke coverage"
    "    description: Cover the basic closeout path against a disposable sqlite file."
    "    files:"
    "      - crates/hjlib/src/sqlite.rs"
    "log:"
    "  - date: 2026-04-16"
    "    summary: Bootstrap visual demo state"
    "    commits:"
    "      - abc1234"
    ""
]
| str join (char newline)
| save --force $handoff_path

let handover_output = capture_out ["handover"]
let handon_output   = capture_out ["handon"]
let handup_output   = capture_out ["handup" "--max-depth" "3"]

# Read generated artifacts
def read_artifact [path: string] {
    if ($path | path exists) { open --raw $path } else { "" }
}

let rendered_handoff  = read_artifact $handoff_path
let rendered_handover = read_artifact ($repo | path join ".ctx" "HANDOVER.md")
let handup_json_path  = $demo_home | path join ".ctx" "handoffs" ($repo | path basename) "HANDUP.json"
let rendered_handup   = read_artifact $handup_json_path
let handoff_name      = $handoff_path | path basename

# HTML-escape
def esc [s: string] {
    $s
    | str replace --all "&"  "&amp;"
    | str replace --all "<"  "&lt;"
    | str replace --all ">"  "&gt;"
    | str replace --all "\"" "&quot;"
}

# HTML template — plain "..." string (no $ prefix = no interpolation).
# Embedded " are escaped as \". Substitution markers are @@KEY@@.
let template = "<!doctype html>
<html lang=\"en\">
<head>
  <meta charset=\"utf-8\">
  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
  <title>hj Visual Demo</title>
  <style>
    :root {
      --bg: #0d1117;
      --surface: #161b22;
      --surface-raised: #1c2330;
      --ink: #cdd9e5;
      --muted: #768390;
      --line: rgba(205, 217, 229, 0.08);
      --accent: #e8834a;
      --accent-soft: rgba(232, 131, 74, 0.12);
      --accent-2: #4caf8a;
      --shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
      --radius: 14px;
      --mono: \"SFMono-Regular\", \"JetBrains Mono\", Menlo, monospace;
      --serif: \"Iowan Old Style\", \"Palatino Linotype\", Georgia, serif;
      --sans: \"Avenir Next\", \"Segoe UI\", \"Helvetica Neue\", sans-serif;
    }
    * { box-sizing: border-box; }
    body {
      margin: 0; font-family: var(--sans); color: var(--ink); background: var(--bg);
    }
    .shell { width: min(1180px, calc(100vw - 32px)); margin: 0 auto; padding: 32px 0 72px; }
    .hero, .panel, .card {
      background: var(--surface);
      border: 1px solid var(--line); border-radius: var(--radius); box-shadow: var(--shadow);
    }
    .hero { overflow: hidden; position: relative; padding: 32px; }
    .eyebrow {
      display: inline-flex; gap: 8px; align-items: center;
      padding: 5px 10px; border-radius: 999px;
      background: var(--accent-soft); color: var(--accent);
      letter-spacing: 0.08em; text-transform: uppercase; font-size: 11px; font-weight: 700;
    }
    h1, h2, h3 { margin: 0; font-family: var(--serif); font-weight: 700; line-height: 1.02; }
    h1 { margin-top: 18px; font-size: clamp(2.2rem, 5vw, 4rem); max-width: 14ch; color: var(--ink); }
    .lede { margin: 14px 0 0; max-width: 58rem; color: var(--muted); font-size: 0.97rem; line-height: 1.65; }
    .meta { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 10px; margin-top: 20px; }
    .meta .chip { padding: 10px 14px; border-radius: 10px; background: var(--surface-raised); border: 1px solid var(--line); font-size: 0.88rem; color: var(--muted); }
    .meta .chip strong { color: var(--ink); display: block; margin-bottom: 2px; font-size: 0.8rem; text-transform: uppercase; letter-spacing: 0.06em; }
    .layout { display: grid; grid-template-columns: 240px minmax(0, 1fr); gap: 16px; margin-top: 16px; }
    .panel { padding: 20px; }
    .cmd-list { margin-top: 16px; display: flex; flex-direction: column; gap: 2px; }
    .cmd-item { display: flex; flex-direction: column; gap: 3px; padding: 8px 8px; border-radius: 8px; }
    .cmd-item:hover { background: var(--surface-raised); }
    .cmd-item code { font-family: var(--mono); font-size: 12px; color: var(--accent); }
    .cmd-item span { font-size: 0.8rem; color: var(--muted); line-height: 1.45; }
    .content { display: grid; gap: 14px; }
    .card { overflow: hidden; animation: rise 380ms ease both; }
    .card:nth-child(2) { animation-delay: 50ms; }
    .card:nth-child(3) { animation-delay: 100ms; }
    .card:nth-child(4) { animation-delay: 150ms; }
    .card:nth-child(5) { animation-delay: 200ms; }
    .card:nth-child(6) { animation-delay: 250ms; }
    @keyframes rise { from { opacity: 0; transform: translateY(12px); } to { opacity: 1; transform: translateY(0); } }
    .card-header { display: flex; justify-content: space-between; gap: 16px; align-items: start; padding: 18px 20px 0; }
    .card-header h3 { margin-top: 10px; font-size: 1.3rem; color: var(--ink); }
    .card-header p { margin: 8px 0 0; color: var(--muted); line-height: 1.55; font-size: 0.9rem; }
    .badge { white-space: nowrap; padding: 5px 9px; border-radius: 999px; background: rgba(76, 175, 138, 0.12); color: var(--accent-2); font-weight: 700; font-size: 11px; letter-spacing: 0.05em; text-transform: uppercase; }
    .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 12px; padding: 16px 20px 20px; }
    .terminal, .artifact { min-height: 100%; border-radius: 10px; overflow: hidden; border: 1px solid var(--line); background: #090d13; color: #adbac7; }
    .artifact { background: var(--surface-raised); color: var(--ink); }
    .window-bar { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 8px 12px; font-family: var(--mono); font-size: 11px; color: var(--muted); border-bottom: 1px solid var(--line); }
    .artifact .window-bar { color: var(--accent); }
    pre { margin: 0; padding: 14px; overflow-x: auto; font-family: var(--mono); font-size: 12px; line-height: 1.6; white-space: pre-wrap; word-break: break-word; }
    .footer { margin-top: 14px; padding: 16px 20px; color: var(--muted); font-size: 0.88rem; line-height: 1.6; }
    code { font-family: var(--mono); font-size: 0.92em; }
    @media (max-width: 860px) { .layout { grid-template-columns: 1fr; } }
  </style>
</head>
<body>
  <main class=\"shell\">
    <section class=\"hero\">
      <div class=\"eyebrow\">Visual walkthrough</div>
      <h1>hj — handoff in practice.</h1>
      <p class=\"lede\">
        Generated from a disposable demo repo using the local binary from this checkout.
        Real command output, real files.
      </p>
      <div class=\"meta\">
        <div class=\"chip\"><strong>Regenerate</strong><code>nu examples/visual-demo.nu</code></div>
        <div class=\"chip\"><strong>Demo repo</strong><code>@@REPO@@</code></div>
        <div class=\"chip\"><strong>Output</strong><code>@@OUTPUT_HTML@@</code></div>
      </div>
    </section>

    <section class=\"layout\">
      <aside class=\"panel\">
        <p style=\"margin: 0; font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--muted);\">Commands</p>
        <div class=\"cmd-list\">
          <div class=\"cmd-item\"><code>detect</code><span>Print the expected handoff file path for the current repo, exiting 2 if it does not exist yet.</span></div>
          <div class=\"cmd-item\"><code>refresh</code><span>Create or repair the <code>.ctx/</code> directory layout and add gitignore rules for state files.</span></div>
          <div class=\"cmd-item\"><code>handoff</code><span>Record session metadata (build status, test status, notes) and append a log entry to the handoff file.</span></div>
          <div class=\"cmd-item\"><code>handon</code><span>Print the open and blocked items from the handoff file, grouped by priority, ready for triage.</span></div>
          <div class=\"cmd-item\"><code>handover</code><span>Render a concise markdown brief from the current handoff state and write it to <code>.ctx/HANDOVER.md</code>.</span></div>
          <div class=\"cmd-item\"><code>handup</code><span>Walk the directory tree, collect handoff state from each repo found, and emit a machine-readable JSON summary.</span></div>
        </div>
      </aside>

      <div class=\"content\">
        <section class=\"card\">
          <div class=\"card-header\">
            <div>
              <h3>Detect and refresh</h3>
              <p><code>hj detect</code> resolves the canonical handoff file path for the current repo
              and exits <code>2</code> when the file does not exist yet — useful for scripting and
              conditional bootstrap. <code>hj refresh</code> creates the <code>.ctx/</code> directory
              structure and writes gitignore rules so state files are never accidentally committed.</p>
            </div>
            <div class=\"badge\">bootstrap</div>
          </div>
          <div class=\"grid\">
            <div class=\"terminal\"><div class=\"window-bar\">Terminal<div><code>hj detect</code></div></div><pre>@@DETECT@@</pre></div>
            <div class=\"terminal\"><div class=\"window-bar\">Terminal<div><code>hj refresh</code></div></div><pre>@@REFRESH@@</pre></div>
          </div>
        </section>

        <section class=\"card\">
          <div class=\"card-header\">
            <div>
              <h3>Close out with handoff</h3>
              <p><code>hj handoff</code> appends a timestamped log entry capturing build status, test
              status, and free-form notes. Pass <code>--allow-create</code> on first use to write the
              file from scratch. The demo injects a few open and blocked items directly so the triage
              and handover views have something interesting to show.</p>
            </div>
            <div class=\"badge\">state write</div>
          </div>
          <div class=\"grid\">
            <div class=\"terminal\"><div class=\"window-bar\">Terminal<div><code>hj handoff --allow-create ...</code></div></div><pre>@@HANDOFF_OUT@@</pre></div>
            <div class=\"artifact\"><div class=\"window-bar\">Artifact<div><code>@@HANDOFF_NAME@@</code></div></div><pre>@@HANDOFF_FILE@@</pre></div>
          </div>
        </section>

        <section class=\"card\">
          <div class=\"card-header\">
            <div>
              <h3>Wake into triage</h3>
              <p><code>hj handon</code> reads the handoff file and prints open and blocked items sorted
              by priority. Run it at the start of a session to see exactly what needs attention before
              touching any code.</p>
            </div>
            <div class=\"badge\">operator view</div>
          </div>
          <div class=\"grid\">
            <div class=\"terminal\"><div class=\"window-bar\">Terminal<div><code>hj handon</code></div></div><pre>@@HANDON@@</pre></div>
          </div>
        </section>

        <section class=\"card\">
          <div class=\"card-header\">
            <div>
              <h3>Generate the handover brief</h3>
              <p><code>hj handover</code> renders the handoff YAML into a compact markdown document
              written to <code>.ctx/HANDOVER.md</code>. It is designed to be pasted into a PR
              description, a chat message, or a commit body — a human-readable snapshot that requires
              no tooling to read.</p>
            </div>
            <div class=\"badge\">compact summary</div>
          </div>
          <div class=\"grid\">
            <div class=\"terminal\"><div class=\"window-bar\">Terminal<div><code>hj handover</code></div></div><pre>@@HANDOVER_OUT@@</pre></div>
            <div class=\"artifact\"><div class=\"window-bar\">Artifact<div><code>.ctx/HANDOVER.md</code></div></div><pre>@@HANDOVER_FILE@@</pre></div>
          </div>
        </section>

        <section class=\"card\">
          <div class=\"card-header\">
            <div>
              <h3>Survey with handup</h3>
              <p><code>hj handup</code> walks the directory tree up to <code>--max-depth</code> levels,
              finds every repo with a handoff file, and emits a JSON summary of their state. Useful for
              cross-repo dashboards, agent orchestration, and morning triage across a whole workspace.</p>
            </div>
            <div class=\"badge\">tree survey</div>
          </div>
          <div class=\"grid\">
            <div class=\"terminal\"><div class=\"window-bar\">Terminal<div><code>hj handup --max-depth 3</code></div></div><pre>@@HANDUP_OUT@@</pre></div>
            <div class=\"artifact\"><div class=\"window-bar\">Artifact<div><code>HANDUP.json</code></div></div><pre>@@HANDUP_FILE@@</pre></div>
          </div>
        </section>

        <section class=\"panel footer\">
          Command-specific examples live under <code>examples/commands/</code> --
          covering <code>reconcile</code>, <code>audit</code>, <code>install</code>, <code>update</code>, and more.
        </section>
      </div>
    </section>
  </main>
</body>
</html>"

mkdir $output_dir

$template
| str replace --all "@@REPO@@"          (esc $repo)
| str replace --all "@@OUTPUT_HTML@@"   (esc $output_html)
| str replace --all "@@DETECT@@"        (esc $detect_output)
| str replace --all "@@REFRESH@@"       (esc $refresh_output)
| str replace --all "@@HANDOFF_OUT@@"   (esc $handoff_output)
| str replace --all "@@HANDOFF_NAME@@"  (esc $handoff_name)
| str replace --all "@@HANDOFF_FILE@@"  (esc $rendered_handoff)
| str replace --all "@@HANDON@@"        (esc $handon_output)
| str replace --all "@@HANDOVER_OUT@@"  (esc $handover_output)
| str replace --all "@@HANDOVER_FILE@@" (esc $rendered_handover)
| str replace --all "@@HANDUP_OUT@@"    (esc $handup_output)
| str replace --all "@@HANDUP_FILE@@"   (esc $rendered_handup)
| save --force $output_html

print $"generated ($output_html)"
