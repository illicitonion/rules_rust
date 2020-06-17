load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_library",
)

load(
    "@io_bazel_rules_rust//cargo:cargo_build_script.bzl",
    "cargo_build_script",
)

rust_library(
    name = "crc32fast",
    crate_type = "lib",
    deps = [
        #":crc32fast_build_script",
        "@cfg_if//:cfg_if",
    ],
    srcs = glob(["**/*.rs"]),
    crate_root = "src/lib.rs",
    edition = "2015",
    rustc_flags = [
        "--cap-lints=allow",
        "--cfg=rc32fast_stdarchx86",
    ],
    version = "1.2.0",
    crate_features = [
        "default",
        "std",
    ],
    visibility = ["//visibility:public"],
    untar = "@io_bazel_rules_rust//tar_wrapper:false",
)
