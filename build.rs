<<<<<<< HEAD
fn main() {
    /*compile_error!("This branch is no longer updated, please use the current branch or the 0.6.x branch.
Please check this document for more information: https://github.com/serenity-rs/serenity/blob/current/CONTRIBUTING.md#pull-requests")*/
}
=======
#[cfg(all(any(feature = "http", feature = "gateway"),
    not(any(feature = "rustls_backend", feature = "native_tls_backend"))))]
compile_error!("You have the `http` or `gateway` feature enabled, \
    either the `rustls_backend` or `native_tls_backend` feature must be
    selected to let Serenity use `http` or `gateway`.\n\
    - `rustls_backend` uses Rustls, a pure Rust TLS-implemenation.\n\
    - `native_tls_backend` uses SChannel on Windows, Secure Transport on macOS, \
    and OpenSSL on other platforms.\n\
    If you are unsure, go with `rustls_backend`.");

fn main() {}
>>>>>>> 595afbc24a4b742ac8e7ce6d179569d746e434e4
