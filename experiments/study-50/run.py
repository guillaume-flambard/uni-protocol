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
import re
import os
import shutil
import subprocess
import sys
import tempfile

UNI = os.environ.get(
    "UNI_BIN",
    os.path.join(os.path.dirname(__file__), "..", "..", "target", "release", "uni"),
)


def sh(args, cwd, capture=True, timeout=None):
    """Run a command. `timeout` bounds it in seconds and is a data point, not a
    stall: a hung implementer used to block the harness indefinitely, which cost
    40 minutes before it was noticed."""
    try:
        p = subprocess.run(args, cwd=cwd, text=True, timeout=timeout,
                           stdout=subprocess.PIPE if capture else None,
                           stderr=subprocess.PIPE if capture else None)
    except subprocess.TimeoutExpired as exc:
        # Decode first: subprocess hands back bytes on a timeout even in text
        # mode, and concatenating a str onto them raises.
        def as_text(v):
            if v is None:
                return ""
            return v.decode("utf-8", "replace") if isinstance(v, bytes) else v
        out = as_text(exc.stdout)
        err = as_text(exc.stderr) + f"\n[harness] killed after {timeout}s"
        return -9, out, err
    return p.returncode, (p.stdout or ""), (p.stderr or "")

def agent_fixture(task, work):
    patch = os.path.abspath(os.path.join(task, "fix.patch"))
    code, out, err = sh(["git", "apply", "--whitespace=nowarn", patch], work)
    if code != 0:
        print(f"  [agent] patch failed in {task}: {out}{err}", file=sys.stderr)
    sh(["git", "add", "-A"], work)
    sh(["git", "-c", "user.email=s@t", "-c", "user.name=s", "commit", "-qm", "agent-fix"], work)
    return code == 0, "DONE"

def agent_codex(task, work):
    code, out, err = sh(["codex", "exec", "implement the issue described in issue.md"], work)
    if code != 0:
        print(f"  [agent] codex failed: {out[:400]}", file=sys.stderr)
    sh(["git", "add", "-A"], work)
    sh(["git", "-c", "user.email=s@t", "-c", "user.name=s", "commit", "-qm", "agent-fix"], work)
    return code == 0, "DONE"

def agent_claude(task, work):
    code, out, err = sh(["claude", "-p", "implement the issue described in issue.md"], work)
    if code != 0:
        print(f"  [agent] claude failed: {out[:400]}", file=sys.stderr)
    sh(["git", "add", "-A"], work)
    sh(["git", "-c", "user.email=s@t", "-c", "user.name=s", "commit", "-qm", "agent-fix"], work)
    return code == 0


def agent_opencode(task, work):
    issue = open(os.path.join(task, "issue.md")).read()
    brief_note = ""
    if os.environ.get("UNI_BRIEF") == "1":
        # 2026-09-15: brief.md was generated but never referenced, and the
        # model ignored it (3/3 naming rejects repeated). The handoff must
        # name the work order, or it is decoration.
        brief_note = (" A file brief.md in this repository is your work order: it names "
                      "the exact evidence each claim needs, including test names. "
                      "Read it first and follow it exactly.\n\n")
    prompt = ("Read issue.md in this repository and implement the requested change.\n\n"
              + issue + "\n" + brief_note
              + "\nOnly modify source and test files. Run the full test suite before finishing. "
                "Reply with exactly DONE or FAILED at the end.")
    code, out, err = sh([
        "opencode", "run", "--pure", "--auto",
        # --dir is REQUIRED: without it opencode resolved a stale project
        # directory from a previous session and edited the wrong repo.
        "--dir", work,
        "-m", os.environ.get("UNI_AGENT_MODEL", "bai/qwen3.8-flash"),
        prompt,
    ], work, timeout=int(os.environ.get("UNI_AGENT_TIMEOUT", "900")))
    tail = (out or "").strip().splitlines()
    print(f"    agent tail: {tail[-1][:80] if tail else '(none)'}")
    if code != 0:
        print(f"  [agent] opencode failed: {err[:300]}", file=sys.stderr)
    sh(["git", "add", "-A"], work)
    sh(["git", "-c", "user.email=s@t", "-c", "user.name=s", "commit", "-qm", "agent-fix"], work)
    return code == 0, parse_self_report(out)


