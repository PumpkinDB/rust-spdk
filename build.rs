use std::process::{Command, exit};
use std::path::Path;
use std::io::{stderr, Write};

fn exec(command_name: &str, mut cmd: Command) {
    match cmd.output() {
        Ok(out) => if !out.status.success() {
            let _ = writeln!(&mut stderr(), "{} failed:\n {}",
                             command_name, String::from_utf8(out.stderr).unwrap());
            exit(1);
        },
        Err(e) => {
            let _ = writeln!(&mut stderr(), "{} exec failed: {:?}", command_name, e);
            exit(1);
        }
    }
}

fn main() {
    let mut make_dpdk = Command::new("make");
    make_dpdk.current_dir(Path::new("spdk/dpdk"))
        .arg("install").arg("T=x86_64-native-linuxapp-gcc").arg("EXTRA_CFLAGS=-fPIC");
    exec("dpdk config", make_dpdk);

    let mut config_spdk = Command::new("./configure");
    config_spdk.current_dir(Path::new("spdk"))
        .arg("--with-dpdk=dpdk/x86_64-native-linuxapp-gcc");
    exec("spdk config", config_spdk);

    let mut make_spdk = Command::new("make");
    make_spdk.current_dir(Path::new("spdk"));
    exec("spdk make", make_spdk);

    println!("cargo:rustc-link-lib=static=spdk_env_dpdk");
    println!("cargo:rustc-link-lib=static=spdk_log");
    println!("cargo:rustc-link-lib=static=spdk_util");
    println!("cargo:rustc-link-lib=static=spdk_nvme");
    println!("cargo:rustc-link-search=spdk/build/lib");
    println!("cargo:rustc-link-lib=spdk/dpdk");
    println!("cargo:rustc-link-search=spdk/dpdk/x86_64-native-linuxapp-gcc/lib");
}
