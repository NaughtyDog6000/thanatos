# Quickstart
## Installation

To install Rust on your Machine (Rust is the language the engine is built in) go to [the rust website](https://www.rust-lang.org/learn/get-started) and install rustup which manages and updates rust along with cargo.  

To compile the project, you will need to have the [Vulkan SDK](https://vulkan.lunarg.com/sdk) installed. **Only the Default Installation Required**

_Optional_ 
- Cargo-Watch `cargo install cargo-watch`automatically re-builds the project when you make changes.

## Building and running

To build the project navigate to the root of the project (C:/.../thanatos) and run `cargo build` which builds the project in debug mode (this will install and compile all the dependancies and store them, along with the final result in the `target/` folder.

If the project is built running `cargo run` will run the previously built debug build, if you have no previous build it will first compile it like with `cargo build` then run the compiled application.

