# How to build
- install cargo
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

- install llvm 18
```bash
wget -qO- https://apt.llvm.org/llvm.sh | sudo bash -s -- 18
```

- install kconfiglib toml
```bash
pip install kconfiglib toml
```

- run the following commands
```bash
cargo clean # optional
cargo run
```
