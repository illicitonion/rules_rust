load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_library",
)

rust_library(
    name = "flate2",
    crate_type = "lib",
    deps = [
        "@cfg_if//:cfg_if",
        "@crc32fast//:crc32fast",
        "@libc//:libc",
        "@miniz_oxide//:miniz_oxide",
    ],
    srcs = glob(["**/*.rs"]),
    crate_root = "src/lib.rs",
    edition = "2018",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "1.0.14",
    crate_features = [
        "miniz_oxide",
    ],
    visibility = ["//visibility:public"],
    untar = "@io_bazel_rules_rust//tar_wrapper:false",
)
