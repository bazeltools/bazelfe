#!/bin/bash
set -e

TOOLS_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
export REPO_ROOT="$(cd $TOOLS_DIR && cd .. && pwd)"


source $REPO_ROOT/tools/bazel_fe_common.sh

1>&2 fetch_binary "$JVM_INDEXER_LOCAL_PATH" "$JVM_INDEXER_URL" "$JVM_INDEXER_SHA_URL"


exec "$JVM_INDEXER_LOCAL_PATH" --index-output-location ${INDEX_INPUT_LOCATION} --bazel-binary-path "$REPO_ROOT/bazelisk" --blacklist-remote-roots io_bazel_rules_scala
