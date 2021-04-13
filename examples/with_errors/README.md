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
- Add the dependency
- Catch the error that the dependency is invalid and remove it when it re-runs the build
- Effectively return to the original error having given up

We can trigger however the index building, in the real world you would likely want to run this in CI/distribute this to your developers lazily rebuilt.

`./tools/run_bazel_fe_jvm_indexer.sh`

Then rerunning the build should just work.


## Test B, learning mode

Reset the state of our working tree to our broken targets
```
git checkout src
rm -f /tmp/bazelfe_current_index
```

The bazel runner where possble will update/learn its index as it goes. Its good to not let this probably get too stale since we don't _delete_ entries which needs to be improved upon to better handle refactoring. (Replacing daily with the index as mentioned above likely suffices for most use cases).

```
./bazelisk build src/main/java/com/example/c
```

This should fail like before.

Now do:

```
./bazelisk build src/main/java/com/example/foo/...
```

And finally,

```
./bazelisk build src/main/java/com/example/c
```

Should now be a success!
Note: You could do `./bazelisk build src/main/java/com/example/...` and depending on the order bazel runs the build/things being available this might just work too. (The index will get the entry before the failure is processed for entry c). Its a race condition, but a nice happenstance when it happens!


## Refactoring

Lets first reset the state:
```
git checkout src
rm -f /tmp/bazelfe_current_index
```

Then seed/build the targets:

```
./bazelisk build src/main/java/com/example/foo/...
./bazelisk build src/main/java/com/example/c
```

Now the bazel runner has 'learned' where the foo target is. But now lets correct our poor labeling of the target name:

Edit `src/main/java/com/example/foo/BUILD` and change `bar` to `foo` as the target's name.


Lets now try build c again, which should be depending on a non existant target...
```
./bazelisk build src/main/java/com/example/c/...
```

Our build will now fail, and we should get a story about the failure like:

```
Build still failed. Active stories about failed targets/what we've tried:
Target: //src/main/java/com/example/c:c
  Removed Dependency //src/main/java/com/example/foo:bar, because: Dependency on does not exist
  Added Dependency //src/main/java/com/example/foo:bar, because: Saw a missing dependency error
  Removed Dependency //src/main/java/com/example/foo:bar, because: Dependency on does not exist
```

But lets build our new foo again:

```
./bazelisk build src/main/java/com/example/foo/...
./bazelisk build src/main/java/com/example/c
```

And it should all succeed!  Today we don't purge the index of the outputs of the previous label/target building, but we do learn concrete class locations (vs packages which are more heuristic) from the rebuild/refactor.
