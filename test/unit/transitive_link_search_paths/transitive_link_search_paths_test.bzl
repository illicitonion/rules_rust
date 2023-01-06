"""Unittests for rust rules."""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")
load("//cargo:defs.bzl", "cargo_build_script")
load("//rust:defs.bzl", "rust_binary", "rust_common", "rust_library", "rust_proc_macro")
load("//test/unit:common.bzl", "assert_action_mnemonic", "assert_argv_contains", "assert_consecutive_flags", "assert_not_consecutive_flags", "get_action_with_mnemonic")

def _transitive_link_flags_provider_test_impl(ctx):
    env = analysistest.begin(ctx)
    tut = analysistest.target_under_test(env)
    link_flag_files = tut[rust_common.dep_info].link_flag_files.to_list()
    link_search_path_basenames = [f.basename for f in link_flag_files]

    # Checks that this contains the dep build script, but not the build script
    # of the dep of the proc_macro.
    asserts.equals(env, link_search_path_basenames, ["dep_build_script.linkflags", "dep_build_script.linksearchpaths"])

    return analysistest.end(env)

def _binary_flags_test_impl(ctx):
    env = analysistest.begin(ctx)
    tut = analysistest.target_under_test(env)

    rustc_action = get_action_with_mnemonic(env, tut, "Rustc")
    assert_consecutive_flags(env, rustc_action, "--arg-file", second_suffix = "dep_build_script.linksearchpaths")
    assert_consecutive_flags(env, rustc_action, "--arg-file", second_suffix = "dep_build_script.linkflags")

    return analysistest.end(env)

def _library_flags_test_impl(ctx):
    env = analysistest.begin(ctx)
    tut = analysistest.target_under_test(env)

    rustc_action = get_action_with_mnemonic(env, tut, "Rustc")
    assert_not_consecutive_flags(env, rustc_action, "--arg-file", second_suffix = "dep_build_script.linksearchpaths")
    assert_not_consecutive_flags(env, rustc_action, "--arg-file", second_suffix = "dep_build_script.linkflags")

    return analysistest.end(env)

transitive_link_flags_provider_test = analysistest.make(_transitive_link_flags_provider_test_impl)

binary_flags_test = analysistest.make(_binary_flags_test_impl)

library_flags_test = analysistest.make(_library_flags_test_impl)

def transitive_link_search_paths_test_suite(name):
    """Entry-point macro called from the BUILD file.

    Args:
        name: Name of the macro.
    """

    cargo_build_script(
        name = "proc_macro_build_script",
        srcs = ["proc_macro_build.rs"],
        edition = "2018",
    )

    rust_proc_macro(
        name = "proc_macro",
        srcs = ["proc_macro.rs"],
        edition = "2018",
        deps = [":proc_macro_build_script"],
    )

    cargo_build_script(
        name = "dep_build_script",
        srcs = ["dep_build.rs"],
        edition = "2018",
    )

    rust_library(
        name = "dep",
        srcs = ["dep.rs"],
        edition = "2018",
        proc_macro_deps = [":proc_macro"],
        deps = [":dep_build_script"],
    )

    rust_binary(
        name = "bin",
        srcs = ["main.rs"],
        edition = "2018",
        deps = [":dep"],
    )

    transitive_link_flags_provider_test(
        name = "transitive_link_flags_provider_test",
        target_under_test = ":dep",
    )

    binary_flags_test(
        name = "binary_flags_test",
        target_under_test = ":bin",
    )

    library_flags_test(
        name = "library_flags_test",
        target_under_test = ":dep",
    )

    native.test_suite(
        name = name,
        tests = [
            ":binary_flags_test",
            ":library_flags_test",
            ":transitive_link_flags_provider_test",
        ],
    )
