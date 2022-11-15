// This software is a fork of https://github.com/slog-rs/json at version 2.6.1 and is licensed under the MIT as below:
// Some of the other changes made are removing default, set_newline and set_pretty to reduce code size and speed as well as changing the extern crate to 2018+ use.
// Documentation for the original crate can be found at https://docs.rs/slog-json/2.7.0/slog_json/ and has been removed from this crate.

/*
Copyright (c) 2014 The Rust Project Developers

Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
*/

#![warn(missing_docs)]
use slog;

use serde::ser::SerializeMap;
use serde::serde_if_integer128;
use slog::Key;
use slog::Record;
use slog::o;
use slog::{FnValue, PushFnValue};
use slog::{OwnedKVList, SendSyncRefUnwindSafeKV, KV};
use std::{fmt, io, result};

use std::cell::RefCell;
use std::fmt::Write;
    
// Serialize
thread_local! {
    static TL_BUF: RefCell<String> = RefCell::new(String::with_capacity(128))
}

/// `slog::Serializer` adapter for `serde::Serializer`
///
/// Newtype to wrap serde Serializer, so that `Serialize` can be implemented
/// for it
struct SerdeSerializer<S: serde::Serializer> {
    /// Current state of map serializing: `serde::Serializer::MapState`
    ser_map: S::SerializeMap,
}

impl<S: serde::Serializer> SerdeSerializer<S> {
    /// Start serializing map of values
    fn start(ser: S, len: Option<usize>) -> result::Result<Self, slog::Error> {
        let ser_map = ser.serialize_map(len).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("serde serialization error: {}", e),
            )
        })?;
        Ok(SerdeSerializer { ser_map })
    }

    /// Finish serialization, and return the serializer
    fn end(self) -> result::Result<S::Ok, S::Error> {
        self.ser_map.end()
    }
}

macro_rules! impl_m(
    ($s:expr, $key:expr, $val:expr) => ({
        let k_s:  &str = $key.as_ref();
        $s.ser_map.serialize_entry(k_s, $val)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("serde serialization error: {}", e)))?;
        Ok(())
    });
);

impl<S> slog::Serializer for SerdeSerializer<S>
where
    S: serde::Serializer,
{
    fn emit_bool(&mut self, key: Key, val: bool) -> slog::Result {
        impl_m!(self, key, &val)
    }

    fn emit_unit(&mut self, key: Key) -> slog::Result {
        impl_m!(self, key, &())
    }

    fn emit_char(&mut self, key: Key, val: char) -> slog::Result {
        impl_m!(self, key, &val)
    }

    fn emit_none(&mut self, key: Key) -> slog::Result {
        let val: Option<()> = None;
        impl_m!(self, key, &val)
    }
    fn emit_u8(&mut self, key: Key, val: u8) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_i8(&mut self, key: Key, val: i8) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_u16(&mut self, key: Key, val: u16) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_i16(&mut self, key: Key, val: i16) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_usize(&mut self, key: Key, val: usize) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_isize(&mut self, key: Key, val: isize) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_u32(&mut self, key: Key, val: u32) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_i32(&mut self, key: Key, val: i32) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_f32(&mut self, key: Key, val: f32) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_u64(&mut self, key: Key, val: u64) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_i64(&mut self, key: Key, val: i64) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_f64(&mut self, key: Key, val: f64) -> slog::Result {
        impl_m!(self, key, &val)
    }
    serde_if_integer128! {
        fn emit_u128(&mut self, key: Key, val: u128) -> slog::Result {
            impl_m!(self, key, &val)
        }
        fn emit_i128(&mut self, key: Key, val: i128) -> slog::Result {
            impl_m!(self, key, &val)
        }
    }
    fn emit_str(&mut self, key: Key, val: &str) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_arguments(
        &mut self,
        key: Key,
        val: &fmt::Arguments,
    ) -> slog::Result {
        TL_BUF.with(|buf| {
            let mut buf = buf.borrow_mut();

            buf.write_fmt(*val).unwrap();

            let res = { || impl_m!(self, key, &*buf) }();
            buf.clear();
            res
        })
    }

    fn emit_serde(
        &mut self,
        key: Key,
        value: &dyn slog::SerdeValue,
    ) -> slog::Result {
        impl_m!(self, key, value.as_serde())
    }
}
// }}}

