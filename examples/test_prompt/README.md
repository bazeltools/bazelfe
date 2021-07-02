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


### Suggest a test target to use

For this to operate you must be using a build which includes the daemon code. It should be available in the releases. This example on master will by default use this build.

```toml
[CommandLineRewriter]
  [CommandLineRewriter.test]
    type = 'SuggestTestTarget'

[DaemonConfig]
  enabled = true
```

Putting this in `./tools/bazelfe_config` as the config for the commandline rewriter will enable the rewriting such that running:

`./bazelisk test`

Will initially fail with a message, that no suggestions are available. However, if you edit `src/test/java/com/example/CatTest.java`, and rerun the operation, you should get a suggestion to test the target that file is owned by. Small caveat, is if the bazel queries are still running, no suggestions maybe returned. So in a fresh bazel there can be a delay, in the examples here within 2-4 seconds of editing that file the information should be populated however.
