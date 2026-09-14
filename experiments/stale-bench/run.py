#!/usr/bin/env python3
"""Stale Evidence Benchmark: does ordinary verification keep saying green after
the context it was produced in changed?

No model is involved. Each scenario builds a small repo, proves it with UNI,
applies one drift and then asks three oracles on the delivered revision:

  B0 exit-code CI   the CI workflow's pinned commands, green when they exit 0
  B1 cached CI      reuse the previous green unless the tests themselves changed
  UNI               the full verification context: expected output, watched
                    content, frozen files, registry, authorization, time

Ground truth is written by the scenario author, independently of UNI.

Usage: python3 run.py [--variants N] [--out results.csv] [--keep]
"""
import argparse
import csv
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
UNI = os.environ.get("UNI_BIN", os.path.join(HERE, "..", "..", "target", "release", "uni"))

CATEGORIES = [
    "artifact_breaks_claim",
    "artifact_keeps_claim",
    "subject_breaks_claim",
    "verifier_weakened",
    "frozen_file_tampered",
    "claim_without_verifier",
    "authorization_rebound",
    "registry_weakened",
    "temporal_expired",
    "platform_drift",
]

# categories where a trusted-config change is the drift: UNI names it rather
# than rejects it, because a weakened trusted registry cannot be defended
# automatically, only surfaced for review
BOUNDARY_CATEGORIES = {"registry_weakened"}
# categories where the drift is context, not content: the artifact is untouched
FRESHNESS_CATEGORIES = {"temporal_expired", "platform_drift"}
# categories whose green state is a plain CI green, not a UNI proof
NO_PROOF_CATEGORIES = {"claim_without_verifier"}


