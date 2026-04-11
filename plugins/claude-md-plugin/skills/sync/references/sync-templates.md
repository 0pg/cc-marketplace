# Sync Templates

## Synthetic Plan.md Template

```markdown
# Spec Plan
target_path: {path}
action: update
round: 1

## Proposed Requirements
- REQ-1: {existing requirement 1}
- REQ-2: {existing requirement 2}
- REQ-3: {new or modified requirement}

## Proposed Constraints
- CONST-1: {existing constraint 1}
- CONST-2: {existing constraint 2}

## Rationale
- Sync: Requirements changed, Constraints need corresponding update
- Preserve unaffected Constraints verbatim
- Changed Requirements: REQ-2 (modified), REQ-3 (added)
```

## Spec Execute Session Template

```markdown
# Spec Execute Session
type: spec-execute | mode: execute | project_root: /path/to/project
target_path: src/auth
action: update
document_language: English

## Approved Plan File
plan_file: /tmp/claude-md/session-id/spec-plan-src-auth.md

## User Requirement
Sync: update DEVELOPERS.md Constraints for changed Requirements

## Existing Modules Index
{scan-claude-md output}

## Project Conventions
{resolved conventions or "None"}
```

## Restore Warning Format

```
[SYNC RESTORE] Restored Technical Context: agent modified preserved section
[SYNC RESTORE] Restored Decision Log: agent modified preserved section
```
