# Issue tracker

Issues for this repo live in **GitHub Issues**, on `mauricedesaxe/claude-harness-template`.

Skills that read from or write to the tracker (`to-tickets`, `triage`, `to-spec`, `wayfinder`,
and the `lazar-*` skills) use the `gh` CLI:

- read: `gh issue list`, `gh issue view <n>`
- write: `gh issue create`, `gh issue edit <n>`
- link: `Closes #<n>` in a PR body

**PRs as a request surface:** off. External PRs do not enter the triage queue.

## Triage labels

The five canonical roles, each label string equal to its name:

`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`

Only `ready-for-agent` exists on the repo so far; the rest are created on first use.

## Spec approval marker

After the product owner approves the final spec body, hash the body as canonical compact JSON. This
avoids differences in terminal newline handling:

```sh
spec=<spec-number>
if command -v sha256sum >/dev/null; then
  body_sha=$(gh issue view "$spec" --json body | jq -cS . | sha256sum | cut -d ' ' -f1)
else
  body_sha=$(gh issue view "$spec" --json body | jq -cS . | shasum -a 256 | cut -d ' ' -f1)
fi
printf 'Approved-Spec-Body-SHA256: %s\n' "$body_sha" | \
  gh issue comment "$spec" --body-file -
```

A fresh Orchestrate session computes `body_sha` with the same command. It reads all spec comments and
uses the newest exact `Approved-Spec-Body-SHA256: <hash>` marker. The marker admits the spec only when
its hash equals `body_sha`. Any later body edit invalidates the marker. Product shaping must obtain
product approval again and post a new marker.

## Wayfinding operations

Matt Wayfinder uses GitHub Issues for the product-decision map. The map has the `wayfinder:map`
label. Each child issue has one `wayfinder:<type>` label. Create missing labels with
`gh label create`.

Set the repository once:

```sh
repo=mauricedesaxe/claude-harness-template
```

Use this sequence:

1. Create the map and its decision issues with `gh issue create`. Get each decision issue's numeric
   database ID:

   ```sh
   gh api "repos/$repo/issues/<decision-number>" --jq .id
   ```

   Add the decision issue to the map:

   ```sh
   gh api --method POST "repos/$repo/issues/<map-number>/sub_issues" \
     -F sub_issue_id=<decision-database-id>
   ```

2. If the sub-issues endpoint is unavailable, add a `## Decision issues` checklist to the map. Each
   item links one decision issue. This list is the fallback parent relation. Do not use the fallback
   when the native endpoint works.

3. Wire native dependencies after all issues exist. Get the blocking issue's numeric database ID
   with the first command. Then add the dependency:

   ```sh
   gh api --method POST \
     "repos/$repo/issues/<blocked-number>/dependencies/blocked_by" \
     -F issue_id=<blocking-database-id>
   ```

   Issue numbers are not database IDs.

4. Query the frontier from the map's children:

   ```sh
   gh api --paginate "repos/$repo/issues/<map-number>/sub_issues?per_page=100"
   gh api --paginate \
     "repos/$repo/issues/<decision-number>/dependencies/blocked_by?per_page=100"
   ```

   Keep open, unassigned children with no open blockers. With the fallback parent relation, read the
   linked checklist first. Apply the same state, assignee, and dependency checks.

5. Claim a frontier issue before work:

   ```sh
   gh issue edit <number> --add-assignee @me
   gh issue view <number> --json assignees,state
   ```

   Skip the issue if another assignee won the claim.

6. Resolve one decision in sequence. Post the full answer, then close the issue:

   ```sh
   gh issue comment <number> --body-file <file>
   gh issue close <number> --reason completed
   ```

   Add one linked gist to the map's `Decisions so far`. Create newly clear decision issues. Attach
   them to the map, wire their dependencies, and refresh `Not yet specified`.

GitHub owns the approved product spec, approval comments, Wayfinder map, and decision issues. A
sandbox Orchestrate run may derive one Beads implementation graph from them. Do not create a second
GitHub implementation graph for that run.
