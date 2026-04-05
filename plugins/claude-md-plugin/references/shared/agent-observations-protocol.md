# Agent Observations Protocol

## On Start (Read Phase)

1. Read `{target_path}/DEVELOPERS.md` → locate `## Agent Observations` section
2. If section exists and is not `None`:
   - Filter entries whose `anchor` matches current work's REQ-N/CONST-N
   - Load all `[preference]` entries (always relevant)
   - For each matched entry: increment `refs` by 1 and update in file
3. Apply matched observations as context for current work

## During Work (Collect Phase)

4. When encountering any of the following, note as observation candidate:
   - Unexpected problem and its solution → `[structural]` or `[tactical]`
   - Technical choice with rationale → `[decision]`
   - User-expressed preference → `[preference]`

## On Complete (Write Phase)

5. For each observation candidate:
   - Check if a similar entry already exists (same anchor + similar content)
     - If exists: increment `refs`, optionally enrich content
     - If not: create new entry with required fields
6. Entry format:
   ```markdown
   ### [type] concise title
   - anchor: REQ-N or CONST-N (omit if module-wide)
   - since: YYYY-MM-DD
   - refs: 1
   - source: /{workflow} {agent-name}
   - Description of what was observed and why it matters.
   ```
7. If entries > 20: run consolidation
   - Remove `[tactical]` entries with refs=0 and age > 30 days
   - Merge duplicate-anchor same-type entries (sum refs, keep earliest since)
8. Write changes to `## Agent Observations` section ONLY (INV-8)
   - If section doesn't exist yet, create it at the end of DEVELOPERS.md
   - Never modify other DEVELOPERS.md sections

## Constraints

- **INV-8**: Write target must be `## Agent Observations` only
- **INV-9**: Anchors must reference existing REQ-N/CONST-N (or be omitted)
- No user approval needed for Agent Observations writes
- Maximum 20 entries before consolidation trigger
