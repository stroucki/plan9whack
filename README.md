<!-- cargo-rdme start -->

This crate is a Rust port of Plan9's whack compression scheme as used
within the venti storage system. Original authors unknown, C source
came via Russ Cox and the 9fans/plan9port repository.

Use the `unwhack` function to decompress, and `whackblock` to compress.
A `whack` function also exists if you want to control some parameters
of compression, or want to collect statistics.

<!-- cargo-rdme end -->
