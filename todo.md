# TODO

## Analysis

- [x] Inter-callback latency
- [x] Inter-message latency
- [X] Timer to callback latency
  - only if the spin wake time is known (so only for r2r)
- [ ] Investigate why there is a message between different topics


## Graph

- [x] Add publisher/subscribers as nodes
- [x] Rankdir LR
- [x] Specify quantiles on commandline
- [x] Color edges inside Node based on latency
  - [x] Green is 1x min val, Red is MAX(max val; 5x min val)
  - [x] Use commandline parameter to set min. multiplier


## Export

- [ ] To CSV
- [ ] To libreoffice calc

## Trace format support

- [ ] LTTng live
- [ ] Multi host traces
