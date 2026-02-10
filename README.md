# interrupt-read ![License](https://img.shields.io/crates/l/interrupt-read) [![interrupt-read on crates.io](https://img.shields.io/crates/v/interrupt-read)](https://crates.io/crates/interrupt-read) [![interrupt-read on docs.rs](https://docs.rs/interrupt-read/badge.svg)](https://docs.rs/interrupt-read)

An interruptable [`Read`][__link0]er

This crate provides the [`InterruptReader`][__link1], which can have its
`read` operations interrupted by an [`Interruptor`][__link2]. They are
acquired from the [`interrupt_reader::pair`][__link3] function, which
returns an [`mpsc`][__link4] channel backed pair.

When [`Interruptor::interrupt`][__link5] is called, the `InterruptReader`
will return an erro of kind [`ErrorKind::Interrupted`][__link6]. Otherwise,
it will act like any normal `Read` struct.

Some things to note about this crate:

* It functions by spawning a separate thread, which will actually
  read from the original `Read`er, so keep that in mind.
* There is some (light) overhead over the read operations.
* You should *not* wrap this struct in a [`BufReader`][__link7] since the
  struct already has its own internal buffer.
* This reader doesn’t assume that `Ok(0)` is the end of input, and
  the spawned thread will only terminate if the
  [`InterruptReader`][__link8] is dropped.


 [__cargo_doc2readme_dependencies_info]: ggGkYW0BYXSEG_W_Gn_kaocAGwCcVPfenh7eGy6gYLEwyIe4G6-xw_FwcbpjYXKEGyrMQVQfJHh3G_yz-C-zJWCrGyA0mIRwpLs6G5hCxXL4EJC0YWSCg25pbnRlcnJ1cHQtcmVhZGUwLjEuMG5pbnRlcnJ1cHRfcmVhZIJwaW50ZXJydXB0X3JlYWRlcvY
 [__link0]: https://doc.rust-lang.org/stable/std/?search=io::Read
 [__link1]: https://docs.rs/interrupt-read/0.1.0/interrupt_read/struct.InterruptReader.html
 [__link2]: https://docs.rs/interrupt-read/0.1.0/interrupt_read/struct.Interruptor.html
 [__link3]: https://docs.rs/interrupt_reader/latest/interrupt_reader/?search=pair
 [__link4]: https://doc.rust-lang.org/stable/std/?search=sync::mpsc
 [__link5]: https://docs.rs/interrupt-read/0.1.0/interrupt_read/?search=Interruptor::interrupt
 [__link6]: https://doc.rust-lang.org/stable/std/?search=io::ErrorKind::Interrupted
 [__link7]: https://doc.rust-lang.org/stable/std/?search=io::BufReader
 [__link8]: https://docs.rs/interrupt-read/0.1.0/interrupt_read/struct.InterruptReader.html
