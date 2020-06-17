load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_library",
)

rust_library(
    name = "xattr",
    crate_type = "lib",
    deps = [
        "@libc//:libc",
    ],
    srcs = glob(["**/*.rs"]),
    crate_root = "src/lib.rs",
    edition = "2015",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.2.2",
    crate_features = [
        "default",
        "unsupported",
    ],
    visibility = ["//visibility:public"],
    untar = "@io_bazel_rules_rust//tar_wrapper:false",
)
