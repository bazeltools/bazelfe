# Example project to show user test prompt behaviors

### Reset demo state
This demo has no meaningful state, though a longer running daemon is present to monitor for changes.
Running `./bazelisk shutdown` will ensure its shutdown along with the bazel version in memory.


## Handling user inputs to rewrite/intercept the command line

### Rewrite test with no target to testing the local repo

```toml
[CommandLineRewriter]
  [CommandLineRewriter.test]
    type = 'EmptyTestToLocalRepo'
```

Putting this in `./tools/bazelfe_config` as the config for the commandline rewriter will enable the rewriting such that running:

`./bazelisk test`

actually runs:

`./bazelisk test //...`

### Rewrite test with no target to testing the local repo

```toml
[CommandLineRewriter]
  [CommandLineRewriter.test]
    type = 'EmptyTestToFail'
```

Putting this in `./tools/bazelfe_config` as the config for the commandline rewriter will enable the rewriting such that running:

`./bazelisk test`

Will fail with an error message about specifying a test target.




