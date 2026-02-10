//! An interruptable [`Read`]er
//!
//! This crate provides the [`InterruptReader`], which can have its
//! `read` operations interrupted by an [`Interruptor`]. They are
//! acquired from the [`interrupt_reader::pair`] function, which
//! returns an [`mpsc`] channel backed pair.
//!
//! When [`Interruptor::interrupt`] is called, the `InterruptReader`
//! will return an erro of kind [`ErrorKind::Other`] with a payload of
//! [`InterruptReceived`] (you can check for that using the
//! [`is_interrupt`] function). Otherwise, it will act like any normal
//! `Read` struct.
//!
//! Some things to note about this crate:
//!
//! - It functions by spawning a separate thread, which will actually
//!   read from the original `Read`er, so keep that in mind.
//! - There is some (light) overhead over the read operations.
//! - You should _not_ wrap this struct in a [`BufReader`] since the
//!   struct already has its own internal buffer.
//! - This reader doesn't assume that `Ok(0)` is the end of input, and
//!   the spawned thread will only terminate if the
//!   [`InterruptReader`] is dropped.
//!
//! # Note
//!
//! The reason why this function returns [`ErrorKind::Other`], rather
//! than [`ErrorKind::Interrupted`] is that the latter error is
//! ignored by functions like [`BufRead::read_line`] and
//! [`BufRead::read_until`], which is probably not what you want to
//! happen.
//!
//! [`BufReader`]: std::io::BufReader
//! [`ErrorKind::Other`]: std::io::ErrorKind::Other
//! [`ErrorKind::Interrupted`]: std::io::ErrorKind::Interrupted
use std::{
    io::{BufRead, Cursor, Error, Read, Take},
    sync::mpsc,
    thread::JoinHandle,
};

/// Returns a pair of an [`InterruptReader`] and an [`Interruptor`].
///
/// When you call any of the reading methods of `InterruptReader`, the
/// current thread will block, being unblocked only if:
///
/// - The underlying [`Read`]er has more bytes or returned an
///   [`Error`].
/// - The [`Interruptor::interrupt`] function was called.
///
/// In the former case, it works just like a regular read, giving an
/// [`std::io::Result`], depending on the operation.
/// If the latter happens, however, an [`Error`] of type
/// [`ErrorKind::Other`] with a payload of [`InterruptReceived`],
/// meaning that reading operations have been interrupted for some
/// user defined reason.
///
/// You can check if an [`std::io::Error`] is of this type by
/// calling the [`is_interrupt`] function.
///
/// If the channel was interrupted this way, further reads will work
/// just fine, until another interrupt comes through, creating a
/// read/interrupt cycle.
///
/// Behind the scenes, this is done through channels and a spawned
/// thread, but no timeout is used, all operations are blocking.
///
/// [`Error`]: std::io::Error
/// [`ErrorKind::Other`]: std::io::ErrorKind::Other
pub fn pair<R: Read + Send + 'static>(mut reader: R) -> (InterruptReader<R>, Interruptor) {
    let (event_tx, event_rx) = mpsc::channel();
    let (buffer_tx, buffer_rx) = mpsc::channel();

    let join_handle = std::thread::spawn({
        let event_tx = event_tx.clone();
        move || {
            // Same capacity as BufReader
            let mut buf = vec![0; 8 * 1024];

            loop {
                match reader.read(&mut buf) {
                    Ok(num_bytes) => {
                        // This means the InterruptReader has been dropped, so no more reading
                        // will be done.
                        let event = Event::Buf(std::mem::take(&mut buf), num_bytes);
                        if event_tx.send(event).is_err() {
                            break reader;
                        }

                        buf = match buffer_rx.recv() {
                            Ok(buf) => buf,
                            // Same as before.
                            Err(_) => break reader,
                        }
                    }
                    Err(err) => {
                        if event_tx.send(Event::Err(err)).is_err() {
                            break reader;
                        }
                    }
                }
            }
        }
    });

    let interrupt_reader = InterruptReader {
        cursor: None,
        buffer_tx,
        event_rx,
        join_handle,
    };
    let interruptor = Interruptor(event_tx);

    (interrupt_reader, interruptor)
}

