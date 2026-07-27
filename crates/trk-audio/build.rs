fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/c_dsp/vendor/trk_c_gain.c");
    println!("cargo:rerun-if-changed=src/c_dsp/vendor/trk_c_gain.h");

    if std::env::var_os("CARGO_FEATURE_C_DSP_BOUNDARY").is_none() {
        return;
    }

    cc::Build::new()
        .file("src/c_dsp/vendor/trk_c_gain.c")
        .include("src/c_dsp/vendor")
        .warnings(true)
        .compile("trk_c_gain");
}
