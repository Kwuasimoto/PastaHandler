fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.compile().expect("embedding the exe icon failed");
    }
    println!("cargo:rerun-if-changed=assets/icon.ico");
}