def sh(args, cwd):
    p = subprocess.run(args, cwd=cwd, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    return p.returncode, (p.stdout or ""), (p.stderr or "")


def sha256(path):
    with open(path, "rb") as fh:
        return hashlib.sha256(fh.read()).hexdigest()


def files_fingerprint(root, rel):
    base = os.path.join(root, rel)
    if not os.path.isdir(base):
        return sha256(base)
    parts = []
    for name in sorted(os.listdir(base)):
        parts.append(name.encode())
        parts.append(open(os.path.join(base, name), "rb").read())
    return hashlib.sha256(b"".join(parts)).hexdigest()


def check_script(template=False):
    if template:
        return '#!/bin/sh\n[ -n "$1" ] || exit 1\ngrep -q GREEN "$1" || exit 1\necho "1 passed"\n'
    return '#!/bin/sh\ngrep -q GREEN src.txt || exit 1\necho "1 passed"\n'


def write_repo(root, kind):
    uni_dir = os.path.join(root, ".uni")
    os.makedirs(os.path.join(uni_dir, "evidence"), exist_ok=True)
    os.makedirs(os.path.join(root, "tests"), exist_ok=True)
    with open(os.path.join(root, "src.txt"), "w") as fh:
        fh.write("GREEN\n")
    with open(os.path.join(root, "b.txt"), "w") as fh:
        fh.write("CONFIRMED\n")
    with open(os.path.join(root, "src-reads-ready.txt"), "w") as fh:
        fh.write("GREEN\n")
    with open(os.path.join(root, "invariants.txt"), "w") as fh:
        fh.write("invariant: total conserved\n")
    with open(os.path.join(root, "tests/check.sh"), "w") as fh:
        fh.write(check_script(template=kind == "authorization_rebound"))
    os.chmod(os.path.join(root, "tests/check.sh"), 0o755)

    with_claim_b = True
    with_verifier_b = kind not in NO_PROOF_CATEGORIES
    lines = [
        "VERSION 0.1", "DOMAIN software", "INTENT bench", "GOAL", "  benchmark",
        "CLAIM a REQUIRED", "  ENSURE src.txt says GREEN",
    ]
    if with_claim_b:
        lines += ["CLAIM b REQUIRED", "  ENSURE b.txt says CONFIRMED"]
    lines += ["INVARIANT frozen CRITICAL", "  ENSURE the frozen file is untouched"]
    lines += ["VERIFY a", "  USING claim.a"]
    if kind == "authorization_rebound":
        lines += ['  REQUIRE behavior("src says GREEN")']
    if with_verifier_b:
        lines += ["VERIFY b", "  USING claim.b"]
    lines += [
        "VERIFY frozen", "  USING frozen",
        "ACCEPT WHEN", "  required_claims == VERIFIED", "  AND critical_failures == 0",
    ]
    with open(os.path.join(root, "c.uni"), "w") as fh:
        fh.write("\n".join(lines) + "\n")

    frozen = sha256(os.path.join(root, "invariants.txt"))
    if kind == "authorization_rebound":
        a = '[verifiers."claim.a"]\nrun = "sh tests/check.sh {{selector}}"\nexpect = "1 passed"\nfiles = ["tests/check.sh"]\n'
    else:
        a = '[verifiers."claim.a"]\nrun = "sh tests/check.sh"\nexpect = "1 passed"\nfiles = ["src.txt", "tests/check.sh"]\n'
    registry = (
        a
        + '\n[verifiers."claim.b"]\nrun = "grep -q CONFIRMED b.txt"\nfiles = ["b.txt"]\n'
        + f'\n[verifiers."frozen"]\ntype = "file-hash"\nfiles = ["invariants.txt"]\nexpect_sha256 = "{frozen}"\n'
    )
    with open(os.path.join(uni_dir, "config.toml"), "w") as fh:
        fh.write(registry)

    sh(["git", "init", "-q"], root)
    sh(["git", "add", "-A"], root)
    sh(["git", "-c", "user.email=b@b", "-c", "user.name=b", "commit", "-q", "-m", "green"], root)

    if kind == "authorization_rebound":
        sh([UNI, "bind", "--claim", "a", "--verifier", "claim.a",
            "--requirement", 'behavior("src says GREEN")', "--selector", "src-reads-ready.txt"], root)
    code, _, _ = sh([UNI, "verify", "c.uni"], root)
    if code != 0 and kind not in NO_PROOF_CATEGORIES:
        return False
    return True


def pinned_commands(root):
    """The commands a CI workflow would have pinned in its file: the registry's
    shell verifiers, with the authorized selector already substituted."""
    text = open(os.path.join(root, ".uni/config.toml")).read()
    selector = None
    bpath = os.path.join(root, ".uni/bindings.toml")
    if os.path.exists(bpath):
        for line in open(bpath):
            if line.strip().startswith("selector"):
                selector = line.split("=", 1)[1].strip().strip('"')
    out = []
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("run = "):
            cmd = line[len("run = "):].strip().strip('"')
            if selector and "{{selector}}" in cmd:
                cmd = cmd.replace("{{selector}}", selector)
            out.append(cmd)
    return out


def b0_exit_code_ci(root, cmds):
    for cmd in cmds:
        code, _, _ = sh(["sh", "-c", cmd], root)
        if code != 0:
            return False
    return True


def b1_cached_ci(root, cmds, pre_tests, pre_contract):
    """A result cache, a test-impact analysis, or a path-filtered workflow:
    reuse the previous green unless the tests or the contract changed."""
    if files_fingerprint(root, "tests") == pre_tests and sha256(os.path.join(root, "c.uni")) == pre_contract:
        return True
    return b0_exit_code_ci(root, cmds)


def uni_verdict(root):
    code, out, _ = sh([UNI, "--json", "verify", "c.uni"], root)
    decision = f"CLI_ERR:{code}"
    try:
        decision = json.loads(out).get("decision", decision)
    except Exception:
        pass
    journal = ""
    jpath = os.path.join(root, ".uni/events.jsonl")
    if os.path.exists(jpath):
        journal = open(jpath).read()
    return {
        "decision": decision,
        "accepts": decision == "Accepted",
        "stale_recorded": "EvidenceStale" in journal,
        "boundary_named": "verifier_config_changed" in journal or "REGISTRY_CHANGED" in journal,
    }


def apply_drift(root, kind, variant):
    """Apply the drift; return (ground_truth_artifact_satisfies_claims, note)."""
    if kind == "artifact_breaks_claim":
        open(os.path.join(root, "src.txt"), "w").write(f"RED {variant}\n")
        return False, "source edited so the claim no longer holds"
    if kind == "artifact_keeps_claim":
        open(os.path.join(root, "src.txt"), "w").write(f"GREEN {variant}\n")
        return True, "source edited, claim still holds"
    if kind == "subject_breaks_claim":
        open(os.path.join(root, "b.txt"), "w").write(f"DENIED {variant}\n")
        return False, "the watched data file no longer says CONFIRMED"
    if kind == "verifier_weakened":
        # The check is disabled and the artifact broken together: the classic
        # "skip the failing test" move. Exit code stays 0, output says 0 passed.
        open(os.path.join(root, "tests/check.sh"), "w").write('#!/bin/sh\necho "0 passed"\nexit 0\n')
        open(os.path.join(root, "src.txt"), "w").write(f"RED {variant}\n")
        return False, "the selected check was disabled to 0 passed while the artifact broke"
    if kind == "frozen_file_tampered":
        open(os.path.join(root, "invariants.txt"), "w").write(f"invariant: weakened {variant}\n")
        return False, "the frozen invariant file was edited"
    if kind == "claim_without_verifier":
        return False, "required claim b has no verifier at all"
    if kind == "authorization_rebound":
        sh([UNI, "bind", "--claim", "a", "--verifier", "claim.a",
            "--requirement", 'behavior("src says READY")', "--selector", "gone"], root)
        return False, "re-authorized to a selector that selects nothing"
    if kind == "registry_weakened":
        text = open(os.path.join(root, ".uni/config.toml")).read()
        text = text.replace('run = "sh tests/check.sh"', 'run = "printf \'1 passed\\n\'"')
        open(os.path.join(root, ".uni/config.toml"), "w").write(text)
        return False, "the trusted registry check was replaced by one that always passes"
    if kind == "temporal_expired":
        edit_evidence(root, {"expires_at": "2026-01-01T00:00:00Z"})
        return True, "proof aged past its window, artifact untouched"
    if kind == "platform_drift":
        edit_evidence(root, {"platform": "plan9-mips"})
        return True, "proof made on another platform, artifact untouched"
    raise ValueError(kind)


def edit_evidence(root, patch):
    ev_dir = os.path.join(root, ".uni/evidence")
    for name in os.listdir(ev_dir):
        path = os.path.join(ev_dir, name)
        ev = json.load(open(path))
        ev.update(patch)
        with open(path, "w") as fh:
            json.dump(ev, fh)


def run_scenario(kind, variant, keep=False):
    root = tempfile.mkdtemp(prefix=f"uni-bench-{kind}-{variant}-")
    row = {"category": kind, "variant": variant}
    try:
        if not write_repo(root, kind):
            row["error"] = "green state not reached"
            return row
        cmds = pinned_commands(root)
        pre_tests = files_fingerprint(root, "tests")
        pre_contract = sha256(os.path.join(root, "c.uni"))
        if kind in FRESHNESS_CATEGORIES:
            sh([UNI, "verify", "c.uni"], root)  # fresh proof to age or mirror
        satisfied, note = apply_drift(root, kind, variant)
        v = uni_verdict(root)
        row.update({
            "ground_truth_satisfied": satisfied,
            "note": note,
            "b0_green": b0_exit_code_ci(root, cmds),
            "b1_green": b1_cached_ci(root, cmds, pre_tests, pre_contract),
            "uni_decision": v["decision"],
            "uni_accepts": v["accepts"],
            "uni_stale_recorded": v["stale_recorded"],
            "uni_boundary_named": v["boundary_named"],
        })
        mark = "ACCEPT" if v["accepts"] else "reject"
        print(f"  {kind} #{variant}: UNI {mark}  B0={'green' if row['b0_green'] else 'red '}  B1={'green' if row['b1_green'] else 'red '}")
        return row
    except Exception as exc:  # keep the run going, record the failure
        row["error"] = f"{type(exc).__name__}: {exc}"
        print(f"  {kind} #{variant}: {row['error']}", file=sys.stderr)
        return row
    finally:
        if keep:
            print(f"    kept {root}", file=sys.stderr)
        else:
            shutil.rmtree(root, ignore_errors=True)


def pct(n, d):
    return "n/a" if d == 0 else f"{100.0 * n / d:.0f}%"


def report(rows, out):
    def pct(n, d):
        return "n/a" if d == 0 else f"{100.0 * n / d:.0f}%"

    print(f"\n=== Stale Evidence Benchmark, {len(rows)} scenarios ===")
    head = (f"{'category':<26}{'n':>4}{'UNI detect':>12}{'UNI false-stale':>17}"
            f"{'B0 miss':>9}{'B1 miss':>9}{'stale noted':>13}{'boundary':>10}")
    print(head)
    print("-" * len(head))
    t = dict(n=0, unguarded=0, detected=0, b0=0, b1=0, false_stale=0, satisfied=0,
             stale_expected=0, stale_noted=0, boundary_expected=0, boundary_named=0)
    for kind in CATEGORIES:
        cat = [r for r in rows if r.get("category") == kind and "error" not in r]
        bad = [r for r in cat if not r["ground_truth_satisfied"]]
        good = [r for r in cat if r["ground_truth_satisfied"]]
        det = [r for r in bad if not r["uni_accepts"]]
        b0 = [r for r in bad if r["b0_green"]]
        b1 = [r for r in bad if r["b1_green"]]
        fs = [r for r in good if not r["uni_accepts"]]
        stale_exp = kind in FRESHNESS_CATEGORIES
        stale_noted = [r for r in cat if r["uni_stale_recorded"]]
        bnd_exp = kind in BOUNDARY_CATEGORIES
        bnd_named = [r for r in cat if r["uni_boundary_named"]]
        print(f"{kind:<26}{len(cat):>4}{pct(len(det), len(bad)):>12}{pct(len(fs), len(good)):>17}"
              f"{pct(len(b0), len(bad)):>9}{pct(len(b1), len(bad)):>9}"
              f"{(pct(len(stale_noted), len(cat)) if stale_exp else '-'):>13}"
              f"{(pct(len(bnd_named), len(cat)) if bnd_exp else '-'):>10}")
        t["n"] += len(cat); t["unguarded"] += len(bad); t["detected"] += len(det)
        t["b0"] += len(b0); t["b1"] += len(b1); t["false_stale"] += len(fs); t["satisfied"] += len(good)
        if stale_exp:
            t["stale_expected"] += len(cat); t["stale_noted"] += len(stale_noted)
        if bnd_exp:
            t["boundary_expected"] += len(cat); t["boundary_named"] += len(bnd_named)

    print("-" * len(head))
    print(f"scenarios {t['n']}; artifact no longer satisfied the claims: {t['unguarded']}")
    print(f"UNI detection rate       {pct(t['detected'], t['unguarded'])}")
    print(f"UNI false-stale rate     {pct(t['false_stale'], t['satisfied'])}")
    print(f"exit-code CI miss rate   {pct(t['b0'], t['unguarded'])}")
    print(f"cached CI miss rate      {pct(t['b1'], t['unguarded'])}")
    print(f"freshness marked stale   {pct(t['stale_noted'], t['stale_expected'])}")
    print(f"boundary change named    {pct(t['boundary_named'], t['boundary_expected'])}")
    print(f"\nwrote {out}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--variants", type=int, default=10)
    ap.add_argument("--out", default=os.path.join(HERE, "results.csv"))
    ap.add_argument("--keep", action="store_true")
    args = ap.parse_args()

    rows = [run_scenario(kind, v, args.keep) for kind in CATEGORIES for v in range(1, args.variants + 1)]
    cols = ["category", "variant", "ground_truth_satisfied", "note", "b0_green", "b1_green",
            "uni_decision", "uni_accepts", "uni_stale_recorded", "uni_boundary_named", "error"]
    with open(args.out, "w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=cols, extrasaction="ignore")
        w.writeheader()
        w.writerows(rows)
    report(rows, args.out)


if __name__ == "__main__":
    main()
