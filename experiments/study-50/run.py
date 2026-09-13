#!/usr/bin/env python3
"""Study-50 execution driver.

For each task dir (tasks/<id>/): copy the base repo, apply the "agent's"
fix (fixture patch or a coding-agent CLI), run `uni verify contract.uni --json`,
append one CSV row, then aggregate with collect.py.

Agents:
  --agent fixture   apply tasks/<id>/fix.patch (ground truth recorded offline)
  --agent codex     call `codex exec` in the base copy (wire-in point)
  --agent claude    call `claude -p` in the base copy (wire-in point)

Usage:
  python3 run.py --tasks tasks --agent fixture --out results.csv [--keep] [--only t01]
"""
import argparse
import csv
import json
import os
import shutil
import subprocess
import sys
import tempfile

UNI = os.environ.get(
    "UNI_BIN",
    os.path.join(os.path.dirname(__file__), "..", "..", "target", "release", "uni"),
)

REGISTRY = ''

def sh(args, cwd, capture=True):
    p = subprocess.run(args, cwd=cwd, text=True,
                       stdout=subprocess.PIPE if capture else None,
                       stderr=subprocess.PIPE if capture else None)
    return p.returncode, (p.stdout or ""), (p.stderr or "")

def agent_fixture(task, work):
    patch = os.path.abspath(os.path.join(task, "fix.patch"))
    code, out, err = sh(["git", "apply", "--whitespace=nowarn", patch], work)
    if code != 0:
        print(f"  [agent] patch failed in {task}: {out}{err}", file=sys.stderr)
    sh(["git", "-c", "user.email=s@t", "-c", "user.name=s", "commit", "-qam", "agent-fix"], work)
    return code == 0

def agent_codex(task, work):
    code, out, err = sh(["codex", "exec", "implement the issue described in issue.md"], work)
    if code != 0:
        print(f"  [agent] codex failed: {out[:400]}", file=sys.stderr)
    sh(["git", "-c", "user.email=s@t", "-c", "user.name=s", "commit", "-qam", "agent-fix"], work)
    return code == 0

def agent_claude(task, work):
    code, out, err = sh(["claude", "-p", "implement the issue described in issue.md"], work)
    if code != 0:
        print(f"  [agent] claude failed: {out[:400]}", file=sys.stderr)
    sh(["git", "-c", "user.email=s@t", "-c", "user.name=s", "commit", "-qam", "agent-fix"], work)
    return code == 0

AGENTS = {"fixture": agent_fixture, "codex": agent_codex, "claude": agent_claude}

def run_task(task_dir, agent_name, out_rows, keep_dir):
    tid = os.path.basename(task_dir.rstrip("/"))
    contract = os.path.join(task_dir, "contract.uni")
    with open(contract) as f:
        src = f.read()
    n_claims = src.count("VERIFY ")
    work = tempfile.mkdtemp(prefix=f"uni-study-{tid}-")

    # copy base
    base = os.path.join(task_dir, "base")
    for root, _dirs, files in os.walk(base):
        rel = os.path.relpath(root, base)
        dst = os.path.join(work, rel) if rel != "." else work
        os.makedirs(dst, exist_ok=True)
        for fn in files:
            shutil.copy(os.path.join(root, fn), os.path.join(dst, fn))
    with open(os.path.join(work, ".gitignore"), "w") as f:
        f.write("/target\n")

    sh(["git", "init", "-q"], work)
    sh(["git", "add", "-A"], work)
    sh(["git", "-c", "user.email=s@t", "-c", "user.name=s", "commit", "-qm", "base"], work)

    # trusted registry for this workspace
    os.makedirs(os.path.join(work, ".uni", "evidence"), exist_ok=True)
    with open(os.path.join(work, ".uni", "config.toml"), "w") as f:
        f.write("""[verifiers]
"mini.tests" = "cargo test"
"mini.t.add" = {"run" = "cargo test add_works -- --exact", "expect" = "test result: ok. 1 passed"}
"mini.t.clamp.lower" = {"run" = "cargo test clamp_lower_works -- --exact", "expect" = "test result: ok. 1 passed"}
"mini.t.clamp.upper" = {"run" = "cargo test clamp_upper_works -- --exact", "expect" = "test result: ok. 1 passed"}
"mini.t.sum.basic" = {"run" = "cargo test sum_three -- --exact", "expect" = "test result: ok. 1 passed"}
"mini.t.sum.one" = {"run" = "cargo test sum_one -- --exact", "expect" = "test result: ok. 1 passed"}
"mini.t.first.normal" = {"run" = "cargo test normal_sentence -- --exact", "expect" = "test result: ok. 1 passed"}
"mini.t.first.empty" = {"run" = "cargo test empty_string -- --exact", "expect" = "test result: ok. 1 passed"}
"mini.t.bump.basic" = {"run" = "cargo test bump_ten -- --exact", "expect" = "test result: ok. 1 passed"}
"mini.t.bump.floor" = {"run" = "cargo test bump_floor -- --exact", "expect" = "test result: ok. 1 passed"}
""")

    # agent implements
    ok = AGENTS[agent_name](task_dir, work)

    # UNI verify
    shutil.copy(contract, os.path.join(work, "contract.uni"))
    code, out_json, uni_err = sh([UNI, "--json", "verify", "contract.uni"], work)
    decision = "ERROR"
    claims_verified = 0
    try:
        v = json.loads(out_json)
        decision = v.get("decision", "ERROR")
        claims_verified = sum(1 for c in v.get("claims", []) if c["state"] == "Valid")
    except Exception:
        decision = f"CLI_ERR:{code} :: {uni_err[:120]}"

    out_rows.append({
        "issue": tid,
        "agent_done": 1,  # agent self-report is always "done"
        "uni_decision": decision,
        # fixture agent encodes ground truth; real study: human column filled by review
        "human_review": "",
        "claims_total": n_claims,
        "claims_verified": claims_verified,
    })
    print(f"  {tid}: {decision} ({claims_verified}/{n_claims} claims)")
    if keep_dir:
        print(f"  (kept: {work})")
    else:
        shutil.rmtree(work, ignore_errors=True)

    return os.path.basename(task_dir)

if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--tasks", default="tasks")
    ap.add_argument("--agent", default="fixture", choices=list(AGENTS))
    ap.add_argument("--out", default="results.csv")
    ap.add_argument("--keep", action="store_true")
    ap.add_argument("--only", default=None)
    args = ap.parse_args()
    keep_dir = args.keep
    rows = []
    for name in sorted(os.listdir(args.tasks)):
        path = os.path.join(args.tasks, name)
        if not os.path.isdir(path) or not os.path.exists(os.path.join(path, "contract.uni")):
            continue
        if args.only and args.only != name:
            continue
        run_task(path, args.agent, rows, args.keep)
    cols = ["issue", "agent_done", "uni_decision", "human_review", "claims_total", "claims_verified"]
    with open(args.out, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=cols)
        w.writeheader()
        for r in rows:
            w.writerow(r)
    print(f"wrote {args.out}")
