# Layouting to DisplayList

This directory leverages a part of servo in order to run the layouting up to the point where a webrender display list is obtained.
Instead of sending it to the rendering pipeline, we print it to stdout.
This is useful when embedding the servo layouting only.

## Running It

In this directory, run

```sh
cargo run
```

to construct and discard a layout thread.

## Next Up

We need to send message to the constructed layout thread.
Our current understanding of the IPC channels is that they do the following.

- layout_pair:
  - add/rm stylesheets
  - reflowing
  - mem reporting
  - exiting
  - creating a layout thread
  - setting scroll states
  - navigation
- namespace_request_channel
  - manage pipeline identifiers
  - manage pipeline namespaces
- time_profiler_chan, mem_profiler_chan
  - obtain timing and memory profiling reports
- webrender_image_channel
  - add images
  - gen image keys
  - communicate with image cache
- script_chan
  - navigation
  - resizing
  - full screen
  - exiting
  - viewports
  - scroll states
  - titles
  - visibility
  - iframes
  - windowing
  - pipeline identifiers
  - history
  - focus
  - animation ticks
  - fonts
  - storages
  - reloading
  - web gpu
- pipeline_port
  - exit
  - set scroll state
  - fonts
  - time stamps
- constellation_chan
  - hang monitor
- constellation_chan_2, layout_chan
  - iframe sizes
  - paint metrics
- control_chan
  - manage sampling profiler
- webrender_chan
  - manage render pipelines
  - perform scrolls
  - send display lists
  - hit testing
  - gen image keys
  - upd image resources
- embedder_chan
  - status messages
  - page title updates
  - resizing
  - prompting
  - navigation
  - web views
  - clipboards
  - cursors
  - favicons
  - full screen
  - page loading state
  - bluetooth
  - file selection dialogs
  - permissions
  - input methods
  - shutdown
  - profiling
  - devtools
  - input events
- compositor_chan
  - shutdown
  - animations
  - frame trees
  - recomposite
  - touch events
  - PNG creation
  - scroll frames
  - WGL GLContext
  - paint metrics
  - page loading states
  - input events
  - windowing
  - forwarding from constellation
