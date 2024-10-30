#include <babeltrace2/babeltrace.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

// Data structure to hold the Babeltrace components
typedef struct {
  bt_graph *graph;
} trace_context;

struct sink {
  bt_graph_simple_sink_component_initialize_func initialize_func;
  bt_graph_simple_sink_component_consume_func consume_func;
  bt_graph_simple_sink_component_finalize_func finalize_func;
  void *user_data;
};

// Initialize trace context with a trace file
trace_context *init_trace(const char *trace_path, const struct sink *sink_def) {
  bt_logging_level log_level = BT_LOGGING_LEVEL_INFO;

  trace_context *ctx = malloc(sizeof(*ctx));
  if (!ctx) {
    fprintf(stderr, "Failed to allocate trace context\n");
    return NULL;
  }

  // Create a new Babeltrace graph
  ctx->graph = bt_graph_create(0);
  if (!ctx->graph) {
    fprintf(stderr, "Failed to create Babeltrace graph\n");
    free(ctx);
    return NULL;
  }

  const bt_plugin *ctf_plugin = NULL;
  {
    bt_plugin_find_status status =
        bt_plugin_find("ctf", 1, 1, 1, 1, 1, &ctf_plugin);
    if (status != BT_PLUGIN_FIND_STATUS_OK) {
      fprintf(stderr, "Failed to find CTF plugin\n");
      BT_GRAPH_PUT_REF_AND_RESET(ctx->graph);
      free(ctx);
      return NULL;
    }
  }

  const bt_plugin *utils_plugin = NULL;
  {
    bt_plugin_find_status status =
        bt_plugin_find("utils", 1, 1, 1, 1, 1, &utils_plugin);
    if (status != BT_PLUGIN_FIND_STATUS_OK) {
      fprintf(stderr, "Failed to find utils plugin\n");
      BT_PLUGIN_PUT_REF_AND_RESET(ctf_plugin);
      BT_GRAPH_PUT_REF_AND_RESET(ctx->graph);
      free(ctx);
      return NULL;
    }
  }

  const bt_component_class_source *source_class =
      bt_plugin_borrow_source_component_class_by_name_const(ctf_plugin, "fs");

  const bt_component_class_filter *filter_class =
      bt_plugin_borrow_filter_component_class_by_name_const(utils_plugin,
                                                            "muxer");

  // Add a source component to read the trace
  bt_value *inputs = bt_value_array_create();
  bt_value *params = bt_value_map_create();

  bt_value_array_append_string_element(inputs, trace_path);
  bt_value_map_insert_entry(params, "inputs", inputs);

  const bt_component_source *src = NULL;
  {
    bt_graph_add_component_status status = bt_graph_add_source_component(
        ctx->graph, source_class, "input", params, log_level, &src);
    if (status != BT_GRAPH_ADD_COMPONENT_STATUS_OK) {
      fprintf(stderr, "Failed to add source component\n");
      BT_PLUGIN_PUT_REF_AND_RESET(ctf_plugin);
      BT_GRAPH_PUT_REF_AND_RESET(ctx->graph);
      free(ctx);
      return NULL;
    }

    BT_VALUE_PUT_REF_AND_RESET(inputs);
    BT_VALUE_PUT_REF_AND_RESET(params);
  }

  uint64_t src_out_port_count = bt_component_source_get_output_port_count(src);
  if (src_out_port_count == 0) {
    fprintf(stderr, "Source component has no output ports\n");
    exit(100);
    return NULL;
  }

  const bt_component_filter *muxer = NULL;
  {
    bt_graph_add_component_status status = bt_graph_add_filter_component(
        ctx->graph, filter_class, "muxer", NULL, log_level, &muxer);
    if (status != BT_GRAPH_ADD_COMPONENT_STATUS_OK) {
      fprintf(stderr, "Failed to add filter component\n");
      exit(100);
      return NULL;
    }
  }

  for (uint64_t i = 0; i < src_out_port_count; i++) {
    const bt_port_output *src_output =
        bt_component_source_borrow_output_port_by_index_const(src, i);

    const bt_port_input *muxer_input =
        bt_component_filter_borrow_input_port_by_index_const(muxer, i);

    {
      bt_graph_connect_ports_status status =
          bt_graph_connect_ports(ctx->graph, src_output, muxer_input, NULL);
      if (status != BT_GRAPH_CONNECT_PORTS_STATUS_OK) {
        fprintf(
            stderr,
            "Failed to connect source and filter components. Port index %lu\n",
            i);
        exit(100);
        return NULL;
      }
    }
  }

  // const bt_plugin *sink_plugin = NULL;
  // {
  //   bt_plugin_find_status status =
  //       bt_plugin_find("proxy", 0, 0, 0, 1, 1, &sink_plugin);
  //   if (status != BT_PLUGIN_FIND_STATUS_OK) {
  //     fprintf(stderr, "Failed to find proxy plugin\n");
  //     exit(100);
  //     return NULL;
  //   }
  // }

  // const bt_component_class_sink *sink_class =
  //     bt_plugin_borrow_sink_component_class_by_name_const(sink_plugin,
  //                                                         "output");

  // const bt_component_sink *sink = NULL;
  // {
  //   bt_graph_add_component_status status = bt_graph_add_sink_component(
  //       ctx->graph, sink_class, "output", NULL, log_level, &sink);
  //   if (status != BT_GRAPH_ADD_COMPONENT_STATUS_OK) {
  //     fprintf(stderr, "Failed to add sink component\n");
  //     exit(100);
  //     return NULL;
  //   }
  // }

  const bt_component_sink *simple_sink = NULL;
  {
    bt_graph_add_component_status status = bt_graph_add_simple_sink_component(
        ctx->graph, "simple sink", sink_def->initialize_func,
        sink_def->consume_func, sink_def->finalize_func, sink_def->user_data,
        &simple_sink);

    if (status != BT_GRAPH_ADD_COMPONENT_STATUS_OK) {
      fprintf(stderr, "Failed to add sink component\n");
      exit(100);
      return NULL;
    }
  }

  // Connect the source component to a message iterator
  const bt_port_output *muxer_output =
      bt_component_filter_borrow_output_port_by_index_const(muxer, 0);

  const bt_port_input *sink_input =
      bt_component_sink_borrow_input_port_by_index_const(simple_sink, 0);

  {
    bt_graph_connect_ports_status status =
        bt_graph_connect_ports(ctx->graph, muxer_output, sink_input, NULL);
    if (status != BT_GRAPH_CONNECT_PORTS_STATUS_OK) {
      fprintf(stderr, "Failed to connect muxer and sink components\n");
      exit(100);
      return NULL;
    }
  }

  BT_PLUGIN_PUT_REF_AND_RESET(ctf_plugin);
  BT_PLUGIN_PUT_REF_AND_RESET(utils_plugin);

  return ctx;
}

// Fetch the next event from the trace
bt_graph_run_once_status next_events(trace_context *ctx) {
  bt_graph_run_once_status run_status = BT_GRAPH_RUN_ONCE_STATUS_AGAIN;

  while (run_status == BT_GRAPH_RUN_ONCE_STATUS_AGAIN) {
    run_status = bt_graph_run_once(ctx->graph);
  }

  return run_status;
}

// Clean up the trace context
void destroy_trace_context(trace_context *ctx) {
  if (ctx->graph) {
    BT_GRAPH_PUT_REF_AND_RESET(ctx->graph);
  }
  free(ctx);
}