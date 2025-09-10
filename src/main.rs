#[cfg(feature = "binary")]
mod main_impl;

#[cfg(feature = "binary")]
mod indicatif_log_bridge;

fn main() {
    #[cfg(not(feature = "binary"))]
    eprintln!("This package is not built with the binary feature enabled. Please enable the binary feature to use the command line interface.");

    #[cfg(feature = "binary")]
    main_impl::main();
}
