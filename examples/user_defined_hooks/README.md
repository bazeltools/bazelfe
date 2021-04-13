# Example project to show some user defined hooks capability

### Reset demo state
This demo has no meaningful state, bazel clean followed by bazel build should re-run.

## Log proto unused dependencies warnings

`src/main/proto/foo/bar/baz/a.proto`

This file has an import for `src/main/proto/foo/bar/baz/b.proto` which is unused. 

If you run:
`./bazelisk build src/main/proto/foo/bar/baz/...`

we should see output in `/tmp/run_log`.

This is based on the config in `tools/bazelfe_config`, and is flagged enabled by the `--config` option in `tools/bazel`.


The format of the config file is (currently!) a list of error processors, in the toml format. 

List entries are denoted by : `[[error_processors]]`

E.g.:

```
 [[error_processors]]
 name = "Identifying unused proto imports"
 active_action_type = "proto_library"
 run_on_success = true
 regex_match =  '^(.*):(\d+):(\d+): warning: Import (.*) but not used.$'
 target_command_line = '''
    /bin/bash -c "echo '{1}' "{2}" >> /tmp/run_log"
 '''

 [[error_processors]]
 name = "Second processor"
 active_action_type = "proto_library"
 regex_match =  '^(.*):(\d+):(\d+): warning: Import (.*) but not used.$'
 target_command_line = '''
    /bin/bash -c "echo '{1}' "{2}" >> /tmp/run_log"
 '''
```

Fields:
- name, this is the human consumable name that the tooling will include in outputs about actions
- active_action_type, this is the mnemonic for the action to bazel. It must be supplied since to run an action globally is thought to be poor for performance and likely to result in bad activations.
- regex_match, the regex match to perform against stdout/stderr outputs from the action. This is using the rust regex library for more examples, though common forms all seem to work well here.
- target_command_line, this is what to run when a match has occured based on the previous conditions. The regex matches can be referred to based on capture number `{_idx}`, e.g. `{1}`. Indexing of the captures themselves starts at 1, the full input line that matched will be `{0}`.