def parse_self_report(output):
    """Read the worker's own claim from its output, and never invent one.

    This returned a hardcoded "DONE" for the whole of the opencode arm, which
    made `agent_self_report` and the `agent_done` baseline meaningless for real
    models: the harness was asserting the claim, not observing it, and the one
    FAILED row in the published results had been entered by hand. The prompt
    asks for exactly DONE or FAILED as the last line, so that is what is read;
    anything else is UNPARSED, which is a result rather than a guess."""
    import re
    for line in reversed((output or "").splitlines()):
        token = line.strip().strip("`*_ .!:\"'")
        if re.fullmatch(r"(?i)(done|failed)", token):
            return token.upper()
    return "UNPARSED"


def agent_plausible(task, work):
    """Scripted plausible-but-wrong delivery, not a model behaviour: a
    floating-point fee rounded to nearest. It conserves the ledger total and it
    is idempotent, so every test the worker writes about behaviour is green,
    and the worker's own `cargo test` passes. The owner's invariant is the only
    place floor and round part ways. This is the "looks done, is wrong" case
    the product exists for, and it is a scripted flow for the same reason as
    `careless`: no model on hand lies convincingly, so the failure mode is
    staged rather than waited for."""
    path = os.path.abspath(os.path.join(task, "wrong.patch"))
    code, out, err = sh(["git", "apply", "--whitespace=nowarn", path], work)
    if code != 0:
        print(f"  [plausible] wrong.patch failed: {out}{err}", file=sys.stderr)
        return False, "FAILED"
    tcode, _, _ = sh(["cargo", "test"], work)
    print(f"    plausible: worker's own cargo test exit={tcode} (green; it never saw the owner's invariant)")
    sh(["git", "add", "-A"], work)
    sh(["git", "-c", "user.email=s@t", "-c", "user.name=s", "commit", "-qm", "plausible-fix"], work)
    return code == 0, "DONE"


