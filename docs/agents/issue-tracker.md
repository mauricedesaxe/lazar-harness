# Issue tracker

Issues for this repo live in **GitHub Issues**, on `mauricedesaxe/lazar-harness`.

- Trusted product approver GitHub logins: `mauricedesaxe`

Resolve the repository once before any tracker operation:

```sh
repo=$(gh repo view --json nameWithOwner --jq .nameWithOwner)
test -n "$repo"
```

Pass `-R "$repo"` to every `gh issue` and `gh label` command. Use `repos/$repo/...` for every
`gh api` path. Do not mix implicit-origin commands with a hardcoded API repository.

- Read with `gh issue list -R "$repo"` and `gh issue view <n> -R "$repo"`.
- Write with `gh issue create -R "$repo"` and `gh issue edit <n> -R "$repo"`.
- Link an issue with `Closes #<n>` in a PR body for the same repository.

**PRs as a request surface:** off. External PRs do not enter the triage queue.

## Triage labels

The five canonical labels are `needs-triage`, `needs-info`, `ready-for-agent`,
`ready-for-human`, and `wontfix`. Only `ready-for-agent` exists now. Create another label on first
use with `gh label create <label> -R "$repo"`.

## Spec approval marker

After the product owner approves the final spec body, keep the target repository as the current
directory. Invoke the helper through its absolute installed path:

```sh
target_repo=/path/to/target-repository
hash_helper=/absolute/path/to/pstack-poteto-mode/scripts/spec-body-hash.sh
cd "$target_repo"
body_sha=$("$hash_helper" "$spec")
repo=$(gh repo view --json nameWithOwner --jq .nameWithOwner)
approver=$(gh api user --jq .login)
test "$approver" = mauricedesaxe
printf 'Approved-Spec-Body-SHA256: %s\n' "$body_sha" | \
  gh issue comment "$spec" -R "$repo" --body-file -
```

The helper resolves `repo` from the current directory with `gh repo view`. It fetches the issue
through that same repository. Do not change into the installed skill directory before invocation.
A failed fetch, malformed response, empty body, or hash failure exits nonzero and produces no hash.

A fresh Orchestrate session runs the same helper. It fetches all comments in server order:

```sh
marker=$(gh api --paginate "repos/$repo/issues/$spec/comments?per_page=100" | jq -rs \
  --argjson trusted '["mauricedesaxe"]' '
  [.[][]
   | select(.user.login as $login | $trusted | index($login))
   | .body
   | select(test("\\AApproved-Spec-Body-SHA256: [0-9a-f]{64}\\z"))]
  | last // empty
')
```

The newest exact marker from a trusted product approver must equal
`Approved-Spec-Body-SHA256: $body_sha`. Ignore marker-shaped comments from other authors. A later
body edit makes approval stale. Product shaping must obtain approval again and post a new trusted
marker.

## Wayfinding operations

Matt Wayfinder uses GitHub Issues for the product-decision map. The map has the `wayfinder:map`
label. Each child has one `wayfinder:<type>` label.

Use this sequence:

1. Create the map and its decision issues with `gh issue create -R "$repo"`. Get each decision
   issue's numeric database ID:

   ```sh
   gh api "repos/$repo/issues/<decision-number>" --jq .id
   ```

   Add each decision issue to the map:

   ```sh
   gh api --method POST "repos/$repo/issues/<map-number>/sub_issues" \
     -F sub_issue_id=<decision-database-id>
   ```

2. If the sub-issues endpoint is unavailable, add a `## Decision issues` checklist to the map. Each
   item links one decision issue. This checklist is the fallback parent relation. Do not use it when
   the native endpoint works.

3. After all issues exist, wire native dependencies. Get the blocking issue's numeric database ID
   with the first API command. Then add the dependency:

   ```sh
   gh api --method POST \
     "repos/$repo/issues/<blocked-number>/dependencies/blocked_by" \
     -F issue_id=<blocking-database-id>
   ```

   Issue numbers are not database IDs.

4. Query every map child and every blocker page:

   ```sh
   gh api --paginate "repos/$repo/issues/<map-number>/sub_issues?per_page=100"
   gh api --paginate \
     "repos/$repo/issues/<decision-number>/dependencies/blocked_by?per_page=100"
   ```

   If the native child endpoint is unavailable, read every linked item under `## Decision issues`.
   Keep open children with no open blockers in the frontier.

5. Claim a frontier issue with one session-unique identifier. Use the runtime session ID when
   `WAYFINDER_SESSION_ID` is exposed. Otherwise, read a kernel UUID on Linux or generate one with
   `uuidgen`. Stop before posting a claim when no source produces a safe identifier. Keep `claim_id`
   unchanged through the claim and release command sequence:

   ```sh
   claim_id=${WAYFINDER_SESSION_ID:-}
   if [ -z "$claim_id" ]; then
     if [ -r /proc/sys/kernel/random/uuid ]; then
       IFS= read -r claim_id </proc/sys/kernel/random/uuid
     elif command -v uuidgen >/dev/null 2>&1; then
       claim_id=$(uuidgen) || exit 1
     else
       exit 1
     fi
   fi
   case "$claim_id" in
     ''|*[!A-Za-z0-9._:-]*) exit 1 ;;
   esac
   printf 'Wayfinder-Claim: %s\n' "$claim_id" | \
     gh issue comment <number> -R "$repo" --body-file -
   ```

   Fetch all comments in server order and select the earliest valid claim:

   ```sh
   claims=$(gh api --paginate \
     "repos/$repo/issues/<number>/comments?per_page=100" | jq -rs '
       [ .[][] | .body ] as $bodies
       | [ range(0; $bodies | length) as $i
           | $bodies[$i]
           | select(test("\\AWayfinder-Claim: [A-Za-z0-9._:-]+\\z"))
           | . as $claim
           | ($claim | sub("^Wayfinder-Claim: "; "")) as $id
           | select(($bodies[($i + 1):]
             | index("Wayfinder-Claim-Released: " + $id)) == null) ]
     ')
   first_claim=$(printf '%s' "$claims" | jq -r '.[0] // empty')
   test "$first_claim" = "Wayfinder-Claim: $claim_id"
   ```

   GitHub returns issue comments from oldest to newest. A valid claim is an exact
   `Wayfinder-Claim: <id>` comment without an exact `Wayfinder-Claim-Released: <id>` comment.
   Proceed only when this session owns the earliest valid claim. Assignment may identify the worker
   after the claim wins. It is not the lock.

   A losing session releases its own claim, then skips the issue and refreshes the frontier:

   ```sh
   printf 'Wayfinder-Claim-Released: %s\n' "$claim_id" | \
     gh issue comment <number> -R "$repo" --body-file -
   ```

   An owner can use the same command after it stops. A human or coordinator can release an abandoned
   claim after confirming that its session cannot resume.

6. Resolve one decision in sequence. Post a non-empty comment whose first line is
   `Wayfinder-Resolution:`. Then close the issue as completed:

   ```sh
   gh issue comment <number> -R "$repo" --body-file <resolution-file>
   gh issue close <number> -R "$repo" --reason completed
   ```

   Add one linked gist to the map's `Decisions so far`. Create new decision issues with
   `gh issue create -R "$repo"`. Attach them to the map, wire their dependencies, and refresh
   `Not yet specified`.

GitHub owns the approved product spec, approval comments, Wayfinder map, and decision issues. A
sandbox Orchestrate run may derive one Beads implementation graph from them. Do not create a second
GitHub implementation graph for that run.
