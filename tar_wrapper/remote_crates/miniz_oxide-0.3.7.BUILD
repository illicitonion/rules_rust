load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_library",
)

rust_library(
    name = "miniz_oxide",
    crate_type = "lib",
    deps = [
        "@adler32//:adler32",
    ],
    srcs = glob(["**/*.rs"]),
    crate_root = "src/lib.rs",
    edition = "2018",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.3.7",
    visibility = ["//visibility:public"],
    untar = "@io_bazel_rules_rust//tar_wrapper:false",
)
