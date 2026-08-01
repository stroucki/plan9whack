<!-- cargo-rdme start -->

This crate is a Rust port of Plan9's whack compression scheme as used
within the venti storage system. Original authors unknown, C source
came via Russ Cox and the 9fans/plan9port repository.

Use the `unwhack` function to decompress, and `whackblock` to compress.
A `whack` function also exists if you want to control some parameters
of compression, or want to collect statistics.

Internally, whack walks through the input by byte and tries to look up the
presence of a three byte set in a dictionary. If it was not previously
seen, a literal is emitted, where ASCII runs get favorable encoding, but
based on history can switch back to neutral binary encoding.

If the trigraph was previously seen, depending on the Whack initialization
parameter, additional searches in the dictionary will be made until a
length minimum is satisfied, or there are no more matches. A (length,
offset) pair is emitted, where the length is a variable length encoded
integer between 3 and 2051, derived by a fixed Huffman tree
representation. The offset consists of a bit count, favoring lower
offsets, followed by the bits of the offset with the leading 1 bit
omitted.

The trigraph at each point is hashed and is the key to a table pointing to
an offset in the history. The previous value at that hash is given to a
next entry keyed by the current dictionary position.

<!-- cargo-rdme end -->
