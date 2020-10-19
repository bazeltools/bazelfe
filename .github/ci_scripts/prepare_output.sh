set -e

#!/usr/bin/env bash


ARTIFACT_NAME=$1
OUTPUT_PATH=$2
BINARY=$3


GENERATED_SHA_256=$(shasum -a 256 $BINARY | awk '{print $1}')

if [ ! -d $OUTPUT_PATH ]; then 
    mkdir $OUTPUT_PATH
fi

mv $BINARY $OUTPUT_PATH/${ARTIFACT_NAME}
echo $GENERATED_SHA_256 > $OUTPUT_PATH/${ARTIFACT_NAME}.sha256
