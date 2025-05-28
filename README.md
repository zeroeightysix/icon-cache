# icon-cache

Complete and user-friendly zero-copy wrappers for the GTK icon cache which is present on most
linux systems.

GTK's icon cache maintains a hash-indexed map from icon names (e.g. `open-menu`) to a list of
images representing that icon, each in a different directory, usually denoting that icon's size,
whether it's scalable, etc.

This crate provides a safe wrapper around this cache and is designed for use with `mmap`.
