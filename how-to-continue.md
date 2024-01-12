# how to continue

## done last time

```sh
cd denate
cargo run # creates a new LayoutThread, and terminates
```

## problems with that

- generating image keys is a big hack, search for TODO denate or so
- sometimes there is a panic at the end, we don't clean up properly (who cares) 

## do next time

- connect denate/src/lib.rs to Deno FFI
- set up IPC channels correctly
- pass in stuff to the IPC channels so that we can do some layouting :heart_eyes:
- set up JS<->Rust comm and pass DOM info and display lists back and forth
- model a simply display list in JS and start writing a compositor

## other ideas

- can we drop the script crate by implementing our own GC for DOM nodes?