def agent_careless(task, work):
    """Scripted failure mode, not a model: the agent fixes the task, runs the
    tests (green), verifies with UNI (ACCEPTED), then edits the code again
    without re-running anything, and reports DONE. This is the classic
    "verified at an earlier revision" case, and it is exactly what UNI's
    evidence lifecycle exists to catch."""
    code, out, err = sh(["git", "apply", "--whitespace=nowarn", os.path.abspath(os.path.join(task, "fix.patch"))], work)
    if code != 0:
        print(f"  [careless] fix.patch failed: {out}{err}", file=sys.stderr)
        return False, "FAILED"
    sh(["git", "add", "-A"], work)
    sh(["git", "-c", "user.email=s@t", "-c", "user.name=s", "commit", "-qm", "agent-fix"], work)
    # The agent's own green run, which it will keep citing.
    tcode, tout, _ = sh(["cargo", "test"], work)
    print(f"    careless: cargo test exit={tcode} (tests were green at this revision)")
    # And an explicit UNI verification at this revision, recorded on disk.
    # The premise of the arm is that valid evidence exists here and goes stale
    # on the late edit, so what is asserted is evidence, not a self-report.
    # `--json` because the decision is only machine-readable there. This used to
    # read the plain output, never got JSON, and fell back to the exit code:
    # 0 became "ACCEPTED" and anything else became "code:1", so a Rejected and
    # an EvidenceRequired were indistinguishable on the line that states the
    # arm's premise. Measured on t11: exit 0 on Accepted, exit 1 on Rejected,
    # and the pre-drift Rejected used to print "code:1". Naming the decision is
    # the point, for the same reason parse_self_report stopped hardcoding DONE.
    #
    # The test is per claim, because staleness is per claim. On a task that
    # withholds its owner invariants.rs until delivery, the contract can never
    # be Accepted here by construction, and requiring that would refuse the
    # arm for the wrong reason; the claims that can carry evidence (the
    # behavioural one and the suite) do, and those are what go stale.
    vcode, vout, _ = sh([UNI, "--json", "verify", "contract.uni"], work)
    try:
        pre = json.loads(vout)
        decision = pre.get("decision", "")
        valid_now = sum(1 for c in pre.get("claims", []) if c.get("state") == "Valid")
    except Exception:
        decision, valid_now = "", 0
    print(f"    careless: uni verify at this revision -> {decision} "
          f"({valid_now} claim(s) with valid evidence to go stale)")
    if not valid_now:
        seen = decision if decision else f"no decision (uni verify exit {vcode})"
        raise RuntimeError(
            f"{task}: no claim carries valid evidence at the pre-drift revision "
            f"({seen}), so there is nothing to go stale and this run would report "
            "a drift measurement that never happened. The arm needs a revision "
            "it can actually verify; --hide-contract removes the contract it "
            "verifies against, and a task whose fix cannot be verified here is "
            "not a candidate for this arm."
        )
    # Then a late edit, no re-run, no re-verify. The regression patch IS the
    # arm: no patch means no late edit, so there is no second revision, no
    # stale evidence and no drift. `if os.path.exists` used to swallow that and
    # fall through to `return True, "DONE"`, which wrote a green result row and
    # a DONE self-report for a measurement that never happened. A missing or
    # inapplicable patch is a broken fixture, not a result.
    reg = os.path.abspath(os.path.join(task, "regression.patch"))
    if not os.path.exists(reg):
        raise RuntimeError(
            f"{task}: no regression.patch. The careless arm is 'verified at one "
            "revision, edited at a later one', so without a patch there is no "
            "later revision and no drift to detect. Ship a regression.patch "
            "that breaks an owner invariant while leaving the worker's own "
            "suite green, or use another agent."
        )
    rcode, rout, rerr = sh(["git", "apply", "--whitespace=nowarn", reg], work)
    if rcode != 0:
        raise RuntimeError(
            f"{task}: regression.patch does not apply to the fixed revision "
            f"(git apply exit {rcode}): {rout}{rerr}. The drift was never "
            "applied, so the arm ran nothing; fix the patch against the tree "
            "that fix.patch leaves behind."
        )
    print(f"    careless: regression applied={rcode == 0}")
    sh(["git", "add", "-A"], work)
    sh(["git", "-c", "user.email=s@t", "-c", "user.name=s", "commit", "-qm", "late-edit"], work)
    return True, "DONE"

AGENTS = {"careless": agent_careless, "plausible": agent_plausible, "fixture": agent_fixture,
          "codex": agent_codex, "claude": agent_claude, "opencode": agent_opencode}

def fixture_reset(repo, tasks_rel):
    """Restore tracked fixture files, and report untracked drift loudly.

    A previous agent run once wrote into the fixture tree and silently
    pre-fixed a task (UNI then accepted the base, not the agent). Tracked
    fixtures are restored; untracked drift is fatal, because it cannot be
    restored and would poison the measurement.
    """
    st = subprocess.run(["git", "-C", repo, "status", "--porcelain", "--", tasks_rel],
                        capture_output=True, text=True)
    for line in st.stdout.splitlines():
        if line.startswith("??"):
            raise RuntimeError(
                f"untracked fixture drift (cannot restore): {line.strip()}. "
                "Commit or remove it before running the study."
            )
    if st.stdout.strip():
        print("  [guard] restoring fixture drift:\n" + st.stdout.rstrip(), file=sys.stderr)
        subprocess.run(["git", "-C", repo, "checkout", "--", tasks_rel])


