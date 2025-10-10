use std::fmt::Display;
use serde::{ser, Serialize};

use crate::command::Command;
use crate::command::raw::RawCommand;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Message(String),
    #[error("internal error: {0}")]
    Internal(String),
    #[error("cannot serialize a top-level type of '{0}'")]
    InvalidTopLevelType(String),
    #[error("value type '{0}' not supported")]
    ValueTypeNotSupported(String),
}

impl Error {
    fn invalid_top_level(t: impl Into<String>) -> Self {
        Self::InvalidTopLevelType(t.into())
    }

    fn value_not_supported(t: impl Into<String>) -> Self {
        Self::ValueTypeNotSupported(t.into())
    }
}

impl ser::Error for Error {
    #[inline]
    fn custom<T: Display>(msg: T) -> Self {
        Self::Message(msg.to_string())
    }
}

struct CommandSerializer {
    cmd: RawCommand,
    key: Option<String>,
}

#[inline]
pub fn to_raw_command<C: Command + ser::Serialize>(cmd: &C) -> Result<RawCommand, Error> {
    cmd.serialize(CommandSerializer {
        cmd: RawCommand::new(C::GROUP, C::COMMAND),
        key: None,
    })
}

impl ser::Serializer for CommandSerializer {
    type Ok = RawCommand;
    type Error = Error;

    type SerializeSeq = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTuple = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = ser::Impossible<Self::Ok, Self::Error>;

    fn serialize_bool(self, _: bool) -> Result<Self::Ok, Self::Error> {
        Err(Error::invalid_top_level("bool"))
    }

    fn serialize_i8(self, _: i8) -> Result<Self::Ok, Self::Error> {
        Err(Error::invalid_top_level("i8"))
    }

    fn serialize_i16(self, _: i16) -> Result<Self::Ok, Self::Error> {
        Err(Error::invalid_top_level("i16"))
    }

    fn serialize_i32(self, _: i32) -> Result<Self::Ok, Self::Error> {
        Err(Error::invalid_top_level("i32"))
    }

    fn serialize_i64(self, _: i64) -> Result<Self::Ok, Self::Error> {
        Err(Error::invalid_top_level("i64"))
    }

    fn serialize_u8(self, _: u8) -> Result<Self::Ok, Self::Error> {
        Err(Error::invalid_top_level("u8"))
    }

    fn serialize_u16(self, _: u16) -> Result<Self::Ok, Self::Error> {
        Err(Error::invalid_top_level("u16"))
    }

    fn serialize_u32(self, _: u32) -> Result<Self::Ok, Self::Error> {
        Err(Error::invalid_top_level("u32"))
    }

    fn serialize_u64(self, _: u64) -> Result<Self::Ok, Self::Error> {
        Err(Error::invalid_top_level("u64"))
    }

    fn serialize_f32(self, _: f32) -> Result<Self::Ok, Self::Error> {
        Err(Error::invalid_top_level("f32"))
    }

    fn serialize_f64(self, _: f64) -> Result<Self::Ok, Self::Error> {
        Err(Error::invalid_top_level("f64"))
    }

    fn serialize_char(self, _: char) -> Result<Self::Ok, Self::Error> {
        Err(Error::invalid_top_level("char"))
    }

    fn serialize_str(self, _: &str) -> Result<Self::Ok, Self::Error> {
        Err(Error::invalid_top_level("str"))
    }

    fn serialize_bytes(self, _: &[u8]) -> Result<Self::Ok, Self::Error> {
        Err(Error::invalid_top_level("[u8]"))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(Error::invalid_top_level("None"))
    }

    fn serialize_some<T>(self, _: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize
    {
        Err(Error::invalid_top_level("Some(T)"))
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(Error::invalid_top_level("unit"))
    }

    fn serialize_unit_struct(self, _: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(Error::invalid_top_level("unit struct"))
    }

    fn serialize_unit_variant(self, _: &'static str, _: u32, _: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(Error::invalid_top_level("unit variant"))
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(self, _: &'static str, _: u32, _: &'static str, _: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize
    {
        Err(Error::invalid_top_level("newtype variant"))
    }

    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(Error::invalid_top_level("sequence"))
    }

    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(Error::invalid_top_level("tuple"))
    }

    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(Error::invalid_top_level("tuple struct"))
    }

