# Rspack Mimalloc Memory Test

This is a repo for reproducing the memory consumption issue especially on the macOS.

## Prepare

To prepare for the demo, pleaser install rust-toolchains on macOS and switch to the nightly version.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh  
```

Then, switch toolchains to the nightly version to work with Rspack:

```bash
rustup toolchain install nightly
```

Then, clone this repo:

```bash
git clone https://github.com/h-a-n-a/rspack-allocation-test.git
```

Checkout git submodules recursively in the repo. This is for building mimalloc on the fly.

```bash
git submodule update --init --recursive
```

## Compile and run

```bash
cargo run --release

# or you can compile and run the executable in separate commands

cargo build --release
CARGO_MANIFEST_DIR=$(pwd) ./target/release/mimalloc-test
```

This executes the code in `src/main.rs`.

In this demo, an rspack compiler was created to compile JavaScript in `10000` directly. For each `build` and `rebuild`, Rspack would trigger [tokio-rs](https://github.com/tokio-rs/tokio) to spawn a few green threads to drive asynchronous tasks. Then, Rpsack would trigger a series of JavaScript module transformations, then optimizations. Finally, assets generated in each `build` or `rebuild` will be emitted to the dist file.

Using macOS Activity Monitor, you would see RSS increases indefinitely as it rebuilds. This does not happen when using the macOS system allocator or on Linux Ubuntu-22.04.

## Environment

This is the environment that I was on:

MacBook Pro, Apple M2 MAX, 64 GB, macOS Sequoia (version 15.0, 24A335)
