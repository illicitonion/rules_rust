load("//rust:rust.bzl", "rust_binary", "rust_library", "rust_test")

rust_library(
    name = "tar_wrapper",
    srcs = ["src/lib.rs"],
    deps = [
        "@flate2//:flate2",
        "@tar//:tar",
    ],
    edition = "2018",
    visibility = ["//visibility:public"],
    untar = ":false",
)

rust_binary(
    name = "untar",
    srcs = ["src/untar.rs"],
    deps = [":tar_wrapper"],
    edition = "2018",
    untar = ":false",
    visibility = ["//visibility:public"],
)

rust_test(
    name = "test",
    crate = ":tar_wrapper",
    data = ["//test:example.tar.gz"],
)

# Hacky script we use as the value of the `untar` attribute for rust_libraries used to build untar itself, which don't
# actually need untar, to avoid dependency cycles.
exports_files(["false"])
