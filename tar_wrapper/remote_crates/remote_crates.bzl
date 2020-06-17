load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

def fetch_remote_crates():
    http_archive(
        name = "adler32",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/adler32/adler32-1.1.0.crate",
        sha256 = "567b077b825e468cc974f0020d4082ee6e03132512f207ef1a02fd5d00d1f32d",
        type = "tar.gz",
        strip_prefix = "adler32-1.1.0",
        build_file = Label("//tar_wrapper/remote_crates:adler32-1.1.0.BUILD"),
    )

    http_archive(
        name = "cfg_if",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/cfg-if/cfg-if-0.1.10.crate",
        sha256 = "4785bdd1c96b2a846b2bd7cc02e86b6b3dbf14e7e53446c4f54c92a361040822",
        type = "tar.gz",
        strip_prefix = "cfg-if-0.1.10",
        build_file = Label("//tar_wrapper/remote_crates:cfg-if-0.1.10.BUILD"),
    )

    http_archive(
        name = "crc32fast",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/crc32fast/crc32fast-1.2.0.crate",
        sha256 = "ba125de2af0df55319f41944744ad91c71113bf74a4646efff39afe1f6842db1",
        type = "tar.gz",
        strip_prefix = "crc32fast-1.2.0",
        build_file = Label("//tar_wrapper/remote_crates:crc32fast-1.2.0.BUILD"),
    )

    http_archive(
        name = "filetime",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/filetime/filetime-0.2.10.crate",
        sha256 = "affc17579b132fc2461adf7c575cc6e8b134ebca52c51f5411388965227dc695",
        type = "tar.gz",
        strip_prefix = "filetime-0.2.10",
        build_file = Label("//tar_wrapper/remote_crates:filetime-0.2.10.BUILD"),
    )

    http_archive(
        name = "flate2",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/flate2/flate2-1.0.14.crate",
        sha256 = "2cfff41391129e0a856d6d822600b8d71179d46879e310417eb9c762eb178b42",
        type = "tar.gz",
        strip_prefix = "flate2-1.0.14",
        build_file = Label("//tar_wrapper/remote_crates:flate2-1.0.14.BUILD"),
    )

    http_archive(
        name = "miniz_oxide",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/miniz_oxide/miniz_oxide-0.3.7.crate",
        sha256 = "791daaae1ed6889560f8c4359194f56648355540573244a5448a83ba1ecc7435",
        type = "tar.gz",
        strip_prefix = "miniz_oxide-0.3.7",
        build_file = Label("//tar_wrapper/remote_crates:miniz_oxide-0.3.7.BUILD"),
    )

    http_archive(
        name = "tar",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/tar/tar-0.4.29.crate",
        sha256 = "c8a4c1d0bee3230179544336c15eefb563cf0302955d962e456542323e8c2e8a",
        type = "tar.gz",
        strip_prefix = "tar-0.4.29",
        build_file = Label("//tar_wrapper/remote_crates:tar-0.4.29.BUILD"),
    )

    http_archive(
        name = "xattr",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/xattr/xattr-0.2.2.crate",
        sha256 = "244c3741f4240ef46274860397c7c74e50eb23624996930e484c16679633a54c",
        type = "tar.gz",
        strip_prefix = "xattr-0.2.2",
        build_file = Label("//tar_wrapper/remote_crates:xattr-0.2.2.BUILD"),
    )
