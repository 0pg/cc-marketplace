# Domain Decomposition (v19 draft)

Heuristics for deciding **when** to split a node into child nodes, and
**where** to draw the boundary.

A node exists because its agent has a cohesive job. Add a child when a
sub-domain starts competing for that agent's prompt — different vocabulary,
different failure modes, different users.

## Primary Signals to Split

1. **Prompt pressure.** The node's `CLAUDE.md` is growing because it
   describes two or more distinct concerns. Readers have to mentally filter
   ("this section is about auth, that one is about billing"). That's a
   decomposition signal, not a "shorten the prompt" signal.

2. **Context-window pressure.** The agent frequently cannot finish a task in
   one pass because the node has too many tools to reason about together.
   Split the tools by domain and let each child agent hold only what it
   needs.

3. **Divergent vocabulary.** Two areas of the node use different nouns
   ("order", "invoice", "shipment" vs. "user", "session", "token") and
   rarely share code. Each vocabulary signals a domain.

4. **Independent evolution.** Two areas change on different cadences, owned
   by different concerns. Coupling them under one prompt makes each change
   touch the whole prompt.

5. **Distinct invariants.** Rules that apply to one half of the node but not
   the other force the current prompt to write "X applies only when…"
   conditionals. Promote each rule to its own child.

## Primary Signals to *Not* Split

1. **Small code, shared vocabulary.** Two tools that operate on the same
   nouns and share most helpers belong together.

2. **One-shot tasks.** A helper that runs once and has no domain around it
   is a tool, not a child.

3. **Layered abstractions of the same domain.** "Data access" + "service"
   + "HTTP handler" over the same entities is one domain in three layers.
   Keep it together unless the team genuinely treats the layers as separate
   concerns.

4. **Speculative future growth.** Do not pre-split because the area "might"
   grow. Split when the signals above actually appear.

## Drawing the Boundary

Once you decide to split, the split point should maximize **in-domain
coupling** and minimize **cross-node traffic**:

- Lines of code or file count is a weak signal. Follow the vocabulary and
  the dependency graph instead.
- A good child is self-sufficient: it can fulfill its responsibilities with
  its own tools + a well-defined parent contract, without reaching into
  siblings.
- If drawing the boundary forces frequent sibling-to-sibling calls, the
  boundary is wrong. Either merge the siblings or promote the shared piece
  to the parent.

## Anti-Patterns

- **Layered-by-technology split** (`models/`, `services/`, `controllers/`
  as top-level children) — creates prompts that are each fragments of the
  same domain. Use technology layers *within* a domain-cohesive child if at
  all.
- **Over-flattening** — flat trees with 20 child nodes under one parent.
  The parent can't summarize its children's roles in one prompt. Intermediate
  groupings are usually warranted.
- **Over-nesting** — single-child chains (`a/b/c/d/` where each level has
  exactly one sub-node). Collapse until every node has either siblings or
  tools that justify its existence.

## Boundary Changes Over Time

Splitting and merging are normal operations. Signals to merge:

- Two siblings end up with near-duplicate prompts.
- Most parent delegations go to the same child; the other child is barely
  used.
- A child's responsibilities shrink to a single tool.

Merging is a refactor like any other — do it when the signal is clear, not
preemptively.