/// An interruptable, buffered [`Read`]er.
///
/// This reader is created by wrapping a `Read` struct in the
/// [`interrupt_read::pair`] function, which also returns an
/// [`Interruptor`], which is capable of sending interrupt signals,
/// which make any `read` operations on the `InterruptReader` return
/// an error of kind [`ErrorKind::Other`], with a payload of
/// [`InterruptReceived`].
///
/// You can check if an [`std::io::Error`] is of this type by
/// calling the [`is_interrupt`] function.
///
/// # Examples
///
/// One potential application of this struct is if you want to stop a
/// thread that is reading from the stdout of a child process without
/// necessarily terminating said childrop_:
///
/// ```rust
/// use std::{
///     io::{BufRead, ErrorKind},
///     process::{Child, Command, Stdio},
///     time::Duration,
/// };
///
/// use interrupt_read::{is_interrupt, pair};
///
/// struct ChildKiller(Child);
/// impl Drop for ChildKiller {
///     fn drop(&mut self) {
///         _ = self.0.kill();
///     }
/// }
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Prints "hello\n" every second forever.
/// let mut child = Command::new("bash")
///     .args(["-c", r#"while true; do echo "hello"; sleep 1; done"#])
///     .stdout(Stdio::piped())
///     .spawn()
///     .unwrap();
///
/// let (mut stdout, interruptor) = pair(child.stdout.take().unwrap());
/// let _child_killer = ChildKiller(child);
///
/// let join_handle = std::thread::spawn(move || {
///     let mut string = String::new();
///     loop {
///         match stdout.read_line(&mut string) {
///             Ok(0) => break Ok(string),
///             Ok(_) => {}
///             Err(err) if is_interrupt(&err) => {
///                 break Ok(string);
///             }
///             Err(err) => break Err(err),
///         }
///     }
/// });
///
/// std::thread::sleep(Duration::new(3, 1_000_000));
///
/// interruptor.interrupt()?;
///
/// let result = join_handle.join().unwrap()?;
///
/// assert_eq!(result, "hello\nhello\nhello\n");
///
/// Ok(())
/// # }
/// # match main() {
/// #     Ok(()) => {}
/// #     Err(err) => panic!("{err}")
/// # }
/// ```
///
/// [`interrupt_read::pair`]: pair
/// [`ErrorKind::Other`]: std::io::ErrorKind::Other
#[derive(Debug)]
pub struct InterruptReader<R> {
    cursor: Option<Take<Cursor<Vec<u8>>>>,
    buffer_tx: mpsc::Sender<Vec<u8>>,
    event_rx: mpsc::Receiver<Event>,
    join_handle: JoinHandle<R>,
}

impl<R: Read> InterruptReader<R> {
    /// Unwraps this `InterruptReader`, returning the underlying
    /// reader.
    ///
    /// Note that any leftover data in the internal buffer is lost.
    /// Therefore, a following read from the underlying reader may
    /// lead to data loss.
    ///
    /// This may return [`Err`] if the underlying joined thread has
    /// panicked, probably because the [`Read`]er has done so.
    pub fn into_inner(self) -> std::thread::Result<R> {
        let Self { buffer_tx, event_rx, join_handle, .. } = self;
        drop(event_rx);
        drop(buffer_tx);
        join_handle.join()
    }
}

impl<R: Read> Read for InterruptReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if let Some(cursor) = self.cursor.as_mut() {
            deal_with_interrupt(&self.event_rx)?;

            match cursor.read(buf) {
                Ok(0) => {
                    let buffer = self.cursor.take().unwrap().into_inner().into_inner();
                    match self.buffer_tx.send(buffer) {
                        Ok(()) => self.read(buf),
                        // Now we handle that.
                        Err(_) => Ok(0),
                    }
                }
                Ok(num_bytes) => Ok(num_bytes),
                Err(_) => unreachable!("Afaik, this shouldn't happen if T is Vec<u8>"),
            }
        } else {
            match self.event_rx.recv() {
                Ok(Event::Buf(buffer, len)) => {
                    self.cursor = Some(Cursor::new(buffer).take(len as u64));
                    if len == 0 { Ok(0) } else { self.read(buf) }
                }
                Ok(Event::Err(err)) => Err(err),
                Ok(Event::Interrupt) => Err(interrupt_error()),
                Err(_) => Ok(0),
            }
        }
    }
}

