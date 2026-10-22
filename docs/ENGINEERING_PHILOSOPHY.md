# Engineering Philosophy — Evidence-Driven Software Development

**Version:** 1.0  
**Status:** Proposed  
**Purpose:** Minimize implementation errors, regressions, unjustified assumptions, and false claims of completion when using AI-assisted software development.

## 1. Foundational Principle: Reality Over Assumption

The repository, its dependencies, the execution environment, the established requirements, and verified system behavior are the sources of truth.

Never substitute assumptions, plausible explanations, remembered code, conventional patterns, or generated output for evidence about the actual system.

Before changing anything, establish what exists, what is required, and what can be verified.

When evidence is missing, identify the uncertainty explicitly. Investigate it, constrain the implementation, or request a decision from the appropriate authority.

**A plausible solution is not necessarily a correct solution.**

## 2. Requirements Before Implementation

Every change must have a clear purpose, scope, and acceptance criteria.

Before implementation:

1. Identify the problem being solved.
2. Identify the authoritative requirements.
3. Inspect the existing implementation and its dependencies.
4. Identify invariants, constraints, and affected components.
5. Identify unresolved decisions and assumptions.
6. Determine how correctness will be verified.

Do not begin implementation when an unresolved decision materially affects architecture, security, data integrity, compatibility, or business behavior.

Distinguish between requirements, approved decisions, proposals, assumptions, and implementation details. Never silently promote a proposal or assumption into an approved requirement.

## 3. Understand Before Changing

Do not modify code based solely on a filename, a search result, a partial snippet, or a presumed architecture.

Inspect the relevant implementation, its callers, its dependencies, its tests, its persistence behavior, and its operational context.

Trace important behavior across system boundaries.

Prefer the smallest investigation that establishes sufficient understanding, but continue investigating when new evidence reveals additional dependencies or risks.

Never claim that a behavior exists, a defect is fixed, or a requirement is satisfied without sufficient evidence.

## 4. Preserve Invariants

An invariant is a condition that must remain true before, during, and after an operation.

Identify relevant invariants before modifying a system. Examples include:

- Data integrity and referential integrity.
- Valid state transitions.
- Authorization and tenant isolation.
- Transaction atomicity.
- Idempotency and duplicate prevention.
- Backward compatibility.
- Auditability and retention requirements.
- Resource limits and concurrency guarantees.

Treat invariants as stronger than implementation convenience.

Prefer designs that make invalid states difficult or impossible to represent. Enforce critical guarantees at the strongest appropriate boundary, such as the type system, database constraints, transaction protocol, or authorization layer.

Do not weaken an invariant to make an implementation easier.

## 5. Minimize the Change Surface

Every additional change introduces another opportunity for error.

Implement the smallest coherent change that fully satisfies the requirements.

Avoid unrelated refactoring, speculative abstractions, unnecessary dependencies, opportunistic renaming, and scope expansion.

Small does not mean incomplete. A change is acceptable only when it satisfies the entire approved scope.

Prefer a narrow, complete, verifiable implementation over a broad implementation whose interactions cannot be adequately understood or tested.

## 6. Treat Failure as a Normal Operating Condition

Do not design only for the happy path.

Consider invalid input, missing records, concurrent requests, timeouts, partial failures, retries, duplicate events, cancellation, process termination, database errors, and unavailable external services where relevant.

Define what happens when each important operation fails.

Preserve enough information to diagnose and recover from failures. Do not swallow errors, invent successful outcomes, or retry operations without understanding their side effects.

For distributed operations, never assume that a database transaction automatically makes external side effects atomic.

## 7. Verify the Actual Guarantee

Verification must match the claim being made.

Use the appropriate evidence:

- Static analysis for statically detectable defects.
- Unit tests for local behavior and domain invariants.
- Integration tests for component interactions.
- Real-database tests for constraints, transactions, locking, and isolation.
- Concurrency tests for race-sensitive guarantees.
- End-to-end tests for complete workflows.
- Staging verification for deployment-dependent behavior.

A test passing proves only what that test actually exercises.

Do not infer production correctness from compilation alone, unit tests alone, mocks alone, or an implementation that appears logically sound.

When verification cannot be performed, state the limitation and what remains unverified.

## 8. Evidence Before Completion

A task is complete only when:

1. The approved requirements have been addressed.
2. The implementation matches the actual repository and system contracts.
3. Relevant tests and quality checks have been executed.
4. Results have been inspected, not merely assumed.
5. Regressions and material risks have been considered.
6. Remaining limitations are explicitly documented.

Distinguish clearly between:

- **Implemented:** Code has been written.
- **Tested:** Relevant tests have run.
- **Verified:** The specified behavior has been demonstrated with appropriate evidence.
- **Accepted:** The designated decision-maker has approved the result, where approval is required.

These statuses are not interchangeable.

Never claim a command passed if it was not executed. Never claim a test passed if its result was not inspected. Never describe unverified work as production-ready.

## 9. Honest Uncertainty

When evidence is insufficient, do not manufacture certainty.

State:

- What is known.
- What remains unknown.
- Which assumptions are being made.
- What evidence would resolve the uncertainty.
- Whether proceeding is safe.

Ask for clarification when an unresolved decision requires human judgment or authority.

For low-risk, reversible decisions, a reasonable assumption may be used if it is explicitly identified, consistent with established requirements, and easy to correct.

For high-impact or irreversible decisions, prefer stopping at the decision boundary over silently guessing.

## 10. Protect Against Regression

Correctness includes preserving behavior that must not change.

Before modifying existing behavior, identify dependent callers, interfaces, stored data, tests, migrations, and operational processes.

Add regression tests for the defect or invariant whenever practical.

Do not remove or weaken a test merely because it fails after a change. Determine whether the test exposes a real defect, an obsolete requirement, or an incorrect expectation before modifying it.

Treat unexpected test failures as evidence requiring investigation, not as obstacles to completing a task.

## 11. Prefer Explicitness Over Hidden Behavior

Prefer code and interfaces whose contracts are visible and understandable.

In Rust, use appropriate types, ownership, explicit error handling, and well-defined module boundaries.

Across languages, prefer clear preconditions, postconditions, state transitions, side effects, and failure semantics.

Avoid implicit global state, ambiguous ownership, hidden network calls, surprising mutation, undocumented defaults, and abstractions that conceal important correctness properties.

Do not confuse verbosity with clarity or abstraction with good design. The objective is to make important behavior easy to reason about.

## 12. Continuous Reassessment

A plan is a hypothesis about how to achieve a goal, not proof that the goal has been achieved.

After each material change:

1. Inspect the resulting diff.
2. Compare it against the requirements.
3. Reassess assumptions affected by the change.
4. Run the appropriate verification.
5. Investigate unexpected results.
6. Update the plan when evidence contradicts it.

If a change reveals a deeper architectural or requirements problem, do not conceal it with a local workaround. Determine whether the issue is within scope and escalate decisions that require additional authority.

## 13. Final Rule

Never optimize for appearing finished.

Optimize for producing a correct, understandable, maintainable result whose important properties are supported by evidence.

When speed conflicts with an essential correctness guarantee, preserve the guarantee.

When confidence conflicts with evidence, follow the evidence.

When implementation convenience conflicts with an approved invariant, preserve the invariant.

When a result cannot be verified, report it honestly.

**The measure of good engineering is not how confidently a solution is produced, but how reliably its correctness can be established.**