load("@rules_rust_external//:repositories_bin.bzl", "resolver_bin_deps")

def rust_external_deps():
    resolver_bin_deps()