impl<R: Read> BufRead for InterruptReader<R> {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        if let Some(cursor) = self.cursor.as_mut() {
            deal_with_interrupt(&self.event_rx)?;

            let (addr, len) = {
                let buf = cursor.fill_buf()?;
                ((buf as *const [u8]).addr(), buf.len())
            };

            if len == 0 {
                let buffer = self.cursor.take().unwrap().into_inner().into_inner();
                match self.buffer_tx.send(buffer) {
                    Ok(()) => self.fill_buf(),
                    Err(_) => Ok(&[]),
                }
            } else {
                let buffer = self.cursor.as_ref().unwrap().get_ref().get_ref();
                let buf_addr = (buffer.as_slice() as *const [u8]).addr();

                // First time the borrow checker actually forced me to do something
                // inconvenient, instead of the safe alternative.
                Ok(&buffer[addr - buf_addr..(addr - buf_addr) + len])
            }
        } else {
            match self.event_rx.recv() {
                Ok(Event::Buf(buffer, len)) => {
                    self.cursor = Some(Cursor::new(buffer).take(len as u64));
                    if len == 0 { Ok(&[]) } else { self.fill_buf() }
                }
                Ok(Event::Err(err)) => Err(err),
                Ok(Event::Interrupt) => Err(interrupt_error()),
                Err(_) => Ok(&[]),
            }
        }
    }

    fn consume(&mut self, amount: usize) {
        if let Some(cursor) = self.cursor.as_mut() {
            cursor.consume(amount);
        }
    }
}

/// An interruptor for an [`InterruptReader`].
///
/// This struct serves the purpose of interrupting any of the [`Read`]
/// or [`BufRead`] functions being performend on the `InterruptReader`
///
/// If it is dropped, the `InterruptReader` will no longer be able to
/// be interrupted.
#[derive(Debug, Clone)]
pub struct Interruptor(mpsc::Sender<Event>);

impl Interruptor {
    /// Interrupts the [`InterruptReader`]
    ///
    /// This will send an interrupt event to the reader, which makes
    /// the next `read` operation return [`Err`], with an
    /// [`ErrorKind::Other`] with a payload of [`InterruptReceived`].
    ///
    /// You can check if an [`std::io::Error`] is of this type by
    /// calling the [`is_interrupt`] function.
    ///
    /// Subsequent `read` operations proceed as normal.
    ///
    /// [`ErrorKind::Other`]: std::io::ErrorKind::Other
    pub fn interrupt(&self) -> Result<(), InterruptSendError> {
        self.0
            .send(Event::Interrupt)
            .map_err(|_| InterruptSendError)
    }
}

/// An error ocurred while calling [`Interruptor::interrupt`].
///
/// This means that the receiving [`InterruptReader`] has been
/// dropped.
#[derive(Debug, Clone, Copy)]
pub struct InterruptSendError;

impl std::fmt::Display for InterruptSendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InterruptReader has been dropped")
    }
}

impl std::error::Error for InterruptSendError {}

/// Indicates that an [`Interruptor`] has called
/// [`Interruptor::interrupt`], causing a read operation to be
/// interrupted.
#[derive(Debug, Clone, Copy)]
pub struct InterruptReceived;

impl std::fmt::Display for InterruptReceived {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Interruptor has interrupted")
    }
}

impl std::error::Error for InterruptReceived {}

#[derive(Debug)]
enum Event {
    Buf(Vec<u8>, usize),
    Err(std::io::Error),
    Interrupt,
}

/// Wether the error in question originated from an [`Interruptor`]
/// calling [`Interruptor::interrupt`].
///
/// This just checks if the error is of type [`InterruptReceived`]..
pub fn is_interrupt(err: &Error) -> bool {
    err.get_ref()
        .is_some_and(|err| err.is::<InterruptReceived>())
}

fn interrupt_error() -> Error {
    Error::other(InterruptReceived)
}

fn deal_with_interrupt(event_rx: &mpsc::Receiver<Event>) -> std::io::Result<()> {
    match event_rx.try_recv() {
        Ok(Event::Interrupt) => Err(interrupt_error()),
        Ok(_) => unreachable!("This should not be possible"),
        // The channel was dropped, but no need to handle that right now.
        Err(_) => Ok(()),
    }
}
