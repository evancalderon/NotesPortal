# Notes Portal
Ensure that both [node](https://nodejs.org/en) and [cargo](https://www.rust-lang.org/) are installed and run the command `cargo build` to build the project in debug mode.

To build the project in release mode, use the command `cargo build --release`

# Compiling for Linux
You will most likely want to use [cross](https://github.com/cross-rs/cross) to compile for Linux.
This repository has been set up to be able to make use of Cross for building for a linux host.
Using cross means that you should only need Docker and Cargo.

To install Cross, run
```
cargo install cross --git https://github.com/cross-rs/cross
```

To use Cross to compile in release mode for a Linux host, run
```
cross build --release
```