REGISTRY = """[verifiers]
"mini.tests" = "cargo test"
"mini.t.add" = {"run" = "cargo test add_works -- --exact", "expect" = "test result: ok. 1 passed"}
"mini.t.greeting" = {"run" = "cargo test greeting_works -- --exact", "expect" = "test result: ok. 1 passed"}
"mini.t.cancel.ok" = {"run" = "cargo test cancel_ok -- --exact", "expect" = "test result: ok. 1 passed"}
"mini.t.cancel.missing" = {"run" = "cargo test cancel_missing -- --exact", "expect" = "test result: ok. 1 passed"}
"mini.layering" = {"run" = "! grep -n 'crate::store' src/service.rs", "expect_not" = "crate::store", "files" = ["src/service.rs"]}
"mini.t.port.valid" = {"run" = "cargo test parse_valid -- --exact", "expect" = "test result: ok. 1 passed"}
"mini.t.port.invalid" = {"run" = "cargo test parse_invalid -- --exact", "expect" = "test result: ok. 1 passed"}
"mini.no.unwrap" = {"run" = '! grep -Rn "\\.unwrap()" src', "expect_not" = ".unwrap()", "files" = ["src/config.rs", "src/lib.rs"]}
"mini.t.format.symbol" = {"run" = "cargo test format_symbol -- --exact", "expect" = "test result: ok. 1 passed"}
"mini.api.freeze" = {"run" = "grep -n 'pub fn format_price(cents: u64) -> String' src/api.rs", "expect" = "pub fn format_price(cents: u64) -> String", "files" = ["src/api.rs"]}
"mini.t.transfer.moves" = {"run" = "cargo test transfer_moves_money -- --exact", "expect" = "test result: ok. 1 passed"}
"mini.ledger.conservation" = {"run" = "cargo test --test invariants", "expect" = "test result: ok. 2 passed", "files" = ["tests/invariants.rs"]}
"mini.invariants.frozen" = {"type" = "file-hash", "files" = ["tests/invariants.rs"], "expect_sha256" = "94c606b701078abb8c7c74d9f694407f3805fead64d996ef30b9fda0c9cf8e06"}
"mini.t.clamp.lower" = {"run" = "cargo test clamp_lower_works -- --exact", "expect" = "test result: ok. 1 passed"}
"mini.t.clamp.upper" = {"run" = "cargo test clamp_upper_works -- --exact", "expect" = "test result: ok. 1 passed"}
"mini.t.sum.basic" = {"run" = "cargo test sum_three -- --exact", "expect" = "test result: ok. 1 passed"}
"mini.t.sum.one" = {"run" = "cargo test sum_one -- --exact", "expect" = "test result: ok. 1 passed"}
"mini.t.first.normal" = {"run" = "cargo test normal_sentence -- --exact", "expect" = "test result: ok. 1 passed"}
"mini.t.first.empty" = {"run" = "cargo test empty_string -- --exact", "expect" = "test result: ok. 1 passed"}
"mini.t.bump.basic" = {"run" = "cargo test bump_ten -- --exact", "expect" = "test result: ok. 1 passed"}
"mini.t.bump.floor" = {"run" = "cargo test bump_floor -- --exact", "expect" = "test result: ok. 1 passed"}
# t11 (ADR-002 shape): the selector is deferred to the authorized binding, so
# the contract never names a test; the task hands the required name to the
# worker in issue.md, and bindings.toml is the human's authorization of it.
"mini.t.sel" = {"run" = "cargo test {{selector}} -- --exact", "expect" = "test result: ok. 1 passed"}
"mini.settlement.invariants" = {"run" = "cargo test --test invariants", "expect" = "test result: ok. 3 passed", "files" = ["tests/invariants.rs"]}
"mini.t11.invariants.frozen" = {"type" = "file-hash", "files" = ["tests/invariants.rs"], "expect_sha256" = "3a5b846f8f598ac7c0924dc772b4cf322a8c83004927be00e579bcfb73b25659"}
# t12: a derived index that a plausible batch path forgets to update, and an
# atomicity rule that a per-entry check against starting balances gets wrong.
"mini.ledger.index" = {"run" = "cargo test --test invariants", "expect" = "test result: ok. 4 passed", "files" = ["tests/invariants.rs"]}
"mini.t12.invariants.frozen" = {"type" = "file-hash", "files" = ["tests/invariants.rs"], "expect_sha256" = "4a57b1e09da36234dcfbaa9a9ed0395628aa091d8fbd33b87d27631bcdc111ec"}
"""


