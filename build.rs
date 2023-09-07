use npm_rs::*;

fn main() {
    println!("cargo:rerun-if-changed=frontend");
    println!("cargo:rerun-if-changed=web");

    NpmEnv::default()
        .set_path("./frontend")
        .with_node_env(&NodeEnv::from_cargo_profile().unwrap_or_default())
        .init_env()
        .install(None)
        .run("build")
        .exec()
        .unwrap();
}