// {{{ Json
/// Json `Drain`
///
/// Each record will be printed as a Json map
/// to a given `io`
pub struct Json<W: io::Write> {
    flush: bool,
    values: Vec<OwnedKVList>,
    io: RefCell<W>,
}

impl<W> Json<W>
where
    W: io::Write,
{
    /// Build custom `Json` `Drain`
    #[cfg_attr(feature = "cargo-clippy", allow(clippy::new_ret_no_self))]
    pub fn new(io: W) -> JsonBuilder<W> {
        JsonBuilder::new(io)
    }

    fn log_impl<F>(
        &self,
        serializer: &mut serde_json::ser::Serializer<&mut W, F>,
        rinfo: &Record,
        logger_values: &OwnedKVList,
    ) -> io::Result<()>
    where
        F: serde_json::ser::Formatter,
    {
        let mut serializer = SerdeSerializer::start(&mut *serializer, None)?;

        for kv in &self.values {
            kv.serialize(rinfo, &mut serializer)?;
        }

        logger_values.serialize(rinfo, &mut serializer)?;

        rinfo.kv().serialize(rinfo, &mut serializer)?;

        let res = serializer.end();

        res.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        Ok(())
    }
}

impl<W> slog::Drain for Json<W>
where
    W: io::Write,
{
    type Ok = ();
    type Err = io::Error;
    fn log(
        &self,
        rinfo: &Record,
        logger_values: &OwnedKVList,
    ) -> io::Result<()> {
        let mut io = self.io.borrow_mut();
        let io = {
            let mut serializer = serde_json::Serializer::new(&mut *io);
            self.log_impl(&mut serializer, rinfo, logger_values)?;
            serializer.into_inner()
        };
        // We always want to write a newline in Arcadia
        io.write_all("\n".as_bytes())?;
        if self.flush {
            io.flush()?;
        }
        Ok(())
    }
}

/// Json `Drain` builder
///
/// Create with `Json::new`.
pub struct JsonBuilder<W: io::Write> {
    flush: bool,
    values: Vec<OwnedKVList>,
    io: W,
}

impl<W> JsonBuilder<W>
where
    W: io::Write,
{
    fn new(io: W) -> Self {
        JsonBuilder {
            flush: false,
            values: vec![],
            io,
        }
    }

    /// Build `Json` `Drain`
    ///
    /// This consumes the builder.
    pub fn build(self) -> Json<W> {
        Json {
            values: self.values,
            flush: self.flush,
            io: RefCell::new(self.io),
        }
    }
    
    /// Enable flushing of the `io::Write` after every log record
    pub fn set_flush(mut self, enabled: bool) -> Self {
        self.flush = enabled;
        self
    }
    
    /// Add custom values to be printed with this formatter
    pub fn add_key_value<T>(mut self, value: slog::OwnedKV<T>) -> Self
    where
        T: SendSyncRefUnwindSafeKV + 'static,
    {
        self.values.push(value.into());
        self
    }

    /// Add default key-values:
    ///
    /// * `ts` - timestamp
    /// * `level` - record logging level name
    /// * `msg` - msg - formatted logging message
    /// * `module` - module path (this is a special function of this fork)
    pub fn add_default_keys(self) -> Self {
        self.add_key_value(o!(
            "ts" => FnValue(move |_ : &Record| {
                    time::OffsetDateTime::now_utc()
                    .format(&time::format_description::well_known::Rfc3339)
                    .ok()
            }),
            "level" => FnValue(move |rinfo : &Record| {
                rinfo.level().as_short_str()
            }),
            "module" => FnValue(move |rinfo : &Record| {
                rinfo.tag().to_string()
            }),
            "msg" => PushFnValue(move |record : &Record, ser| {
                ser.emit(record.msg())
            }),
        ))
    }
}
