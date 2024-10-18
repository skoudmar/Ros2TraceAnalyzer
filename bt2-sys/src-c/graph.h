#include <babeltrace2/babeltrace.h>

typedef struct trace_context trace_context;

struct sink {
  bt_graph_simple_sink_component_initialize_func initialize_func;
  bt_graph_simple_sink_component_consume_func consume_func;
  bt_graph_simple_sink_component_finalize_func finalize_func;
  void *user_data;
};

trace_context *init_trace(const char *trace_path, const struct sink *sink_def);

bt_graph_run_once_status next_events(trace_context *ctx);

void destroy_trace_context(trace_context *ctx);
