# interrupt-read ![License: MIT](https://img.shields.io/badge/license-MIT-blue) [![interrupt-read on crates.io](https://img.shields.io/crates/v/interrupt-read)](https://crates.io/crates/interrupt-read) [![interrupt-read on docs.rs](https://docs.rs/interrupt-read/badge.svg)](https://docs.rs/interrupt-read) [![Source Code Repository](https://img.shields.io/badge/Code-On%20GitHub-blue?logo=GitHub)](https://github.com/AhoyISki/interrupt-read)

An interruptable [`Read`][__link0]er

This crate provides the [`InterruptReader`][__link1], which can have its
`read` operations interrupted by an [`Interruptor`][__link2]. They are
acquired from the [`interrupt_reader::pair`][__link3] function, which
returns an [`mpsc`][__link4] channel backed pair.

When [`Interruptor::interrupt`][__link5] is called, the `InterruptReader`
will return an erro of kind [`ErrorKind::Other`][__link6] with a payload of
[`InterruptReceived`][__link7] (you can check for that using the
[`is_interrupt`][__link8] function). Otherwise, it will act like any normal
`Read` struct.

When an interrupt is received, *the underlying data is not lost*,
it still exists, and if you call a reading function again, it will
be retrieved, unless another interrupt is sent before that.

Some things to note about this crate:

* It functions by spawning a separate thread, which will actually
  read from the original `Read`er, so keep that in mind.
* There is some (light) overhead over the read operations.
* You should *not* wrap this struct in a [`BufReader`][__link9] since the
  struct already has its own internal buffer.
* This reader doesn’t assume that `Ok(0)` is the end of input, and
  the spawned thread will only terminate if the
  [`InterruptReader`][__link10] is dropped.

## Note

The reason why this function returns [`ErrorKind::Other`][__link11], rather
than [`ErrorKind::Interrupted`][__link12] is that the latter error is
ignored by functions like [`BufRead::read_line`][__link13] and
[`BufRead::read_until`][__link14], which is probably not what you want to
happen.


 [__cargo_doc2readme_dependencies_info]: ggGkYW0BYXSEG_W_Gn_kaocAGwCcVPfenh7eGy6gYLEwyIe4G6-xw_FwcbpjYXKEG7OWreJQsAuaG0YQ155oPkvhG_NizlGmKBhtG83DvCigfVzOYWSBg25pbnRlcnJ1cHQtcmVhZGUwLjQuMG5pbnRlcnJ1cHRfcmVhZA
 [__link0]: https://doc.rust-lang.org/stable/std/?search=io::Read
 [__link1]: https://docs.rs/interrupt-read/0.4.0/interrupt_read/struct.InterruptReader.html
 [__link10]: https://docs.rs/interrupt-read/0.4.0/interrupt_read/struct.InterruptReader.html
 [__link11]: https://doc.rust-lang.org/stable/std/?search=io::ErrorKind::Other
 [__link12]: https://doc.rust-lang.org/stable/std/?search=io::ErrorKind::Interrupted
 [__link13]: https://doc.rust-lang.org/stable/std/?search=io::BufRead::read_line
 [__link14]: https://doc.rust-lang.org/stable/std/?search=io::BufRead::read_until
 [__link2]: https://docs.rs/interrupt-read/0.4.0/interrupt_read/struct.Interruptor.html
 [__link3]: https://docs.rs/interrupt-read/0.4.0/interrupt_read/?search=pair
 [__link4]: https://doc.rust-lang.org/stable/std/?search=sync::mpsc
 [__link5]: https://docs.rs/interrupt-read/0.4.0/interrupt_read/?search=Interruptor::interrupt
 [__link6]: https://doc.rust-lang.org/stable/std/?search=io::ErrorKind::Other
 [__link7]: https://docs.rs/interrupt-read/0.4.0/interrupt_read/struct.InterruptReceived.html
 [__link8]: https://docs.rs/interrupt-read/0.4.0/interrupt_read/?search=is_interrupt
 [__link9]: https://doc.rust-lang.org/stable/std/?search=io::BufReader
