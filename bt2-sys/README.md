# Rust bindings for Babeltrace 2

The bindings use the C API internally.

## Required native dependencies

The `babeltrace2` library is required with development headers.

On ubuntu it can be installed with:

```sh
apt-get install libbabeltrace2-dev
```

## Derive macro

Derive macro for conversion from event payload field is provided separately in the [bt2-derive](../bt2-derive/) crate.
