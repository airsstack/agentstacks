# dist 0.33.1: pull requests with per-binary versions

Same scratch copy as dist-results.md (agentstacks-cli 0.1.0, claudevs-cli 0.1.2). Run 2026-11-24.

## Untagged `dist plan` per pr-run-mode (`prmode-probe.sh`)

```text
$ pr-run-mode = "upload"; dist plan   # no tag
exit=255
  × There are too many unrelated apps in your workspace to coherently Announce!
        --tag=v0.1.0 will Announce: agentstacks-cli
        --tag=v0.1.2 will Announce: claudevs-cli
$ pr-run-mode = "plan"; dist plan   # no tag
exit=0
announcing v0.1.0
announcing v0.1.2
$ pr-run-mode = "skip"; dist plan   # no tag
exit=0
announcing v0.1.0
announcing v0.1.2
$ (pr-run-mode = "skip") allow-dirty removed; dist generate; sed -n 41,46p .github/workflows/release.yml
exit=0
on:
  push:
    tags:
      - '**[0-9]+.[0-9]+.[0-9]+*'

jobs:
14:pr-run-mode = "upload"
15:allow-dirty = ["ci"]
```

## Jobs of the generated release workflow under pr-run-mode = "plan" (`planmode-probe.sh`)

```text
generate exit=0
$ grep -n -E "^  [a-z-]+:$|^    if:|^    needs:|publishing" release-planmode.yml
43:  push:
49:  plan:
55:      publishing: ${{ !github.event.pull_request }}
91:  build-local-artifacts:
94:    needs:
96:    if: ${{ fromJson(needs.plan.outputs.val).ci.github.artifacts_matrix.include != null && (needs.plan.outputs.publishing == 'true' || fromJson(needs.plan.outputs.val).ci.github.pr_run_mode == 'upload') }}
177:  build-global-artifacts:
178:    needs:
223:  custom-smoke:
224:    needs:
232:  host:
233:    needs:
239:    if: ${{ always() && needs.plan.result == 'success' && needs.plan.outputs.publishing == 'true' && (needs.build-global-artifacts.result == 'skipped' || needs.build-global-artifacts.result == 'success') && (needs.custom-smoke.result == 'skipped' || needs.custom-smoke.result == 'success') && (needs.build-local-artifacts.result == 'skipped' || needs.build-local-artifacts.result == 'success') }}
299:  announce:
300:    needs:
306:    if: ${{ always() && needs.host.result == 'success' }}
$ sed -n 41,45p release-planmode.yml
on:
  pull_request:
  push:
    tags:
      - '**[0-9]+.[0-9]+.[0-9]+*'
$ sed -n 223,227p release-planmode.yml
  custom-smoke:
    needs:
      - plan
      - build-local-artifacts
    uses: ./.github/workflows/smoke.yml
```

## GitHub: a job whose needed job is skipped is skipped (workflow-syntax.md:1260)

```text
Use `jobs.<job_id>.needs` to identify any jobs that must complete successfully before this job will run. It can be a string or array of strings. If a job fails or is skipped, all jobs that need it are skipped unless the jobs use a conditional expression that causes the job to continue. If a run contains a series of jobs that need each other, a failure or skip applies to all jobs in the dependency chain from the point of failure or skip onwards. If you would like a job to run even if a job it is dependent on did not succeed, use the `always()` conditional expression in `jobs.<job_id>.if`.
```
