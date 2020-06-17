load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_library",
)

rust_library(
    name = "tar",
    crate_type = "lib",
    deps = [
        "@filetime//:filetime",
        "@libc//:libc",
        "@xattr//:xattr",
    ],
    srcs = glob(["**/*.rs"]),
    crate_root = "src/lib.rs",
    edition = "2018",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.4.29",
    crate_features = [
        "default",
        "xattr",
    ],
    visibility = ["//visibility:public"],
    untar = "@io_bazel_rules_rust//tar_wrapper:false",
)
