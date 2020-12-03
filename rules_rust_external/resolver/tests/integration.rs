use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::create_dir_all;
use std::path::PathBuf;

use assert_cmd::assert::Assert;
use assert_cmd::Command;
use maplit::btreemap;
use predicates::boolean::PredicateBooleanExt;
use predicates::ord::eq;
use semver::VersionReq;

use resolver::config::{Config, Override, Package};
use resolver::NamedTempFile;

#[test]
fn basic() {
    let cargo_toml_file = NamedTempFile::with_str_content(
        "Cargo.toml",
        r#"[package]
name = "basic"
version = "0.1.0"
edition = "2018"

[dependencies]
lazy_static = "=1.4.0"
"#,
    )
    .expect("Error making temporary file");

    let config = Config {
        cargo_toml_files: btreemap! { String::from("//some:Cargo.toml") => cargo_toml_file.path().to_path_buf() },
        overrides: Default::default(),
        repository: "https://crates-io.s3-us-west-1.amazonaws.com/crates/crates/{name}/{name}-{version}.crate".to_owned(),
        target_triples: vec!["x86_64-apple-darwin".to_owned()],
        packages: vec![],
        cargo: PathBuf::from(env!("CARGO")),
    };

    let want_output = r##"
load("@bazel_tools//tools/build_defs/repo:git.bzl", "new_git_repository")

load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

def pinned_rust_install():
    http_archive(
        name = "__lazy_static__1_4_0",
        # TODO: Allow configuring where rust_library comes from
        build_file_content = """# buildifier: disable=load
load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

# buildifier: disable=load
load("@bazel_skylib//lib:selects.bzl", "selects")

package(default_visibility = [
    "//visibility:public",
])

licenses([
    "notice",  # MIT from expression "MIT OR Apache-2.0"
])

# Generated targets

# buildifier: leave-alone
rust_library(
    name = "lazy_static",
    crate_type = "lib",
    deps = [
    ],
    srcs = glob(["**/*.rs"]),
    crate_root = "src/lib.rs",
    edition = "2015",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    data = glob(["**"], exclude=[
        # These can be manually added with overrides if needed.

        # If you run `cargo build` in this dir, the target dir can get very big very quick.
        "target/**",

        # These are not vendored from the crate - we exclude them to avoid busting caches
        # when we change how we generate BUILD files and such.
        "BUILD.bazel",
        "WORKSPACE.bazel",
        "WORKSPACE",
    ]),
    version = "1.4.0",
    tags = [
        "cargo-raze",
        "manual",
    ],
    crate_features = [
    ],
    aliases = select({
        # Default
        "//conditions:default": {
        },
    }),
)
# Unsupported target "no_std" with type "test" omitted
# Unsupported target "test" with type "test" omitted
""",
        strip_prefix = "lazy_static-1.4.0",
        type = "tar.gz",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/crates/lazy_static/lazy_static-1.4.0.crate",
    )


CRATE_TARGET_NAMES = {
    "lazy_static": "@__lazy_static__1_4_0//:lazy_static",
}

def crate(crate_name):
    """Return the name of the target for the given crate.
    """
    target_name = CRATE_TARGET_NAMES.get(crate_name)
    if target_name == None:
        fail("Unknown crate name: {}".format(crate_name))
    return target_name

def all_deps():
    """Return all standard dependencies explicitly listed in the Cargo.toml or packages list."""
    return [
        crate(crate_name) for crate_name in [
            "lazy_static",
        ]
    ]

def all_proc_macro_deps():
    """Return all proc-macro dependencies explicitly listed in the Cargo.toml or packages list."""
    return [
        crate(crate_name) for crate_name in [
        ]
    ]

def crates_from(label):
    mapping = {
        "//some:Cargo.toml": [crate("lazy_static")],
    }

    return mapping[_absolutify(label)]

def proc_macro_crates_from(label):
    mapping = {
        "//some:Cargo.toml": [],
    }

    return mapping[_absolutify(label)]

def _absolutify(label):
    if label.startswith("//") or label.startswith("@"):
        return label
    if label.startswith(":"):
        return "//" + native.package_name() + label
    return "//" + native.package_name() + ":" + label
"##;

    // Ignore header which contains a hash
    test(&config)
        .success()
        .stdout(predicates::str::ends_with(want_output));
}

