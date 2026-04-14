# Explore Result (candidate-only)

## Candidate Nodes
- .                          # project root always included
- core/src/foo               # included because it handles schema X which this requirement mentions
- core/src/bar               # included because the requirement references behavior listed in its Roadmap
- core/src/baz               # included: shares data flow with the requested behavior
