// Cargo does not track files pulled in via include_str!/global_asm! and
// does not track the linker script at all. Without these lines, editing
// boot.s or linker.ld silently produces a STALE binary (we learned this
// the hard way). Do not remove.
fn main() {
    println!("cargo:rerun-if-changed=src/arch/aarch64/boot.s");
    println!("cargo:rerun-if-changed=linker.ld");
}
