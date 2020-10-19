# Example project to show a sample usage of the tools

### Reset demo state
To reset state we would just need to remove the index between runs, in this demo its stored @ `/tmp/bazelfe_current_index`
So:
```
rm -f /tmp/bazelfe_current_index
```

## Heuristic test

`src/main/java/com/example/a/ExampleA.java`

This class has a dependency on `ExampleB`, but its not declared in its build file.

If you run:
`./bazelisk build src/main/java/com/example/a`

The build should automatically repair itself and build this target.

This is based on the tooling 'guessing' the location of b based on the class being looked for.

## Test B

Building the `com.example.c.ExampleC`:

```
./bazelisk build src/main/java/com/example/c
```

This depends upon `com.example.foo.BarExample` , however this is not in a target called `foo` in the `com/example/foo` folder, its in a target called bar. So the heuristic approach we use will fail. We need to improve some of the heuristics here but this looks close enough to possible today that it will :
-> Add the dependency
-> Catch the error that the dependency is invalid and remove it when it re-runs the build
-> Effectively return to the original error having given up

We can trigger however the index building, in the real world you would likely want to run this in CI/distribute this to your developers lazily rebuilt.

`./tools/run_bazel_fe_jvm_indexer.sh`

Then rerunning the build should just work.
