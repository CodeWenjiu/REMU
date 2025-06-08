# How to build
- install cargo
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

- install llvm 18
```bash
wget -qO- https://apt.llvm.org/llvm.sh | sudo bash -s -- 18
```

- install python kconfiglib
```bash
pip install kconfiglib
```

- run the following commands
```bash
cargo clean # optional
cargo run # will enter config menu if config file not exist
```

- enter config menu
```bash
make menuconfig-static # control marco
make menuconfig-dynamic # control dynamic config
```

# Key map
- `Ctrl + D`: Exit
- `Ctrl + C`: Cancel input
- `Ctrl + Backspace`: Delete the word before the cursor
- `Ctrl + ←`: Move the cursor to the word before
- `Ctrl + →`: Move the cursor to the word after
- `Ctrl + A`: Move the cursor to the beginning of the line
- `Ctrl + E`: Move the cursor to the end of the line
- `↑`: Move the cursor to the previous command
- `↓`: Move the cursor to the next command
- `←`: Move the cursor to the left
- `→`: Move the cursor to the right or complete the command by history
- `Tab`: Complete the command; twice to list all possible completions
- `Enter`: Execute the command