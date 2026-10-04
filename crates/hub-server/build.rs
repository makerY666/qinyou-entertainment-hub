use std::{env, fs, path::Path};
fn main() {
    let root = Path::new("../../web/dist");
    println!("cargo:rerun-if-changed=../../web/dist");
    fn visit(root: &Path, dir: &Path, entries: &mut Vec<String>) {
        if let Ok(files) = fs::read_dir(dir) {
            for file in files.flatten() {
                let p = file.path();
                if p.is_dir() {
                    visit(root, &p, entries);
                } else {
                    let relative = p
                        .strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/");
                    let abs = fs::canonicalize(&p).unwrap();
                    entries.push(format!(
                        "({:?}, include_bytes!({:?}) as &'static [u8]),",
                        relative,
                        abs.to_string_lossy()
                    ));
                }
            }
        }
    }
    let mut entries = vec![];
    visit(root, root, &mut entries);
    entries.sort();
    if env::var("PROFILE").as_deref() == Ok("release") && entries.is_empty() {
        panic!("Build offline UI first: npm --prefix web run build");
    }
    fs::write(
        Path::new(&env::var("OUT_DIR").unwrap()).join("assets.rs"),
        format!(
            "static ASSETS: &[(&str, &[u8])] = &[{}];",
            entries.join("\n")
        ),
    )
    .unwrap();
}
