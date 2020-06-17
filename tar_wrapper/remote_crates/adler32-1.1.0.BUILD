load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_library",
)

rust_library(
    name = "adler32",
    crate_type = "lib",
    srcs = glob(["**/*.rs"]),
    crate_root = "src/lib.rs",
    edition = "2015",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "1.1.0",
    crate_features = [
        "default",
        "std",
    ],
    visibility = ["//visibility:public"],
    untar = "@io_bazel_rules_rust//tar_wrapper:false",
)