    fn serialize_tuple_variant(self, _: &'static str, _: u32, _: &'static str, _: usize) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(Error::invalid_top_level("tuple variant"))
    }

    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(self)
    }

    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(self)
    }

    fn serialize_struct_variant(self, _: &'static str, _: u32, _: &'static str, _: usize) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(Error::invalid_top_level("struct variant"))
    }
}

impl ser::SerializeMap for CommandSerializer {
    type Ok = RawCommand;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let key = key.serialize(DisplaySerializer::default())?
            .ok_or(Error::Internal("key should not be 'None'".to_string()))?;
        self.key = Some(key);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let key = self.key.take()
            .ok_or(Error::Internal("key not previously set in map".to_string()))?;
        let value = value.serialize(DisplaySerializer::default())?;
        if let Some(value) = value {
            self.cmd.param(key, value);
        }
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.cmd)
    }
}

impl ser::SerializeStruct for CommandSerializer {
    type Ok = RawCommand;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let value = value.serialize(DisplaySerializer::default())?;
        if let Some(value) = value {
            self.cmd.param(key, value);
        }
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.cmd)
    }
}

#[derive(Default)]
struct DisplaySerializer {}

impl<'a> ser::Serializer for DisplaySerializer {
    type Ok = Option<String>;
    type Error = Error;
    type SerializeSeq = BufferedDisplaySerializer;
    type SerializeTuple = BufferedDisplaySerializer;
    type SerializeTupleStruct = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeMap = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStructVariant = ser::Impossible<Self::Ok, Self::Error>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        // HEOS commands usually expect on/off in place of true/false
        Ok(Some(match v {
            true => "on",
            false => "off",
        }.to_string()))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(Some(v.to_string()))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(Some(v.to_string()))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(Some(v.to_string()))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(Some(v.to_string()))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(Some(v.to_string()))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(Some(v.to_string()))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(Some(v.to_string()))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(Some(v.to_string()))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(Some(v.to_string()))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Some(v.to_string()))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(Some(v.to_string()))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Some(v.to_string()))
    }

    fn serialize_bytes(self, _: &[u8]) -> Result<Self::Ok, Self::Error> {
        Err(Error::value_not_supported("[u8]"))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(None)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize
    {
        value.serialize(Self::default())
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(Error::value_not_supported("unit"))
    }

    fn serialize_unit_struct(self, _: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(Error::value_not_supported("unit struct"))
    }

    fn serialize_unit_variant(self, _: &'static str, _: u32, _: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(Error::value_not_supported("unit variant"))
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize
    {
        value.serialize(Self::default())
    }

    fn serialize_newtype_variant<T>(self, _name: &'static str, _variant_index: u32, _variant: &'static str, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize
    {
        value.serialize(Self::default())
    }

    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(BufferedDisplaySerializer::default())
    }

    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(BufferedDisplaySerializer::default())
    }

    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(Error::value_not_supported("tuple struct"))
    }

    fn serialize_tuple_variant(self, _: &'static str, _: u32, _: &'static str, _: usize) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(Error::value_not_supported("tuple variant"))
    }

    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(Error::value_not_supported("map"))
    }

    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self::SerializeStruct, Self::Error> {
        Err(Error::value_not_supported("struct"))
    }

    fn serialize_struct_variant(self, _: &'static str, _: u32, _: &'static str, _: usize) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(Error::value_not_supported("struct variant"))
    }
}

#[derive(Default)]
pub struct BufferedDisplaySerializer {
    output: String,
}

impl ser::SerializeSeq for BufferedDisplaySerializer {
    type Ok = Option<String>;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let element = value.serialize(DisplaySerializer::default())?
            .ok_or(Error::Message("element should not be None".to_string()))?;
        if self.output.is_empty() {
            self.output += &element;
        } else {
            self.output += ",";
            self.output += &element;
        }
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Some(self.output))
    }
}

impl ser::SerializeTuple for BufferedDisplaySerializer {
    type Ok = Option<String>;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let element = value.serialize(DisplaySerializer::default())?
            .ok_or(Error::Message("element should not be None".to_string()))?;
        if self.output.is_empty() {
            self.output += &element;
        } else {
            self.output += ",";
            self.output += &element;
        }
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Some(self.output))
    }
}