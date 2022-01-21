use std::{
    env,
    fs,
    path::Path,
};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let libjvm_path = env::var_os("JAVA_JVM_LIBRARY").unwrap();
    let libjvm_dir_path = Path::new(&libjvm_path).parent().unwrap();
    println!("cargo:rustc-link-arg=-Wl,-rpath={}", libjvm_dir_path.display());
}
