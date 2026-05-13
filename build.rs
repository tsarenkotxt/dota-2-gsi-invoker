fn main() {
    println!("cargo:rerun-if-changed=assets/app.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    winresource::WindowsResource::new()
        .set_icon("assets/app.ico")
        .set("FileDescription", "dota_2_gsi_invoker")
        .set("ProductName", "dota_2_gsi_invoker")
        .compile()
        .expect("failed to embed Windows resources");
}
