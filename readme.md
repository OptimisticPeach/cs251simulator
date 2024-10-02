= CS251 9-instruction ARM Simulator

== Compilation

To compile, have a recent version of Rust installed (this was developed
using Rust 1.80.0).

Check your version of Rust by running:
```bash
rustc -V
```

To compile, run:
```bash
cargo build --release
```
(You can add other parameters, such as a different target, etc. Those params
would be covered in Cargo's documentation)

The executable (assuming you ran the above command) will be found at:
```
./target/release/cs251simulator[.exe]
```

== Usage

There are two main commands you can run when launching it from the command-line:
- `load` allows you to load a file saved with this program into the UI.
  ```bash
  cs251simulator.exe load --file ./fib.arm
  ```
- `run` allows you to run a file for a specified number of iterations and
  save the result to an output file.
  ```bash
  $ cs251simulator.exe run --file ./fib.arm --max-iters 1000 --out output_state.arm
  Successfully exited after 400 iterations.
  ```

Specifying no arguments will bring up the UI with an empty state.

When in the UI, key bindings are listed in the bottom row of the screen.

--------

Please let me know (via an issue or an email) if you find an issue with this.