#[test]
fn intermediate() {
    let cargo_toml_file = NamedTempFile::with_str_content(
        "Cargo.toml",
        r#"[package]
name = "intermediate"
version = "0.1.0"
edition = "2018"

# TODO: support lockfile instead of passing the transitive pinned list as "packages" directly.
[dependencies]
lazy_static = "=1.4.0"
bytes = "=0.5.6"
pin-project-lite = "=0.1.7"
bitflags = "=1.2.1"

# System dependency (libz-sys) and its transitive deps.
libz-sys = "=1.1.2"
cc = "=1.0.62"
libc = "=0.2.80"
pkg-config = "=0.3.19"
vcpkg = "=0.2.10"

# TODO: do not depend on this section to be present rules_rust_external.
[[bin]]
name = "basic"
path = "src/main.rs"
"#,
    )
    .expect("Error making temporary file");

    let mut tokio_extra_rust_env_vars = BTreeMap::<String, String>::new();
    tokio_extra_rust_env_vars.insert("ENV_VAR_1".to_owned(), "value1".to_owned());
    tokio_extra_rust_env_vars.insert("ENV_VAR_2".to_owned(), "value2".to_owned());

    let mut tokio_extra_bazel_deps = BTreeMap::<String, Vec<String>>::new();
    tokio_extra_bazel_deps.insert(
        "x86_64-apple-darwin".to_owned(),
        vec!["@some//:dep".to_owned(), "@other//:dep".to_owned()],
    );
    tokio_extra_bazel_deps.insert("cfg(unix)".to_owned(), vec!["@yetanother//:dep".to_owned()]);

    let mut bitflags_extra_build_script_env_vars = BTreeMap::<String, String>::new();
    bitflags_extra_build_script_env_vars
        .insert("BUILD_SCRIPT_ENV_VAR".to_owned(), "value".to_owned());

    let mut bitflags_extra_bazel_builds_script_deps = BTreeMap::<String, Vec<String>>::new();
    bitflags_extra_bazel_builds_script_deps.insert(
        "x86_64-unknown-linux-gnu".to_owned(),
        vec!["@buildscriptdep//:dep".to_owned()],
    );

    let mut bitflags_extra_bazel_builds_script_data_deps = BTreeMap::<String, Vec<String>>::new();
    bitflags_extra_bazel_builds_script_data_deps.insert(
        "x86_64-unknown-linux-gnu".to_owned(),
        vec!["@buildscriptdep//:somedata".to_owned()],
    );

    let mut lazy_static_extra_bazel_deps = BTreeMap::<String, Vec<String>>::new();
    lazy_static_extra_bazel_deps.insert("cfg(all())".to_owned(), vec!["@such//:dep".to_owned()]);

    let mut lazy_static_extra_bazel_data_deps = BTreeMap::<String, Vec<String>>::new();
    lazy_static_extra_bazel_data_deps.insert(
        "x86_64-unknown-linux-gnu".to_owned(),
        vec!["@such//:somedata".to_owned()],
    );

    let mut overrides = HashMap::new();
    overrides.insert(
        "tokio".into(),
        Override {
            extra_rust_env_vars: tokio_extra_rust_env_vars,
            extra_build_script_env_vars: Default::default(),
            extra_bazel_deps: tokio_extra_bazel_deps,
            extra_build_script_bazel_deps: Default::default(),
            extra_bazel_data_deps: Default::default(),
            extra_build_script_bazel_data_deps: Default::default(),
            features_to_remove: BTreeSet::new(),
        },
    );
    overrides.insert(
        "lazy_static".into(),
        Override {
            extra_rust_env_vars: Default::default(),
            extra_build_script_env_vars: Default::default(),
            extra_bazel_deps: lazy_static_extra_bazel_deps,
            extra_build_script_bazel_deps: Default::default(),
            extra_bazel_data_deps: lazy_static_extra_bazel_data_deps,
            extra_build_script_bazel_data_deps: Default::default(),
            features_to_remove: BTreeSet::new(),
        },
    );
    overrides.insert(
        "bitflags".into(),
        Override {
            extra_rust_env_vars: Default::default(),
            extra_build_script_env_vars: bitflags_extra_build_script_env_vars,
            extra_bazel_deps: Default::default(),
            extra_build_script_bazel_deps: bitflags_extra_bazel_builds_script_deps,
            extra_bazel_data_deps: Default::default(),
            extra_build_script_bazel_data_deps: bitflags_extra_bazel_builds_script_data_deps,
            features_to_remove: BTreeSet::new(),
        },
    );

    let config = Config {
        cargo_toml_files: btreemap! { String::from("//some:Cargo.toml") => cargo_toml_file.path().to_path_buf() },
        overrides,
        repository: "https://crates-io.s3-us-west-1.amazonaws.com/crates/crates/{name}/{name}-{version}.crate".to_owned(),
        target_triples: vec![
            "x86_64-apple-darwin".to_owned(),
            "x86_64-unknown-linux-gnu".to_owned(),
            "x86_64-pc-windows-gnu".to_owned(),
        ],
        packages: vec![Package {
            name: "tokio".to_string(),
            semver: VersionReq::parse("=0.2.22").unwrap(),
            features: vec![],
        }],
        cargo: PathBuf::from(env!("CARGO")),
    };

    let want_output = r##"
load("@bazel_tools//tools/build_defs/repo:git.bzl", "new_git_repository")

load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

def pinned_rust_install():
    http_archive(
        name = "__bitflags__1_2_1",
        # TODO: Allow configuring where rust_library comes from
        build_file_content = """# buildifier: disable=load
load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

# buildifier: disable=load
load("@bazel_skylib//lib:selects.bzl", "selects")

package(default_visibility = [
    "//visibility:public",
])

licenses([
    "notice",  # MIT from expression "MIT OR Apache-2.0"
])

# Generated targets
# buildifier: disable=load-on-top
load(
    "@io_bazel_rules_rust//cargo:cargo_build_script.bzl",
    "cargo_build_script",
)

# buildifier: leave-alone
cargo_build_script(
    name = "bitflags_build_script",
    srcs = glob(["**/*.rs"]),
    crate_root = "build.rs",
    edition = "2015",
    deps = [
    ] + selects.with_or({
        # x86_64-unknown-linux-gnu
        (
            "@io_bazel_rules_rust//rust/platform:x86_64-unknown-linux-gnu",
        ): [
            "@buildscriptdep//:dep",
        ],
        "//conditions:default": [],
    }),
    rustc_flags = [
        "--cap-lints=allow",
    ],
    crate_features = [
      "default",
    ],
    build_script_env = {
        "BUILD_SCRIPT_ENV_VAR": "value",
    },
    data = glob(["**"], exclude=[
        # These can be manually added with overrides if needed.

        # If you run `cargo build` in this dir, the target dir can get very big very quick.
        "target/**",

        # These are not vendored from the crate - we exclude them to avoid busting caches
        # when we change how we generate BUILD files and such.
        "BUILD.bazel",
        "WORKSPACE.bazel",
        "WORKSPACE",
    ]) + selects.with_or({
        # x86_64-unknown-linux-gnu
        (
            "@io_bazel_rules_rust//rust/platform:x86_64-unknown-linux-gnu",
        ): [
            "@buildscriptdep//:somedata",
        ],
        "//conditions:default": [],
    }),
    tags = [
        "cargo-raze",
        "manual",
    ],
    version = "1.2.1",
    visibility = ["//visibility:private"],
)


# buildifier: leave-alone
rust_library(
    name = "bitflags",
    crate_type = "lib",
    deps = [
        ":bitflags_build_script",
    ],
    srcs = glob(["**/*.rs"]),
    crate_root = "src/lib.rs",
    edition = "2015",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    data = glob(["**"], exclude=[
        # These can be manually added with overrides if needed.

        # If you run `cargo build` in this dir, the target dir can get very big very quick.
        "target/**",

        # These are not vendored from the crate - we exclude them to avoid busting caches
        # when we change how we generate BUILD files and such.
        "BUILD.bazel",
        "WORKSPACE.bazel",
        "WORKSPACE",
    ]),
    version = "1.2.1",
    tags = [
        "cargo-raze",
        "manual",
    ],
    crate_features = [
        "default",
    ],
    aliases = select({
        # Default
        "//conditions:default": {
        },
        #  x86_64-unknown-linux-gnu
        "@io_bazel_rules_rust//rust/platform:x86_64-unknown-linux-gnu": {
        },
    }),
)
""",
        strip_prefix = "bitflags-1.2.1",
        type = "tar.gz",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/crates/bitflags/bitflags-1.2.1.crate",
    )

    http_archive(
        name = "__bytes__0_5_6",
        # TODO: Allow configuring where rust_library comes from
        build_file_content = """# buildifier: disable=load
load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

# buildifier: disable=load
load("@bazel_skylib//lib:selects.bzl", "selects")

package(default_visibility = [
    "//visibility:public",
])

licenses([
    "notice",  # MIT from expression "MIT"
])

# Generated targets
# Unsupported target "buf" with type "bench" omitted
# Unsupported target "bytes" with type "bench" omitted
# Unsupported target "bytes_mut" with type "bench" omitted

# buildifier: leave-alone
rust_library(
    name = "bytes",
    crate_type = "lib",
    deps = [
    ],
    srcs = glob(["**/*.rs"]),
    crate_root = "src/lib.rs",
    edition = "2018",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    data = glob(["**"], exclude=[
        # These can be manually added with overrides if needed.

        # If you run `cargo build` in this dir, the target dir can get very big very quick.
        "target/**",

        # These are not vendored from the crate - we exclude them to avoid busting caches
        # when we change how we generate BUILD files and such.
        "BUILD.bazel",
        "WORKSPACE.bazel",
        "WORKSPACE",
    ]),
    version = "0.5.6",
    tags = [
        "cargo-raze",
        "manual",
    ],
    crate_features = [
        "default",
        "std",
    ],
    aliases = select({
        # Default
        "//conditions:default": {
        },
    }),
)
# Unsupported target "test_buf" with type "test" omitted
# Unsupported target "test_buf_mut" with type "test" omitted
# Unsupported target "test_bytes" with type "test" omitted
# Unsupported target "test_bytes_odd_alloc" with type "test" omitted
# Unsupported target "test_bytes_vec_alloc" with type "test" omitted
# Unsupported target "test_chain" with type "test" omitted
# Unsupported target "test_debug" with type "test" omitted
# Unsupported target "test_iter" with type "test" omitted
# Unsupported target "test_reader" with type "test" omitted
# Unsupported target "test_serde" with type "test" omitted
# Unsupported target "test_take" with type "test" omitted
""",
        strip_prefix = "bytes-0.5.6",
        type = "tar.gz",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/crates/bytes/bytes-0.5.6.crate",
    )

    http_archive(
        name = "__cc__1_0_62",
        # TODO: Allow configuring where rust_library comes from
        build_file_content = """# buildifier: disable=load
load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

# buildifier: disable=load
load("@bazel_skylib//lib:selects.bzl", "selects")

package(default_visibility = [
    "//visibility:public",
])

licenses([
    "notice",  # MIT from expression "MIT OR Apache-2.0"
])

# Generated targets

# buildifier: leave-alone
rust_binary(
    # Prefix bin name to disambiguate from (probable) collision with lib name
    # N.B.: The exact form of this is subject to change.
    name = "cargo_bin_gcc_shim",
    deps = [
        # Binaries get an implicit dependency on their crate's lib
        ":cc",
    ],
    srcs = glob(["**/*.rs"]),
    crate_root = "src/bin/gcc-shim.rs",
    edition = "2018",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    data = glob(["**"], exclude=[
        # These can be manually added with overrides if needed.

        # If you run `cargo build` in this dir, the target dir can get very big very quick.
        "target/**",

        # These are not vendored from the crate - we exclude them to avoid busting caches
        # when we change how we generate BUILD files and such.
        "BUILD.bazel",
        "WORKSPACE.bazel",
        "WORKSPACE",
    ]),
    version = "1.0.62",
    tags = [
        "cargo-raze",
        "manual",
    ],
    crate_features = [
    ],
    aliases = select({
        # Default
        "//conditions:default": {
        },
    }),
)

# buildifier: leave-alone
rust_library(
    name = "cc",
    crate_type = "lib",
    deps = [
    ],
    srcs = glob(["**/*.rs"]),
    crate_root = "src/lib.rs",
    edition = "2018",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    data = glob(["**"], exclude=[
        # These can be manually added with overrides if needed.

        # If you run `cargo build` in this dir, the target dir can get very big very quick.
        "target/**",

        # These are not vendored from the crate - we exclude them to avoid busting caches
        # when we change how we generate BUILD files and such.
        "BUILD.bazel",
        "WORKSPACE.bazel",
        "WORKSPACE",
    ]),
    version = "1.0.62",
    tags = [
        "cargo-raze",
        "manual",
    ],
    crate_features = [
    ],
    aliases = select({
        # Default
        "//conditions:default": {
        },
    }),
)
# Unsupported target "cc_env" with type "test" omitted
# Unsupported target "cflags" with type "test" omitted
# Unsupported target "cxxflags" with type "test" omitted
# Unsupported target "test" with type "test" omitted
""",
        strip_prefix = "cc-1.0.62",
        type = "tar.gz",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/crates/cc/cc-1.0.62.crate",
    )

    http_archive(
        name = "__lazy_static__1_4_0",
        # TODO: Allow configuring where rust_library comes from
        build_file_content = """# buildifier: disable=load
load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

# buildifier: disable=load
load("@bazel_skylib//lib:selects.bzl", "selects")

package(default_visibility = [
    "//visibility:public",
])

licenses([
    "notice",  # MIT from expression "MIT OR Apache-2.0"
])

# Generated targets

# buildifier: leave-alone
rust_library(
    name = "lazy_static",
    crate_type = "lib",
    deps = [
    ] + selects.with_or({
        # cfg(all())
        (
            "@io_bazel_rules_rust//rust/platform:x86_64-apple-darwin",
            "@io_bazel_rules_rust//rust/platform:x86_64-unknown-linux-gnu",
        ): [
            "@such//:dep",
        ],
        "//conditions:default": [],
    }),
    srcs = glob(["**/*.rs"]),
    crate_root = "src/lib.rs",
    edition = "2015",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    data = glob(["**"], exclude=[
        # These can be manually added with overrides if needed.

        # If you run `cargo build` in this dir, the target dir can get very big very quick.
        "target/**",

        # These are not vendored from the crate - we exclude them to avoid busting caches
        # when we change how we generate BUILD files and such.
        "BUILD.bazel",
        "WORKSPACE.bazel",
        "WORKSPACE",
    ]) + selects.with_or({
        # x86_64-unknown-linux-gnu
        (
            "@io_bazel_rules_rust//rust/platform:x86_64-unknown-linux-gnu",
        ): [
            "@such//:somedata",
        ],
        "//conditions:default": [],
    }),
    version = "1.4.0",
    tags = [
        "cargo-raze",
        "manual",
    ],
    crate_features = [
    ],
    aliases = select({
        # Default
        "//conditions:default": {
        },
        #  cfg(all())
        "@io_bazel_rules_rust//rust/platform:x86_64-apple-darwin": {
        },
        #  cfg(all()) x86_64-unknown-linux-gnu
        "@io_bazel_rules_rust//rust/platform:x86_64-unknown-linux-gnu": {
        },
    }),
)
# Unsupported target "no_std" with type "test" omitted
# Unsupported target "test" with type "test" omitted
""",
        strip_prefix = "lazy_static-1.4.0",
        type = "tar.gz",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/crates/lazy_static/lazy_static-1.4.0.crate",
    )

    http_archive(
        name = "__libc__0_2_80",
        # TODO: Allow configuring where rust_library comes from
        build_file_content = """# buildifier: disable=load
load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

# buildifier: disable=load
load("@bazel_skylib//lib:selects.bzl", "selects")

package(default_visibility = [
    "//visibility:public",
])

licenses([
    "notice",  # MIT from expression "MIT OR Apache-2.0"
])

# Generated targets
# buildifier: disable=load-on-top
load(
    "@io_bazel_rules_rust//cargo:cargo_build_script.bzl",
    "cargo_build_script",
)

# buildifier: leave-alone
cargo_build_script(
    name = "libc_build_script",
    srcs = glob(["**/*.rs"]),
    crate_root = "build.rs",
    edition = "2015",
    deps = [
    ],
    rustc_flags = [
        "--cap-lints=allow",
    ],
    crate_features = [
      "default",
      "std",
    ],
    build_script_env = {
    },
    data = glob(["**"], exclude=[
        # These can be manually added with overrides if needed.

        # If you run `cargo build` in this dir, the target dir can get very big very quick.
        "target/**",

        # These are not vendored from the crate - we exclude them to avoid busting caches
        # when we change how we generate BUILD files and such.
        "BUILD.bazel",
        "WORKSPACE.bazel",
        "WORKSPACE",
    ]),
    tags = [
        "cargo-raze",
        "manual",
    ],
    version = "0.2.80",
    visibility = ["//visibility:private"],
)


# buildifier: leave-alone
rust_library(
    name = "libc",
    crate_type = "lib",
    deps = [
        ":libc_build_script",
    ],
    srcs = glob(["**/*.rs"]),
    crate_root = "src/lib.rs",
    edition = "2015",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    data = glob(["**"], exclude=[
        # These can be manually added with overrides if needed.

        # If you run `cargo build` in this dir, the target dir can get very big very quick.
        "target/**",

        # These are not vendored from the crate - we exclude them to avoid busting caches
        # when we change how we generate BUILD files and such.
        "BUILD.bazel",
        "WORKSPACE.bazel",
        "WORKSPACE",
    ]),
    version = "0.2.80",
    tags = [
        "cargo-raze",
        "manual",
    ],
    crate_features = [
        "default",
        "std",
    ],
    aliases = select({
        # Default
        "//conditions:default": {
        },
    }),
)
# Unsupported target "const_fn" with type "test" omitted
""",
        strip_prefix = "libc-0.2.80",
        type = "tar.gz",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/crates/libc/libc-0.2.80.crate",
    )

    http_archive(
        name = "__libz_sys__1_1_2",
        # TODO: Allow configuring where rust_library comes from
        build_file_content = """# buildifier: disable=load
load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

# buildifier: disable=load
load("@bazel_skylib//lib:selects.bzl", "selects")

package(default_visibility = [
    "//visibility:public",
])

licenses([
    "notice",  # MIT from expression "MIT OR Apache-2.0"
])

# Generated targets
# buildifier: disable=load-on-top
load(
    "@io_bazel_rules_rust//cargo:cargo_build_script.bzl",
    "cargo_build_script",
)

# buildifier: leave-alone
cargo_build_script(
    name = "libz_sys_build_script",
    srcs = glob(["**/*.rs"]),
    crate_root = "build.rs",
    edition = "2015",
    deps = [
        "@__cc__1_0_62//:cc",
        "@__pkg_config__0_3_19//:pkg_config",
    ],
    rustc_flags = [
        "--cap-lints=allow",
    ],
    crate_features = [
      "default",
      "libc",
      "stock-zlib",
    ],
    build_script_env = {
    },
    links = "z",
    data = glob(["**"], exclude=[
        # These can be manually added with overrides if needed.

        # If you run `cargo build` in this dir, the target dir can get very big very quick.
        "target/**",

        # These are not vendored from the crate - we exclude them to avoid busting caches
        # when we change how we generate BUILD files and such.
        "BUILD.bazel",
        "WORKSPACE.bazel",
        "WORKSPACE",
    ]),
    tags = [
        "cargo-raze",
        "manual",
    ],
    version = "1.1.2",
    visibility = ["//visibility:private"],
)


# buildifier: leave-alone
rust_library(
    name = "libz_sys",
    crate_type = "lib",
    deps = [
        ":libz_sys_build_script",
        "@__libc__0_2_80//:libc",
    ],
    srcs = glob(["**/*.rs"]),
    crate_root = "src/lib.rs",
    edition = "2015",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    data = glob(["**"], exclude=[
        # These can be manually added with overrides if needed.

        # If you run `cargo build` in this dir, the target dir can get very big very quick.
        "target/**",

        # These are not vendored from the crate - we exclude them to avoid busting caches
        # when we change how we generate BUILD files and such.
        "BUILD.bazel",
        "WORKSPACE.bazel",
        "WORKSPACE",
    ]),
    version = "1.1.2",
    tags = [
        "cargo-raze",
        "manual",
    ],
    crate_features = [
        "default",
        "libc",
        "stock-zlib",
    ],
    aliases = select({
        # Default
        "//conditions:default": {
        },
    }),
)
""",
        strip_prefix = "libz-sys-1.1.2",
        type = "tar.gz",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/crates/libz-sys/libz-sys-1.1.2.crate",
    )

    http_archive(
        name = "__pin_project_lite__0_1_7",
        # TODO: Allow configuring where rust_library comes from
        build_file_content = """# buildifier: disable=load
load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

# buildifier: disable=load
load("@bazel_skylib//lib:selects.bzl", "selects")

package(default_visibility = [
    "//visibility:public",
])

licenses([
    "notice",  # Apache-2.0 from expression "Apache-2.0 OR MIT"
])

# Generated targets

# buildifier: leave-alone
rust_library(
    name = "pin_project_lite",
    crate_type = "lib",
    deps = [
    ],
    srcs = glob(["**/*.rs"]),
    crate_root = "src/lib.rs",
    edition = "2018",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    data = glob(["**"], exclude=[
        # These can be manually added with overrides if needed.

        # If you run `cargo build` in this dir, the target dir can get very big very quick.
        "target/**",

        # These are not vendored from the crate - we exclude them to avoid busting caches
        # when we change how we generate BUILD files and such.
        "BUILD.bazel",
        "WORKSPACE.bazel",
        "WORKSPACE",
    ]),
    version = "0.1.7",
    tags = [
        "cargo-raze",
        "manual",
    ],
    crate_features = [
    ],
    aliases = select({
        # Default
        "//conditions:default": {
        },
    }),
)
# Unsupported target "compiletest" with type "test" omitted
# Unsupported target "lint" with type "test" omitted
# Unsupported target "test" with type "test" omitted
""",
        strip_prefix = "pin-project-lite-0.1.7",
        type = "tar.gz",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/crates/pin-project-lite/pin-project-lite-0.1.7.crate",
    )

    http_archive(
        name = "__pkg_config__0_3_19",
        # TODO: Allow configuring where rust_library comes from
        build_file_content = """# buildifier: disable=load
load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

# buildifier: disable=load
load("@bazel_skylib//lib:selects.bzl", "selects")

package(default_visibility = [
    "//visibility:public",
])

licenses([
    "notice",  # MIT from expression "MIT OR Apache-2.0"
])

# Generated targets

# buildifier: leave-alone
rust_library(
    name = "pkg_config",
    crate_type = "lib",
    deps = [
    ],
    srcs = glob(["**/*.rs"]),
    crate_root = "src/lib.rs",
    edition = "2015",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    data = glob(["**"], exclude=[
        # These can be manually added with overrides if needed.

        # If you run `cargo build` in this dir, the target dir can get very big very quick.
        "target/**",

        # These are not vendored from the crate - we exclude them to avoid busting caches
        # when we change how we generate BUILD files and such.
        "BUILD.bazel",
        "WORKSPACE.bazel",
        "WORKSPACE",
    ]),
    version = "0.3.19",
    tags = [
        "cargo-raze",
        "manual",
    ],
    crate_features = [
    ],
    aliases = select({
        # Default
        "//conditions:default": {
        },
    }),
)
# Unsupported target "test" with type "test" omitted
""",
        strip_prefix = "pkg-config-0.3.19",
        type = "tar.gz",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/crates/pkg-config/pkg-config-0.3.19.crate",
    )

    http_archive(
        name = "__tokio__0_2_22",
        # TODO: Allow configuring where rust_library comes from
        build_file_content = """# buildifier: disable=load
load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

# buildifier: disable=load
load("@bazel_skylib//lib:selects.bzl", "selects")

package(default_visibility = [
    "//visibility:public",
])

licenses([
    "notice",  # MIT from expression "MIT"
])

# Generated targets

# buildifier: leave-alone
rust_library(
    name = "tokio",
    crate_type = "lib",
    deps = [
        "@__bytes__0_5_6//:bytes",
        "@__pin_project_lite__0_1_7//:pin_project_lite",
    ] + selects.with_or({
        # cfg(unix)
        (
            "@io_bazel_rules_rust//rust/platform:x86_64-apple-darwin",
            "@io_bazel_rules_rust//rust/platform:x86_64-unknown-linux-gnu",
        ): [
            "@yetanother//:dep",
        ],
        "//conditions:default": [],
    }) + selects.with_or({
        # x86_64-apple-darwin
        (
            "@io_bazel_rules_rust//rust/platform:x86_64-apple-darwin",
        ): [
            "@some//:dep",
            "@other//:dep",
        ],
        "//conditions:default": [],
    }),
    srcs = glob(["**/*.rs"]),
    crate_root = "src/lib.rs",
    edition = "2018",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    rustc_env = {
        "ENV_VAR_1": "value1",
        "ENV_VAR_2": "value2",
    },
    data = glob(["**"], exclude=[
        # These can be manually added with overrides if needed.

        # If you run `cargo build` in this dir, the target dir can get very big very quick.
        "target/**",

        # These are not vendored from the crate - we exclude them to avoid busting caches
        # when we change how we generate BUILD files and such.
        "BUILD.bazel",
        "WORKSPACE.bazel",
        "WORKSPACE",
    ]),
    version = "0.2.22",
    tags = [
        "cargo-raze",
        "manual",
    ],
    crate_features = [
        "default",
    ],
    aliases = select({
        # Default
        "//conditions:default": {
        },
        #  cfg(unix) cfg(unix) x86_64-apple-darwin
        "@io_bazel_rules_rust//rust/platform:x86_64-apple-darwin": {
        },
        #  cfg(unix) cfg(unix)
        "@io_bazel_rules_rust//rust/platform:x86_64-unknown-linux-gnu": {
        },
    }),
)
# Unsupported target "_require_full" with type "test" omitted
# Unsupported target "async_send_sync" with type "test" omitted
# Unsupported target "buffered" with type "test" omitted
# Unsupported target "fs" with type "test" omitted
# Unsupported target "fs_copy" with type "test" omitted
# Unsupported target "fs_dir" with type "test" omitted
# Unsupported target "fs_file" with type "test" omitted
# Unsupported target "fs_file_mocked" with type "test" omitted
# Unsupported target "fs_link" with type "test" omitted
# Unsupported target "io_async_read" with type "test" omitted
# Unsupported target "io_chain" with type "test" omitted
# Unsupported target "io_copy" with type "test" omitted
# Unsupported target "io_driver" with type "test" omitted
# Unsupported target "io_driver_drop" with type "test" omitted
# Unsupported target "io_lines" with type "test" omitted
# Unsupported target "io_read" with type "test" omitted
# Unsupported target "io_read_exact" with type "test" omitted
# Unsupported target "io_read_line" with type "test" omitted
# Unsupported target "io_read_to_end" with type "test" omitted
# Unsupported target "io_read_to_string" with type "test" omitted
# Unsupported target "io_read_until" with type "test" omitted
# Unsupported target "io_split" with type "test" omitted
# Unsupported target "io_take" with type "test" omitted
# Unsupported target "io_write" with type "test" omitted
# Unsupported target "io_write_all" with type "test" omitted
# Unsupported target "io_write_int" with type "test" omitted
# Unsupported target "macros_join" with type "test" omitted
# Unsupported target "macros_pin" with type "test" omitted
# Unsupported target "macros_select" with type "test" omitted
# Unsupported target "macros_test" with type "test" omitted
# Unsupported target "macros_try_join" with type "test" omitted
# Unsupported target "net_bind_resource" with type "test" omitted
# Unsupported target "net_lookup_host" with type "test" omitted
# Unsupported target "no_rt" with type "test" omitted
# Unsupported target "process_issue_2174" with type "test" omitted
# Unsupported target "process_issue_42" with type "test" omitted
# Unsupported target "process_kill_on_drop" with type "test" omitted
# Unsupported target "process_smoke" with type "test" omitted
# Unsupported target "read_to_string" with type "test" omitted
# Unsupported target "rt_basic" with type "test" omitted
# Unsupported target "rt_common" with type "test" omitted
# Unsupported target "rt_threaded" with type "test" omitted
# Unsupported target "signal_ctrl_c" with type "test" omitted
# Unsupported target "signal_drop_recv" with type "test" omitted
# Unsupported target "signal_drop_rt" with type "test" omitted
# Unsupported target "signal_drop_signal" with type "test" omitted
# Unsupported target "signal_multi_rt" with type "test" omitted
# Unsupported target "signal_no_rt" with type "test" omitted
# Unsupported target "signal_notify_both" with type "test" omitted
# Unsupported target "signal_twice" with type "test" omitted
# Unsupported target "signal_usr1" with type "test" omitted
# Unsupported target "stream_chain" with type "test" omitted
# Unsupported target "stream_collect" with type "test" omitted
# Unsupported target "stream_empty" with type "test" omitted
# Unsupported target "stream_fuse" with type "test" omitted
# Unsupported target "stream_iter" with type "test" omitted
# Unsupported target "stream_merge" with type "test" omitted
# Unsupported target "stream_once" with type "test" omitted
# Unsupported target "stream_pending" with type "test" omitted
# Unsupported target "stream_reader" with type "test" omitted
# Unsupported target "stream_stream_map" with type "test" omitted
# Unsupported target "stream_timeout" with type "test" omitted
# Unsupported target "sync_barrier" with type "test" omitted
# Unsupported target "sync_broadcast" with type "test" omitted
# Unsupported target "sync_cancellation_token" with type "test" omitted
# Unsupported target "sync_errors" with type "test" omitted
# Unsupported target "sync_mpsc" with type "test" omitted
# Unsupported target "sync_mutex" with type "test" omitted
# Unsupported target "sync_mutex_owned" with type "test" omitted
# Unsupported target "sync_notify" with type "test" omitted
# Unsupported target "sync_oneshot" with type "test" omitted
# Unsupported target "sync_rwlock" with type "test" omitted
# Unsupported target "sync_semaphore" with type "test" omitted
# Unsupported target "sync_semaphore_owned" with type "test" omitted
# Unsupported target "sync_watch" with type "test" omitted
# Unsupported target "task_blocking" with type "test" omitted
# Unsupported target "task_local" with type "test" omitted
# Unsupported target "task_local_set" with type "test" omitted
# Unsupported target "tcp_accept" with type "test" omitted
# Unsupported target "tcp_connect" with type "test" omitted
# Unsupported target "tcp_echo" with type "test" omitted
# Unsupported target "tcp_into_split" with type "test" omitted
# Unsupported target "tcp_peek" with type "test" omitted
# Unsupported target "tcp_shutdown" with type "test" omitted
# Unsupported target "tcp_split" with type "test" omitted
# Unsupported target "test_clock" with type "test" omitted
# Unsupported target "time_delay" with type "test" omitted
# Unsupported target "time_delay_queue" with type "test" omitted
# Unsupported target "time_interval" with type "test" omitted
# Unsupported target "time_rt" with type "test" omitted
# Unsupported target "time_throttle" with type "test" omitted
# Unsupported target "time_timeout" with type "test" omitted
# Unsupported target "udp" with type "test" omitted
# Unsupported target "uds_cred" with type "test" omitted
# Unsupported target "uds_datagram" with type "test" omitted
# Unsupported target "uds_split" with type "test" omitted
# Unsupported target "uds_stream" with type "test" omitted
""",
        strip_prefix = "tokio-0.2.22",
        type = "tar.gz",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/crates/tokio/tokio-0.2.22.crate",
    )

    http_archive(
        name = "__vcpkg__0_2_10",
        # TODO: Allow configuring where rust_library comes from
        build_file_content = """# buildifier: disable=load
load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

# buildifier: disable=load
load("@bazel_skylib//lib:selects.bzl", "selects")

package(default_visibility = [
    "//visibility:public",
])

licenses([
    "notice",  # MIT from expression "MIT OR Apache-2.0"
])

# Generated targets

# buildifier: leave-alone
rust_library(
    name = "vcpkg",
    crate_type = "lib",
    deps = [
    ],
    srcs = glob(["**/*.rs"]),
    crate_root = "src/lib.rs",
    edition = "2015",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    data = glob(["**"], exclude=[
        # These can be manually added with overrides if needed.

        # If you run `cargo build` in this dir, the target dir can get very big very quick.
        "target/**",

        # These are not vendored from the crate - we exclude them to avoid busting caches
        # when we change how we generate BUILD files and such.
        "BUILD.bazel",
        "WORKSPACE.bazel",
        "WORKSPACE",
    ]),
    version = "0.2.10",
    tags = [
        "cargo-raze",
        "manual",
    ],
    crate_features = [
    ],
    aliases = select({
        # Default
        "//conditions:default": {
        },
    }),
)
""",
        strip_prefix = "vcpkg-0.2.10",
        type = "tar.gz",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/crates/vcpkg/vcpkg-0.2.10.crate",
    )


CRATE_TARGET_NAMES = {
    "bitflags": "@__bitflags__1_2_1//:bitflags",
    "bytes": "@__bytes__0_5_6//:bytes",
    "cc": "@__cc__1_0_62//:cc",
    "lazy_static": "@__lazy_static__1_4_0//:lazy_static",
    "libc": "@__libc__0_2_80//:libc",
    "libz-sys": "@__libz_sys__1_1_2//:libz_sys",
    "pin-project-lite": "@__pin_project_lite__0_1_7//:pin_project_lite",
    "pkg-config": "@__pkg_config__0_3_19//:pkg_config",
    "tokio": "@__tokio__0_2_22//:tokio",
    "vcpkg": "@__vcpkg__0_2_10//:vcpkg",
}

def crate(crate_name):
    """Return the name of the target for the given crate.
    """
    target_name = CRATE_TARGET_NAMES.get(crate_name)
    if target_name == None:
        fail("Unknown crate name: {}".format(crate_name))
    return target_name

def all_deps():
    """Return all standard dependencies explicitly listed in the Cargo.toml or packages list."""
    return [
        crate(crate_name) for crate_name in [
            "bitflags",
            "bytes",
            "cc",
            "lazy_static",
            "libc",
            "libz-sys",
            "pin-project-lite",
            "pkg-config",
            "tokio",
            "vcpkg",
        ]
    ]

def all_proc_macro_deps():
    """Return all proc-macro dependencies explicitly listed in the Cargo.toml or packages list."""
    return [
        crate(crate_name) for crate_name in [
        ]
    ]

def crates_from(label):
    mapping = {
        "//some:Cargo.toml": [crate("bitflags"), crate("bytes"), crate("cc"), crate("lazy_static"), crate("libc"), crate("libz-sys"), crate("pin-project-lite"), crate("pkg-config"), crate("vcpkg")],
    }

    return mapping[_absolutify(label)]

def proc_macro_crates_from(label):
    mapping = {
        "//some:Cargo.toml": [],
    }

    return mapping[_absolutify(label)]

def _absolutify(label):
    if label.startswith("//") or label.startswith("@"):
        return label
    if label.startswith(":"):
        return "//" + native.package_name() + label
    return "//" + native.package_name() + ":" + label
"##;

    // Ignore header which contains a hash
    test(&config)
        .success()
        .stdout(predicates::str::ends_with(want_output));
}

