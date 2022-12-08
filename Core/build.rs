fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/**");

    #[cfg(target_os = "macos")]
    {
        let mut build = cc::Build::new();
        build.include("src/ifname").cpp(false).file("src/ifname/ifname.c");
        build.compile("ifname");
    }
}
