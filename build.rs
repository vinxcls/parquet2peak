fn main() {
    println!("cargo:rustc-link-search=native=C:\\Peak"); // Library path
    println!("cargo:rustc-link-lib=static=PCANBasic"); // Link to static library
}

