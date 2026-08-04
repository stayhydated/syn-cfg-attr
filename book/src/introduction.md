# Introduction

`syn-cfg-attr` lets procedural macros and code-generation tools inspect an
attribute the same way whether a user wrote it directly or nested it in
`cfg_attr`. Recursive expansion returns the nested attributes while preserving
the condition that guarded each one.

Expansion is syntax-oriented: it does not decide whether a guard is active for
a target or feature set. Direct attributes and every parseable nested entry are
available for inspection. Nested entries carry their raw condition tokens, and
nested guards are combined with `all(...)`.

Use the crate when your tool needs to:

- find one attribute identifier across direct and conditional forms;
- parse list arguments from either form through one API;
- retain a combined guard when generating output or diagnostics;
- evaluate a guard against configuration state owned by the caller.

`CfgPredicate::evaluate` obtains target flags, feature values, and custom cfg
state through a callback. Supply the configuration for the code being
inspected rather than inferring it from the host process.

Start with [Get started](getting-started.md), then choose expansion, condition,
and error behavior for your integration.