#[test]
fn aliased_deps() {
    let cargo_toml_file = NamedTempFile::with_str_content(
        "Cargo.toml",
        r#"[package]
name = "basic"
version = "0.1.0"
edition = "2018"

[dependencies]
plist = "=1.0.0"
"#,
    )
    .expect("Error making temporary file");

    let config = Config {
        cargo_toml_files: btreemap! { String::from("//some:Cargo.toml") => cargo_toml_file.path().to_path_buf() },
        overrides: Default::default(),
        repository: "https://crates-io.s3-us-west-1.amazonaws.com/crates/crates/{name}/{name}-{version}.crate".to_owned(),
        target_triples: vec!["x86_64-apple-darwin".to_owned()],
        packages: vec![],
        cargo: PathBuf::from(env!("CARGO")),
    };

    let want_output = r#"aliases = select({
        # Default
        "//conditions:default": {
            "@__xml_rs__0_8_3//:xml_rs": "xml_rs",
        },
    })"#;

    // Ignore header which contains a hash
    test(&config)
        .success()
        .stdout(predicates::str::contains(want_output));
}

#[test]
fn git_deps() {
    let cargo_toml_file = NamedTempFile::with_str_content(
        "Cargo.toml",
        r#"[package]
name = "has_git_deps"
version = "0.1.0"
edition = "2018"

[dependencies]
tonic-build = "=0.3.1"
anyhow = "=1.0.33"
itertools = "=0.9.0"
proc-macro2 = "=1.0.24"
quote = "=1.0.7"
syn = "=1.0.45"

[patch.crates-io]
prost = { git = "https://github.com/danburkert/prost.git", rev = "4ded4a98ef339da0b7babd4efee3fbe8adaf746b" }
prost-build = { git = "https://github.com/danburkert/prost.git", rev = "4ded4a98ef339da0b7babd4efee3fbe8adaf746b" }
prost-derive = { git = "https://github.com/danburkert/prost.git", rev = "4ded4a98ef339da0b7babd4efee3fbe8adaf746b" }
prost-types = { git = "https://github.com/danburkert/prost.git", rev = "4ded4a98ef339da0b7babd4efee3fbe8adaf746b" }
"#,
    )
        .expect("Error making temporary file");

    let config = Config {
        cargo_toml_files: btreemap! { String::from("//some:Cargo.toml") => cargo_toml_file.path().to_path_buf() },
        overrides: Default::default(),
        repository: "https://crates-io.s3-us-west-1.amazonaws.com/crates/crates/{name}/{name}-{version}.crate".to_owned(),
        target_triples: vec!["x86_64-apple-darwin".to_owned()],
        packages: vec![],
        cargo: PathBuf::from(env!("CARGO")),
    };

    let wanted_prost = r###"    new_git_repository(
        name = "__prost__0_6_1",
        strip_prefix = "",
        build_file_content = """# buildifier: disable=load
load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

# buildifier: disable=load
load("@bazel_skylib//lib:selects.bzl", "selects")

package(default_visibility = [
    "//visibility:public",
])

licenses([
    "notice",  # Apache-2.0 from expression "Apache-2.0"
])

# Generated targets
# Unsupported target "varint" with type "bench" omitted

# buildifier: leave-alone
rust_library(
    name = "prost",
    crate_type = "lib",
    deps = [
        "@__bytes__0_5_6//:bytes",
    ],
    srcs = glob(["**/*.rs"]),
    crate_root = "src/lib.rs",
    edition = "2018",
    proc_macro_deps = [
        "@__prost_derive__0_6_1//:prost_derive",
    ],
    rustc_flags = [
        "--cap-lints=allow",
    ],
    data = glob(["**"], exclude=[
        # These can be manually added with overrides if needed.

        # If you run `cargo build` in this dir, the target dir can get very big very quick.
        "target/**",

        # These are not vendored from the crate - we exclude them to avoid busting caches
        # when we change how we generate BUILD files and such.
        "BUILD.bazel",
        "WORKSPACE.bazel",
        "WORKSPACE",
    ]),
    version = "0.6.1",
    tags = [
        "cargo-raze",
        "manual",
    ],
    crate_features = [
        "prost-derive",
    ],
    aliases = select({
        # Default
        "//conditions:default": {
        },
    }),
)
""",
        remote = "https://github.com/danburkert/prost.git",
        # TODO: tag?
        commit = "4ded4a98ef339da0b7babd4efee3fbe8adaf746b",
    )"###;

    let unwanted_prost = r###"http_archive(
        name = "__prost__0_6_1","###;

    let wanted_prost_build = r###"    new_git_repository(
        name = "__prost_build__0_6_1",
        strip_prefix = "prost-build",
        build_file_content = """# buildifier: disable=load
load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

# buildifier: disable=load
load("@bazel_skylib//lib:selects.bzl", "selects")

package(default_visibility = [
    "//visibility:public",
])

licenses([
    "notice",  # Apache-2.0 from expression "Apache-2.0"
])

# Generated targets
# buildifier: disable=load-on-top
load(
    "@io_bazel_rules_rust//cargo:cargo_build_script.bzl",
    "cargo_build_script",
)

# buildifier: leave-alone
cargo_build_script(
    name = "prost_build_build_script",
    srcs = glob(["**/*.rs"]),
    crate_root = "build.rs",
    edition = "2018",
    deps = [
        "@__which__4_0_2//:which",
    ],
    rustc_flags = [
        "--cap-lints=allow",
    ],
    crate_features = [
    ],
    build_script_env = {
    },
    data = glob(["**"], exclude=[
        # These can be manually added with overrides if needed.

        # If you run `cargo build` in this dir, the target dir can get very big very quick.
        "target/**",

        # These are not vendored from the crate - we exclude them to avoid busting caches
        # when we change how we generate BUILD files and such.
        "BUILD.bazel",
        "WORKSPACE.bazel",
        "WORKSPACE",
    ]),
    tags = [
        "cargo-raze",
        "manual",
    ],
    version = "0.6.1",
    visibility = ["//visibility:private"],
)


# buildifier: leave-alone
rust_library(
    name = "prost_build",
    crate_type = "lib",
    deps = [
        ":prost_build_build_script",
        "@__bytes__0_5_6//:bytes",
        "@__heck__0_3_1//:heck",
        "@__itertools__0_9_0//:itertools",
        "@__log__0_4_11//:log",
        "@__multimap__0_8_2//:multimap",
        "@__petgraph__0_5_1//:petgraph",
        "@__prost__0_6_1//:prost",
        "@__prost_types__0_6_1//:prost_types",
        "@__tempfile__3_1_0//:tempfile",
    ],
    srcs = glob(["**/*.rs"]),
    crate_root = "src/lib.rs",
    edition = "2018",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    data = glob(["**"], exclude=[
        # These can be manually added with overrides if needed.

        # If you run `cargo build` in this dir, the target dir can get very big very quick.
        "target/**",

        # These are not vendored from the crate - we exclude them to avoid busting caches
        # when we change how we generate BUILD files and such.
        "BUILD.bazel",
        "WORKSPACE.bazel",
        "WORKSPACE",
    ]),
    version = "0.6.1",
    tags = [
        "cargo-raze",
        "manual",
    ],
    crate_features = [
    ],
    aliases = select({
        # Default
        "//conditions:default": {
        },
    }),
)
""",
        remote = "https://github.com/danburkert/prost.git",
        # TODO: tag?
        commit = "4ded4a98ef339da0b7babd4efee3fbe8adaf746b",
    )"###;

    let unwanted_prost_build = r###"http_archive(
        name = "__prost_build__0_6_1","###;

    let wanted_prost_derive = r###"new_git_repository(
        name = "__prost_derive__0_6_1",
        strip_prefix = "prost-derive",
        build_file_content = """# buildifier: disable=load
load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

# buildifier: disable=load
load("@bazel_skylib//lib:selects.bzl", "selects")

package(default_visibility = [
    "//visibility:public",
])

licenses([
    "notice",  # Apache-2.0 from expression "Apache-2.0"
])

# Generated targets

# buildifier: leave-alone
rust_library(
    name = "prost_derive",
    crate_type = "proc-macro",
    deps = [
        "@__anyhow__1_0_33//:anyhow",
        "@__itertools__0_9_0//:itertools",
        "@__proc_macro2__1_0_24//:proc_macro2",
        "@__quote__1_0_7//:quote",
        "@__syn__1_0_45//:syn",
    ],
    srcs = glob(["**/*.rs"]),
    crate_root = "src/lib.rs",
    edition = "2018",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    data = glob(["**"], exclude=[
        # These can be manually added with overrides if needed.

        # If you run `cargo build` in this dir, the target dir can get very big very quick.
        "target/**",

        # These are not vendored from the crate - we exclude them to avoid busting caches
        # when we change how we generate BUILD files and such.
        "BUILD.bazel",
        "WORKSPACE.bazel",
        "WORKSPACE",
    ]),
    version = "0.6.1",
    tags = [
        "cargo-raze",
        "manual",
    ],
    crate_features = [
    ],
    aliases = select({
        # Default
        "//conditions:default": {
        },
    }),
)
""",
        remote = "https://github.com/danburkert/prost.git",
        # TODO: tag?
        commit = "4ded4a98ef339da0b7babd4efee3fbe8adaf746b",
    )"###;

    let unwanted_prost_derive = r###"http_archive(
        name = "__prost_derive__0_6_1","###;

    let wanted_prost_types = r###"new_git_repository(
        name = "__prost_types__0_6_1",
        strip_prefix = "prost-types",
        build_file_content = """# buildifier: disable=load
load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

# buildifier: disable=load
load("@bazel_skylib//lib:selects.bzl", "selects")

package(default_visibility = [
    "//visibility:public",
])

licenses([
    "notice",  # Apache-2.0 from expression "Apache-2.0"
])

# Generated targets

# buildifier: leave-alone
rust_library(
    name = "prost_types",
    crate_type = "lib",
    deps = [
        "@__bytes__0_5_6//:bytes",
        "@__prost__0_6_1//:prost",
    ],
    srcs = glob(["**/*.rs"]),
    crate_root = "src/lib.rs",
    edition = "2018",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    data = glob(["**"], exclude=[
        # These can be manually added with overrides if needed.

        # If you run `cargo build` in this dir, the target dir can get very big very quick.
        "target/**",

        # These are not vendored from the crate - we exclude them to avoid busting caches
        # when we change how we generate BUILD files and such.
        "BUILD.bazel",
        "WORKSPACE.bazel",
        "WORKSPACE",
    ]),
    version = "0.6.1",
    tags = [
        "cargo-raze",
        "manual",
    ],
    crate_features = [
    ],
    aliases = select({
        # Default
        "//conditions:default": {
        },
    }),
)
""",
        remote = "https://github.com/danburkert/prost.git",
        # TODO: tag?
        commit = "4ded4a98ef339da0b7babd4efee3fbe8adaf746b",
    )"###;

    let unwanted_prost_types = r###"http_archive(
        name = "__prost_types__0_6_1","###;

    let result = test(&config).success();
    result
        .stdout(predicates::str::contains(wanted_prost))
        .stdout(predicates::str::contains(wanted_prost_build))
        .stdout(predicates::str::contains(wanted_prost_derive))
        .stdout(predicates::str::contains(wanted_prost_types))
        .stdout(predicates::str::contains(unwanted_prost).not())
        .stdout(predicates::str::contains(unwanted_prost_build).not())
        .stdout(predicates::str::contains(unwanted_prost_derive).not())
        .stdout(predicates::str::contains(unwanted_prost_types).not());
}

