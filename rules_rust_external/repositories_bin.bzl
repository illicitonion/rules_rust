load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_file")

# These could perhaps be populated with pre-built versions as part of a release pipeline. Details to be discussed :)

def resolver_bin_deps():
    http_file(
        name = "rules_rust_external_resolver_linux",
        urls = [
            "file:///dev/null",
        ],
        executable = True,
    )

    http_file(
        name = "rules_rust_external_resolver_darwin",
        urls = [
            "file:///dev/null",
        ],
        executable = True,
    )
