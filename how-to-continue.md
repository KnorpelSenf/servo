# how to continue

## done last time

```sh
cd denate
cargo run # creates a new LayoutThread, sends a reflow message, waits for the reflow, and segfaults
```

## problems with that

- generating image keys is a big hack, search for TODO denate or so
- sometimes there is a panic at the end, we don't clean up properly (who cares)

## do next time

- we still need to find out how to construct DOM nodes without the script crate because it ships with SpiderMonkey
  - perhaps we can drop the script crate by implementing our own GC for DOM nodes?
- pass in those nodes rather than mut ref cell pointers to C++ memory managed by the SM GC

- set up JS<->Rust comm and pass DOM info and display lists back and forth
- model a simply display list in JS and start writing a compositor