// #[test] // TODO: Unignore when we fix workspace support - currently broken by the fact that we generate our Cargo.toml somewhere standalone but don't strip out the workspace information
fn workspace_root() {
    let dir = tempfile::tempdir().expect("Could not make tempdir");
    let subdir = dir.path().join("subcrate");
    create_dir_all(&subdir).expect("Could not make subcrate dir");
    let workspace_cargo_toml = dir.path().join("Cargo.toml");
    std::fs::write(
        &workspace_cargo_toml,
        r#"[workspace]
members = ["subcrate"]

[package]
name = "ws"
version = "0.1.0"
edition = "2018"

[dependencies]
lazy_static = "=1.4.0"

# TODO: do not depend on this section to be present rules_rust_external.
[lib]
path = "lib.rs"
"#
        .as_bytes(),
    )
    .expect("Failed to write Cargo.toml");

    std::fs::write(
        subdir.join("Cargo.toml"),
        r#"[package]
name = "subcrate"
version = "0.1.0"
edition = "2018"

[dependencies]
bitflags = "=1.2.1"

# TODO: do not depend on this section to be present rules_rust_external.
[lib]
path = "lib.rs"
"#
        .as_bytes(),
    )
    .expect("Failed to write Cargo.toml");

    let config = Config {
        cargo_toml_files: btreemap! { String::from("//some/other:Cargo.toml") => workspace_cargo_toml },
        overrides: Default::default(),
        repository: "https://crates-io.s3-us-west-1.amazonaws.com/crates/crates/{name}/{name}-{version}.crate".to_owned(),
        target_triples: vec!["x86_64-apple-darwin".to_owned()],
        packages: vec![],
        cargo: PathBuf::from(env!("CARGO")),
    };

    test(&config).success().stdout(eq(r#"load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

def pinned_rust_install():
    http_archive(
    name = "bitflags",
    # TODO: Allow configuring where rust_library comes from
    build_file_content = """load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//cargo:cargo_build_script.bzl", "cargo_build_script")

cargo_build_script(
    name = "bitflags_build_script",
    srcs = glob(["**/*.rs"]),
    crate_root = "build.rs",
    deps = [],
    proc_macro_deps = [],
    data = glob(["**"]),
    edition = "2015",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    crate_features = [
        "default"
    ],
    version = "1.2.1",
)


rust_library(
    name = "bitflags",
    srcs = glob(["**/*.rs"]),
    crate_features = [
        "default"
    ],
    crate_root = "src/lib.rs",
    crate_type = "lib",
    edition = "2015",
    proc_macro_deps = [],
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "1.2.1",
    visibility = ["//visibility:public"],
    deps = [
        ":bitflags_build_script"
    ],
)
""",
    strip_prefix = "bitflags-1.2.1",
    type = "tar.gz",
    url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/crates/bitflags/bitflags-1.2.1.crate",
)
    http_archive(
    name = "lazy_static",
    # TODO: Allow configuring where rust_library comes from
    build_file_content = """load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")

rust_library(
    name = "lazy_static",
    srcs = glob(["**/*.rs"]),
    crate_features = [],
    crate_root = "src/lib.rs",
    crate_type = "lib",
    edition = "2015",
    proc_macro_deps = [],
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "1.4.0",
    visibility = ["//visibility:public"],
    deps = [],
)
""",
    strip_prefix = "lazy_static-1.4.0",
    type = "tar.gz",
    url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/crates/lazy_static/lazy_static-1.4.0.crate",
)
"#
    ));
}

fn test(config: &Config) -> Assert {
    Command::cargo_bin("resolver")
        .unwrap()
        .arg("--input_path")
        .arg("/dev/stdin")
        .arg("--output_path")
        .arg("/dev/stdout")
        .arg("--repo-name")
        .arg("whatever")
        .write_stdin(serde_json::to_string(&config).unwrap())
        .assert()
}

// Tests still to add:
// Transitive deps
// Lib and bin in the same crate
