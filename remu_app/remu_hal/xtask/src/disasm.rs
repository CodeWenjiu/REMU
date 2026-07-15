pub fn infer_isa_from_elf_path(elf_path: &str) -> String {
    const SUFFIX: &str = "-unknown-none-elf";
    elf_path
        .split('/')
        .find(|s| s.ends_with(SUFFIX))
        .map(|s| s.trim_end_matches(SUFFIX).to_string())
        .unwrap_or_else(|| "riscv32i".into())
}
