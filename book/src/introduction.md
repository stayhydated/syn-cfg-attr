# Introduction

`syn-cfg-attr` lets procedural macros and code-generation tools inspect an
attribute the same way whether a user wrote it directly or nested it in
`cfg_attr`. Recursive expansion returns the nested attributes while preserving
the condition that guarded each one.

Expansion preserves syntax independently of the active target or feature set.
It returns direct attributes and expands nested entries, combining their guards
with `all(...)`. Choose fallible expansion to report malformed entries or
best-effort expansion to keep the parseable ones.

Use the crate when your tool needs to:

- find one attribute identifier across direct and conditional forms;
- parse list arguments from either form through one API;
- retain a combined guard when generating output or diagnostics;
- evaluate a guard against configuration state owned by the caller.

`CfgPredicate::evaluate` reads target flags, feature values, and custom `cfg`
state through a callback. Supply the configuration for the code being
inspected; a procedural macro's host can have a different target or feature set.

Start with [Get started](getting-started.md), then choose expansion, condition,
and error behavior for your integration.
