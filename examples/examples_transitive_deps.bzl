"""Define transitive dependencies for `rules_rust` examples

There are some transitive dependencies of the dependencies of the examples' 
dependencies. This file contains the required macros to pull these dependencies
"""

load("@bazel_tools//tools/build_defs/repo:utils.bzl", "maybe")
load("@rules_proto//proto:repositories.bzl", "rules_proto_dependencies", "rules_proto_toolchains")

load("@rules_rust_external//:repositories_bin.bzl", "resolver_bin_deps")

load("@rules_rust_external//:workspace.bzl", "crate", "crate_universe")

# buildifier: disable=unnamed-macro
def transitive_deps(is_top_level = False):
    """Define transitive dependencies for `rules_rust` examples

    Args:
        is_top_level (bool, optional): Indicates wheather or not this is being called
            from the root WORKSPACE file of `rules_rust`. Defaults to False.
    """

    rules_proto_dependencies()

    rules_proto_toolchains()

    # Needed by the hello_uses_cargo_manifest_dir example.
    if is_top_level:
        maybe(
            native.local_repository,
            name = "rules_rust_example_cargo_manifest_dir",
            path = "examples/cargo_manifest_dir/external_crate",
        )
    else:
        maybe(
            native.local_repository,
            name = "rules_rust_example_cargo_manifest_dir",
            path = "cargo_manifest_dir/external_crate",
        )

    resolver_bin_deps()

    crate_universe(
        name = "complex_sys_deps",
        cargo_toml_files = ["@examples//complex_sys:Cargo.toml"],
        supported_targets = [
            "x86_64-apple-darwin",
            "x86_64-unknown-linux-gnu",
        ],
        overrides = {
            "libgit2-sys": crate.override(
                extra_build_script_env_vars = {
                    "OPENSSL_DIR": "../openssl/openssl",
                },
                extra_bazel_deps = {
                    "cfg(all())": ["@openssl//:openssl"],
                },
                extra_build_script_bazel_data_deps = {
                    "cfg(all())": ["@openssl//:openssl"],
                },
                features_to_remove = ["vendored"],
            ),
            "libssh2-sys": crate.override(
                extra_build_script_env_vars = {
                    "OPENSSL_DIR": "../openssl/openssl",
                },
                extra_bazel_deps = {
                    "cfg(all())": ["@openssl//:openssl"],
                },
                extra_build_script_bazel_data_deps = {
                    "cfg(all())": ["@openssl//:openssl"],
                },
                features_to_remove = ["vendored-openssl"],
            ),
            "openssl-sys": crate.override(
                extra_build_script_env_vars = {
                    "OPENSSL_DIR": "../openssl/openssl",
                    "OPENSSL_STATIC": "1",
                },
                extra_bazel_deps = {
                    "cfg(all())": ["@openssl//:openssl"],
                },
                extra_build_script_bazel_data_deps = {
                    "cfg(all())": ["@openssl//:openssl"],
                },
                features_to_remove = ["vendored"],
            ),
        },
    )
