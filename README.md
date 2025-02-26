# Rspack Mimalloc Memory Test

This is a repo for reproducing the memory consumption issue on the macOS.

## Prepare

To prepare for the demo, please install rust-toolchains on macOS and switch to the nightly version.

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

In this demo, an rspack compiler was created to compile JavaScript in `10000` directly. For each `build` and `rebuild`, Rspack would trigger [tokio-rs](https://github.com/tokio-rs/tokio) to spawn(if not already spawned) a few green threads to drive asynchronous tasks. Then, Rspack would trigger a series of JavaScript module transformations, then optimizations. Finally, assets generated in each `build` or `rebuild` will be emitted to the dist file.

**Actual behavior**:

Using macOS Activity Monitor, you would see RSS increases indefinitely as it rebuilds. At first, I thought it was a memory leak in Rspack. However, it turns out that this does not happen when using the macOS system allocator or on Linux Ubuntu-22.04.

We do have some statically and globally initialized objects like `Ustr`, but this does not consume much of the memory.

**Expected behavior**:

RSS memory goes down as soon as rebuild starts. This is where we clean the previous compilation and cache.

**Switch Mimalloc version:**

Go to `mimalloc_rust/libmimalloc-sys/c_src/mimalloc` and checkout to a different ref. To recompile, you need to remove the `target` directory or use `cargo clean` and trigger `cargo build --release` again.

## Things I also tried

1. I tried to enable a few environment variables recommended in `README`, MIMALLOC_PURGE_DELAY=0, and use `mi_collect(true)` on each rebuild. However, this does not help either.

2. I also tried to use mimalloc v3 (branch `dev-3`, I believe) and the problem still exists.

3. I also generated a `mimalloc.log`, which was built with mimalloc debug enabled, to see if this might be helpful.

4. Tokio-wise, I tried to use `RUSTFLAGS="--cfg tokio_unstable" cargo run --release` and uncomment `dbg!` macro down below the `src/main.rs` file to see the remaining tasks and threads, and the results indicated that they had been freed.

## Behavior on Ubuntu-22.04

Memory does not goes up indefinitely. It's around 550 MB ~ 600 MB for v2.1.7 and <550MB for branch dev-3.

I didn't got the chance to test it on Windows.

## Environment

This is the environment that I was on:

MacBook Pro, Apple M2 MAX, 64 GB, macOS Sequoia (version 15.0, 24A335)
