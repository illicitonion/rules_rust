# rules_rust_external

## Using rules_rust_external

Add the following to your WORKSPACE:

```python

local_repository(
    name = "rules_rust_external",
    path = "/path/to/rules_rust/rules_rust_external",
)

load("@rules_rust_external//:repositories.bzl", "rust_external_deps")
rust_external_deps()

```
