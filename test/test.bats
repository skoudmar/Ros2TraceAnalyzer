#!/usr/bin/env bats

set -u

trace_start() {
    rm -rf "$TRACE" # delete old trace to prevent "ros2 trace start" error
    ros2 trace start -u 'ros2:*' 'r2r:*' --path "$TRACE_DIR" "$ROS_DISTRO-$BATS_TEST_NAME"
}

trace_stop() {
    ros2 trace stop "$ROS_DISTRO-$BATS_TEST_NAME"
}

setup() {
    # get the containing directory of this file
    # use $BATS_TEST_FILENAME instead of ${BASH_SOURCE[0]} or $0,
    # as those will point to the bats executable's location or the preprocessed file respectively
    DIR="$( cd "$( dirname "$BATS_TEST_FILENAME" )" >/dev/null 2>&1 && pwd )"
    # Make Ros2TraceAnalyzer visible to PATH
    PATH="$DIR/../target/debug:$PATH"
    TRACE_DIR=$BATS_TEST_DIRNAME/traces/
    TRACE=$TRACE_DIR/$ROS_DISTRO-$BATS_TEST_NAME

    trace_start
}

teardown() {
    trace_stop
    Ros2TraceAnalyzer analyze "$TRACE"
}

@test "talker listener" {
    timeout -p -s SIGINT 3s ros2 launch demo_nodes_cpp talker_listener_launch.xml
}

@test "add_two_ints service" {
    ros2 launch "$BATS_TEST_DIRNAME/add_two_ints_launch.xml"
}
