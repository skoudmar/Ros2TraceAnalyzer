# Ros2TraceAnalyzer

Ros2TraceAnalyzer is a command-line tool to extract useful data from LTTng traces of ROS application.

## Supported ROS versions

- Jazzy

## Installation

First make sure you have development version of Babeltrace 2 library. It can be installed on Ubuntu using:

```sh
apt install libbabeltrace2-dev
```

The Ros2TraceAnalyzer can then be compiled and installed using `cargo` by running:

```sh
cargo install --git https://github.com/skoudmar/Ros2TraceAnalyzer.git
```

## Usage

```
Usage: Ros2TraceAnalyzer [OPTIONS] <TRACE_PATHS>... <COMMAND>

Commands:
  message-latency   Analyze the latency of the messages
  callback          Analyze the callback duration and inter-arrival time
  utilization       Analyze the utilization of the system based on the
                    quantile callback durations
  utilization-real  Analyze the utilization of the system based on the
                    real execution times
  dependency-graph  Construct a detailed dependency graph
  all               Run all analyses
  help              Print this message or the help of the given
                    subcommand(s)

Arguments:
  <TRACE_PATHS>...  Path to a directory containing the trace to analyze
```

**Message latency** and **Callback** analyses options:
```
--quantiles <QUANTILES>...    Print results with these quantiles.
--json-dir  <JSON_DIR_PATH>   Instead of printing aggregated results, export the measured
                              data into a JSON file in the specified directory.
```

**Utilization** analysis option:
```
--quantile <QUANTILE>   Quantile used for callback duration calculation.
```

**Detailed dependency graph** options:
```
-o, --output-path <OUTPUT_PATH>   mandatory option specifying the output directory for the graph
--color                           Color edges based on the edge weight.
--thickness                       Set edge thickness based on edge weight.
--min-multiplier <MIN_MULTIPLIER> Set the maximum value of the color or thickness range to be
                                  lower bounded by MIN_MULTIPLIER times the minimum value.
```