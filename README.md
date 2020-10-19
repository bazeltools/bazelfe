# Bazel runner

## Goal/motivation

Be a suite of tools to provide ultimately a different frontend around bazel. Initially this is something to be injected to wrap calls for build/test operations in bazel to repair common issues.

## Requirements

- Ability to run/provide a CI job or some means to run the index building code (Indexer itself in this repo isn't quite complete yet, eta ~10/15).
- Not be consuming the BEP from bazel directly. The tool hooks in and tells bazel to send the BEP out to it to sniff on the results.

## Using it

1. Configure a CI job to run the indexer, it should produce a binary output
2. Store the output in a location which is fetchable by your developers/users
3. Expose buildozer at a path(fetch or via https://doc.rust-lang.org/std/macro.include_bytes.html or https://github.com/pyros2097/rust-embed we could probably embed in the release to ease distribution.)
4. From the examples you need to install:
   -> Some code/bash script (could be built into the launcher in future?) to fetch the index to provide
   -> Bash script for tools/bazel to alloow hooking into the bazel commands and delegating to the `bazel-runner` application
5. Run it

Other things:
We also include/have a small script to measure how well it can do for you/potentially handle targets with unused dependencies. the `slow_unused_deps.sh` script will remove all dependencies from a target then try build it again. If the above is all working right, hopefully like magic it should just recover + build ok.

## TODO:

- [x] Bazel runner that can wrap bazel
- x ] JVM Indexer to find/index all jvm producing targets
  - [ ] Investigate TUI/tabbed interface when running tooling so that the bazel stdout/stderr can be preserved/viewed but not swamp things like indexing
  - [ ] Investigate using an aspect to gather the index information
- [x] Example project
- [x] All scripts in the right place
- [ ] Integration for auto formatting handling for java/scala
- [ ] Investigate persistant daemon mode:
  - [ ] When file changes rebuild the target that owns it
  - [ ] When the above is successful run tests that directly ddepend on the rebuilt target
  - [ ] Optionally run all tests that transitively depend on the target
- [ ] Build UI experiments using the TUI library to show better histograms/data while building.
- [ ] Web interface?

## Bazel runner

```
basic

USAGE:
    bazel-runner [OPTIONS] <passthrough-args>... --buildozer-path <buildozer-path>

ARGS:
    <passthrough-args>...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --bind-address <bind-address>                    [env: BIND_ADDRESS=]
        --buildozer-path <buildozer-path>                [env: BUILDOZER_PATH=]
        --index-input-location <index-input-location>    [env: INDEX_INPUT_LOCATION=]
```

Options:

- Bind adddress, optional, change the ip/port we bind our GRPC server to that we tell bazel to use for BEP
- buildozer path required for buildozer operations to allow making changes/sniffing dependencies in build files
- passthrough args, what the user called bazel with, e.g.:
  `/path/to/bazel-real build --flag --flag src/main/blah:wer`