def write_registry(work, task_dir=None):
    """Write the trusted registry, and refuse to continue if it is not valid
    TOML: a parse error empties the registry, which silently turns every
    verifier reference into 'unknown verifier' and poisons the measurement.

    When the task ships `bindings.toml`, it is authorized here with the real
    `uni bind --from` act: the harness stands in for the human whose reviewed
    decision it is (ADR-002). Without it, a `{{selector}}` template is a hard
    error and the task would measure nothing."""
    import tomllib
    try:
        tomllib.loads(REGISTRY)
    except Exception as exc:  # pragma: no cover - guard
        raise RuntimeError(f"generated registry is invalid TOML: {exc}")
    os.makedirs(os.path.join(work, ".uni", "evidence"), exist_ok=True)
    with open(os.path.join(work, ".uni", "config.toml"), "w") as f:
        f.write(REGISTRY)
    if task_dir is None:
        return
    reviewed = os.path.join(task_dir, "bindings.toml")
    if not os.path.exists(reviewed):
        return
    dest = os.path.join(work, ".uni", "reviewed-bindings.toml")
    with open(reviewed) as src, open(dest, "w") as out:
        out.write(src.read())
    code, out, err = sh([UNI, "bind", "--from", ".uni/reviewed-bindings.toml"], work)
    if code != 0:
        raise RuntimeError(f"{task_dir}: could not authorize bindings.toml: {out}{err}")


def verification_trace(work):
    """The lifecycle trail of the verification that just produced the decision:
    the journal's event names in order, and the stale dimensions and revision
    it recorded.

    A bundle otherwise says what was decided but not why an earlier proof was
    not reused, which is the entire question on a drifted delivery. Both parts
    are raw CLI output: the event names verbatim, and the `stale` block and
    commit of `decisions/last.json` untouched."""
    trace = {}
    journal = os.path.join(work, ".uni", "events.jsonl")
    if os.path.exists(journal):
        with open(journal) as f:
            trace["journal"] = [
                json.loads(line).get("event", "") for line in f if line.strip()
            ]
    last = os.path.join(work, ".uni", "decisions", "last.json")
    if os.path.exists(last):
        try:
            with open(last) as f:
                data = json.load(f)
            trace["commit"] = data.get("commit")
            trace["stale"] = data.get("stale", {})
        except Exception:
            pass
    return trace


