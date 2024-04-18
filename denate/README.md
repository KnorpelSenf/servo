# Layouting to DisplayList

This directory leverages a part of servo in order to run the layouting up to the point where a webrender display list is obtained.
Instead of sending it to the rendering pipeline, we print it to stdout.
This is useful when embedding the servo layouting only.

## Running It

In this directory, run

```sh
cargo run
```

to try fire a reflow event which segfaults the layouting.

Note that the code will not compile right now, as we are running into the following problem.


## Next Up

We want to remove all pointers to C++ memory.
This means that we effectively have to get rid of all the smart pointers in components/script/dom/bindings/root.rs.

When we replace a pointer to a DOM node by an `Rc<Node>`, we run into the problem of having to cast `T&` to `Rc<T>` hundreds of times.

When we replace a pointer to a DOM node by borrowing the node via `&Node`, we run into the problem of having to introduce lifetimes in hundreds of places.

This boils down to the problem that there is no memory management of the DOM nodes on the Rust side yet.
Basically, we must replace the GC in SpiderMonkey by something that Rust does.
It currently is not clear how this can be done, but most likely, using reference counting seems like the more plausible solution than lifetimes (is there a third option?) because the only real lifetime that makes sense would be the lifetime of the document.
This is obviously a too broad scope, meaning that all DOM nodes live as long as the page itself.

What can we do?

## IPC Channels

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
