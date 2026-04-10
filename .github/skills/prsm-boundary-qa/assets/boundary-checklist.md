# Boundary Checklist

## Producer
- What file or command produced the data?
- What is the exact shape, path, span, or artifact name?

## Consumer
- What file or command consumed the data?
- What shape, path, span, or artifact name does it expect?

## Contract comparison
- Do both sides use the same file extension and filename?
- Do both sides agree on project root resolution?
- Do both sides agree on line and column bases?
- Do both sides agree on JSON field names?
- Do both sides agree on generated artifact location?
- Do both sides handle stale artifacts or fallback logic the same way?

## Evidence
- Which file or command proves the mismatch?
- Can the mismatch be observed with a narrower repro?

## Conclusion
- Is the boundary confirmed broken, or merely suspicious?
- What is the next smallest validation step?