**JDS** is a compiled, statically-structured scripting language designed for x86_64 Linux systems. It compiles directly into NASM assembly, offering high performance, standalone executables, and low-level control over memory execution.

---

## Features

* Direct to ASM: Compiles straight to x86_64 NASM assembly.

* Clean Syntax: Intuitive and easy-to-read syntax (let, if, while).

* I/O Operations: Simple console printing and user input (print, input).

* Dynamic Execution: Compile modules as raw .bin files and load them directly into memory at runtime using the exec statement(for now pretty useless).

---

**Installation**

To build the compiler and compile JDS scripts, you need Rust, NASM, and a linker (LD) installed on your system.

# Ubuntu / Debian

```bash
sudo apt update
sudo apt install nasm binutils
```

Build the Compiler
Compile the Rust source code to get the jds compiler executable:

```bash
cd compiler
cargo build --release
```