def run_task(task_dir, agent_name, out_rows, keep_dir, brief_mode=False, hide_contract=False,
             evidence_dir=None):
    tid = os.path.basename(task_dir.rstrip("/"))
    repo = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
    tasks_rel = os.path.join("experiments", "study-50", "tasks")
    fixture_reset(repo, tasks_rel)
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
    # The agent must find the task statement WHERE IT WORKS. Without this it
    # wanders up the filesystem (and once wrote into the fixture tree).
    shutil.copy(os.path.join(task_dir, "issue.md"), os.path.join(work, "issue.md"))

    sh(["git", "init", "-q"], work)
    sh(["git", "add", "-A"], work)
    sh(["git", "-c", "user.email=s@t", "-c", "user.name=s", "commit", "-qm", "base"], work)

    # trusted registry for this workspace
    write_registry(work, task_dir)

    # THE OWNER'S INVARIANT IS NOT THE WORKER'S TO READ. When a task ships
    # `invariants.rs` at its root (instead of inside base/), that file is the
    # specification the work must satisfy, and it is applied only after the
    # agent has finished: a worker who can read the invariant is not being
    # tested against it, it is being handed the answer. This is the whole claim
    # under test, so the file is never tracked in the agent's git history
    # either (no `git show HEAD~1` leak): it is written, measured, and removed.
    owner_invariants = os.path.join(task_dir, "invariants.rs")
    has_owner_invariants = os.path.exists(owner_invariants)
    if has_owner_invariants:
        os.makedirs(os.path.join(work, "tests"), exist_ok=True)
        shutil.copy(owner_invariants, os.path.join(work, "tests", "invariants.rs"))

    # PRE-FLIGHT: the base must NOT already satisfy the contract. A fixture
    # contaminated by a previous run (or a task that needs no work) would make
    # the agent's contribution invisible and inflate UNI's acceptance record.
    shutil.copy(contract, os.path.join(work, "contract.uni"))
    base_code, base_json, base_err = sh([UNI, "--json", "verify", "contract.uni"], work)
    baseline_decision = "ERROR"
    try:
        baseline_decision = json.loads(base_json).get("decision", "ERROR")
    except Exception:
        baseline_decision = f"CLI_ERR:{base_code} :: {base_err[:120]}"
    if baseline_decision == "Accepted":
        raise RuntimeError(
            f"{tid}: fixture already satisfies the contract (baseline {baseline_decision}); "
            "the task is invalid and was not run"
        )
    # Reset evidence so the agent's result is measured from a clean slate.
    shutil.rmtree(os.path.join(work, ".uni", "evidence"), ignore_errors=True)
    os.makedirs(os.path.join(work, ".uni", "evidence"), exist_ok=True)

    # Take the owner's invariant away again before the agent starts. The agent
    # may still write its own tests/invariants.rs; the owner's file replaces it
    # at delivery, which is what "the owner's invariant wins" means in practice.
    if has_owner_invariants:
        os.remove(os.path.join(work, "tests", "invariants.rs"))
        print("    arm: the owner's invariant is hidden from the agent")

    # Arm: the agent gets the issue only, no contract, no brief. This is the
    # realistic "handed an issue from a tracker" case, and the only arm that can
    # show UNI catching work that looks plausible but violates a declared rule.
    if hide_contract:
        # Issue-only handoff: the agent sees the repository and the issue, not
        # the contract and not the trusted registry (both are restored after).
        shutil.rmtree(os.path.join(work, ".uni"), ignore_errors=True)
        os.remove(os.path.join(work, "contract.uni"))
        print("    arm: contract + registry hidden from the agent")

    # A/B arm: hand the agent the deterministic work order (uni brief), which
    # names the exact evidence each claim needs, including test selectors.
    if brief_mode:
        code, _, err = sh([UNI, "brief", "contract.uni", "--out", "brief.md"], work)
        if code != 0:
            raise RuntimeError(f"{tid}: could not generate brief.md: {err[:200]}")

    # agent implements; the self-report is data, not truth
    if brief_mode:
        os.environ["UNI_BRIEF"] = "1"
    try:
        ok, self_report = AGENTS[agent_name](task_dir, work)
    finally:
        os.environ.pop("UNI_BRIEF", None)
    if hide_contract:
        write_registry(work, task_dir)

    # Human-review artifact: the agent's source/test diff, persisted durably.
    # Review must never depend on a temporary directory surviving.
    _, agent_diff, _ = sh(["git", "diff", "HEAD~1", "HEAD", "--", "src", "tests"], work)
    diffs_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "diffs")
    os.makedirs(diffs_dir, exist_ok=True)
    # One file per (task, agent): a sweep across models used to overwrite every
    # earlier delivery with the last one, so review could not see them.
    model = os.environ.get("UNI_AGENT_MODEL", "")
    slug = re.sub(r"[^a-z0-9]+", "-", model.lower()).strip("-") if model else ""
    diff_name = f"{tid}__{slug}.patch" if slug else f"{tid}.patch"
    with open(os.path.join(diffs_dir, diff_name), "w") as f:
        f.write(agent_diff)

    # Deliver the owner's invariant now that the agent cannot see it: this is
    # the revision that gets verified, and the only one that ever contained it.
    if has_owner_invariants:
        shutil.copy(owner_invariants, os.path.join(work, "tests", "invariants.rs"))
        sh(["git", "add", "-A"], work)
        sh(["git", "-c", "user.email=s@t", "-c", "user.name=s", "commit", "-qm", "owner-invariants"], work)

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
        "model": os.environ.get("UNI_AGENT_MODEL", agent_name),
        # 1 only when the agent itself claimed completion (DONE), never assumed
        "agent_done": 1 if self_report == "DONE" else 0,
        "agent_self_report": self_report,
        "uni_decision": decision,
        # fixture agent encodes ground truth; real study: human column filled by review
        "human_review": "",
        "claims_total": n_claims,
        "claims_verified": claims_verified,
        "baseline_decision": baseline_decision,
        "brief_mode": 1 if brief_mode else 0,
        "contract_visible": 0 if hide_contract else 1,
        "diff_lines": len(agent_diff.splitlines()),
    })
    print(f"  {tid}: {decision} ({claims_verified}/{n_claims} claims)")

    # Evidence bundle: the verification that produced the row, persisted so a
    # reader can audit the decision without re-running (and without a temporary
    # directory surviving). The raw verify output is stored whole, not summarised.
    if evidence_dir:
        try:
            verify_json = json.loads(out_json)
        except Exception:
            verify_json = {"raw": out_json, "stderr": uni_err, "exit_code": code}
        os.makedirs(evidence_dir, exist_ok=True)
        bundle_name = f"{tid}__{slug or agent_name}"
        with open(os.path.join(evidence_dir, bundle_name + ".json"), "w") as f:
            json.dump({
                "task": tid,
                "agent": agent_name,
                "model": os.environ.get("UNI_AGENT_MODEL", agent_name),
                "baseline_decision": baseline_decision,
                "agent_self_report": self_report,
                "agent_done": 1 if self_report == "DONE" else 0,
                "uni_decision": decision,
                "claims_total": n_claims,
                "claims_verified": claims_verified,
                "contract_visible": 0 if hide_contract else 1,
                "brief_mode": 1 if brief_mode else 0,
                "diff_file": os.path.join("diffs", diff_name),
                "verify": verify_json,
                "trace": verification_trace(work),
            }, f, indent=2)

    # Post-run guard: same reset, so the next task starts clean.
    fixture_reset(repo, tasks_rel)

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
    ap.add_argument("--append", action="store_true", help="keep existing rows in --out")
    ap.add_argument("--brief", action="store_true", help="A/B arm: generate uni brief into the agent workspace")
    ap.add_argument("--hide-contract", action="store_true", help="arm: remove the contract from the agent workspace (issue-only handoff)")
    ap.add_argument("--evidence", default="evidence", help="directory for per-run verification bundles (empty string to disable)")
    args = ap.parse_args()
    evidence_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), args.evidence) if args.evidence else None
    keep_dir = args.keep
    rows = []
    if args.append and os.path.exists(args.out):
        with open(args.out, newline="") as f:
            rows = [r for r in csv.DictReader(f)]
    for name in sorted(os.listdir(args.tasks)):
        path = os.path.join(args.tasks, name)
        if not os.path.isdir(path) or not os.path.exists(os.path.join(path, "contract.uni")):
            continue
        if args.only and args.only != name:
            continue
        run_task(path, args.agent, rows, args.keep, brief_mode=args.brief,
                 hide_contract=args.hide_contract, evidence_dir=evidence_dir)
    cols = [
        "issue", "model", "agent_done", "agent_self_report", "uni_decision", "human_review",
        "claims_total", "claims_verified", "baseline_decision", "brief_mode", "contract_visible",
        "agent_seconds", "agent_cost_usd", "diff_lines",
    ]
    with open(args.out, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=cols, extrasaction="ignore")
        w.writeheader()
        for r in rows:
            w.writerow(r)
    print(f"wrote {args.out}")
