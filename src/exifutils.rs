use exif::{Context, Error, Field, In, Rational, SRational, Tag, Value};
use std::collections::HashMap;
use std::mem;

// Copied and adapted from https://github.com/kamadak/exif-rs/
// Licensed under the BSD 2-Clause "Simplified" License.

//
// Copyright (c) 2016 KAMADA Ken'ichi.
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions
// are met:
// 1. Redistributions of source code must retain the above copyright
//    notice, this list of conditions and the following disclaimer.
// 2. Redistributions in binary form must reproduce the above copyright
//    notice, this list of conditions and the following disclaimer in the
//    documentation and/or other materials provided with the distribution.
//
// THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
// ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
// ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
// OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
// HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
// LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
// OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
// SUCH DAMAGE.
//

trait CastSigned<T> {
    fn cast_signed_non_nightly(self) -> T;
}
impl CastSigned<i8> for u8 {
    fn cast_signed_non_nightly(self) -> i8 {
        self as i8
    }
}
impl CastSigned<i16> for u16 {
    fn cast_signed_non_nightly(self) -> i16 {
        self as i16
    }
}
impl CastSigned<i32> for u32 {
    fn cast_signed_non_nightly(self) -> i32 {
        self as i32
    }
}

type Parser = fn(&[u8], usize, usize) -> Value;

// Return the length of a single value and the parser of the type.
pub fn get_type_info<E>(typecode: u16) -> (usize, Parser)
where
    E: Endian,
{
    match typecode {
        1 => (1, parse_byte),
        2 => (1, parse_ascii),
        3 => (2, parse_short::<E>),
        4 => (4, parse_long::<E>),
        5 => (8, parse_rational::<E>),
        6 => (1, parse_sbyte),
        7 => (1, parse_undefined),
        8 => (2, parse_sshort::<E>),
        9 => (4, parse_slong::<E>),
        10 => (8, parse_srational::<E>),
        11 => (4, parse_float::<E>),
        12 => (8, parse_double::<E>),
        _ => (0, parse_unknown),
    }
}

fn parse_byte(data: &[u8], offset: usize, count: usize) -> Value {
    Value::Byte(data[offset..offset + count].to_vec())
}

fn parse_ascii(data: &[u8], offset: usize, count: usize) -> Value {
    // Any ASCII field can contain multiple strings [TIFF6 Image File
    // Directory].
    let iter = data[offset..offset + count].split(|&b| b == b'\0');
    let mut v: Vec<Vec<u8>> = iter.map(<[u8]>::to_vec).collect();
    if v.last().is_some_and(Vec::is_empty) {
        v.pop();
    }
    Value::Ascii(v)
}

fn parse_short<E>(data: &[u8], offset: usize, count: usize) -> Value
where
    E: Endian,
{
    let mut val = Vec::with_capacity(count);
    for i in 0..count {
        val.push(E::loadu16(data, offset + i * 2));
    }
    Value::Short(val)
}

fn parse_long<E>(data: &[u8], offset: usize, count: usize) -> Value
where
    E: Endian,
{
    let mut val = Vec::with_capacity(count);
    for i in 0..count {
        val.push(E::loadu32(data, offset + i * 4));
    }
    Value::Long(val)
}

fn parse_rational<E>(data: &[u8], offset: usize, count: usize) -> Value
where
    E: Endian,
{
    let mut val = Vec::with_capacity(count);
    for i in 0..count {
        val.push(Rational {
            num: E::loadu32(data, offset + i * 8),
            denom: E::loadu32(data, offset + i * 8 + 4),
        });
    }
    Value::Rational(val)
}

fn parse_sbyte(data: &[u8], offset: usize, count: usize) -> Value {
    let bytes = data[offset..offset + count]
        .iter()
        .map(|x| x.cast_signed_non_nightly())
        .collect();
    Value::SByte(bytes)
}

fn parse_undefined(data: &[u8], offset: usize, count: usize) -> Value {
    Value::Undefined(
        data[offset..offset + count].to_vec(),
        u32::try_from(offset).unwrap_or(u32::MAX),
    )
}

fn parse_sshort<E>(data: &[u8], offset: usize, count: usize) -> Value
where
    E: Endian,
{
    let mut val = Vec::with_capacity(count);
    for i in 0..count {
        val.push(E::loadu16(data, offset + i * 2).cast_signed_non_nightly());
    }
    Value::SShort(val)
}

fn parse_slong<E>(data: &[u8], offset: usize, count: usize) -> Value
where
    E: Endian,
{
    let mut val = Vec::with_capacity(count);
    for i in 0..count {
        val.push(E::loadu32(data, offset + i * 4).cast_signed_non_nightly());
    }
    Value::SLong(val)
}

fn parse_srational<E>(data: &[u8], offset: usize, count: usize) -> Value
where
    E: Endian,
{
    let mut val = Vec::with_capacity(count);
    for i in 0..count {
        val.push(SRational {
            num: E::loadu32(data, offset + i * 8).cast_signed_non_nightly(),
            denom: E::loadu32(data, offset + i * 8 + 4).cast_signed_non_nightly(),
        });
    }
    Value::SRational(val)
}

// TIFF and Rust use IEEE 754 format, so no conversion is required.
fn parse_float<E>(data: &[u8], offset: usize, count: usize) -> Value
where
    E: Endian,
{
    let mut val = Vec::with_capacity(count);
    for i in 0..count {
        val.push(f32::from_bits(E::loadu32(data, offset + i * 4)));
    }
    Value::Float(val)
}

// TIFF and Rust use IEEE 754 format, so no conversion is required.
fn parse_double<E>(data: &[u8], offset: usize, count: usize) -> Value
where
    E: Endian,
{
    let mut val = Vec::with_capacity(count);
    for i in 0..count {
        val.push(f64::from_bits(E::loadu64(data, offset + i * 8)));
    }
    Value::Double(val)
}

// This is a dummy function and will never be called.
#[allow(unused_variables)]
fn parse_unknown(data: &[u8], offset: usize, count: usize) -> Value {
    unreachable!()
}

pub trait Endian {
    fn loadu16(buf: &[u8], from: usize) -> u16;
    fn loadu32(buf: &[u8], from: usize) -> u32;
    fn loadu64(buf: &[u8], from: usize) -> u64;
}

pub struct LittleEndian;

macro_rules! generate_load {
    ($name:ident, $int_type:ident, $from_func:ident) => {
        fn $name(buf: &[u8], offset: usize) -> $int_type {
            let mut num = [0u8; mem::size_of::<$int_type>()];
            num.copy_from_slice(&buf[offset..offset + mem::size_of::<$int_type>()]);
            $int_type::$from_func(num)
        }
    };
}

impl Endian for LittleEndian {
    generate_load!(loadu16, u16, from_le_bytes);
    generate_load!(loadu32, u32, from_le_bytes);
    generate_load!(loadu64, u64, from_le_bytes);
}

// Parse IFD [EXIF23 4.6.2].
pub(crate) fn parse_ifd<E>(
    data: &[u8],
    mut offset: usize,
    ctx: Context,
    ifd_num: u16,
) -> Result<HashMap<u16, Field>, Error>
where
    E: Endian,
{
    let mut entries: Vec<Field> = Vec::new();

    // Count (the number of the entries).
    if data.len() < offset || data.len() - offset < 2 {
        return Err(Error::InvalidFormat("Truncated IFD count"));
    }
    let count = LittleEndian::loadu16(data, offset) as usize;
    offset += 2;

    // Array of entries.
    for _ in 0..count {
        if data.len() - offset < 12 {
            return Err(Error::InvalidFormat("Truncated IFD"));
        }
        let entry = parse_ifd_entry::<E>(data, offset);
        offset += 12;
        let (tag, val) = match entry {
            Ok(x) => x,
            Err(_e) => {
                //return Err(e)
                continue;
            }
        };

        // No infinite recursion will occur because the context is not
        // recursively defined.
        let tag = Tag(ctx, tag);
        let _child_ctx = match tag {
            Tag::ExifIFDPointer => Context::Exif,
            Tag::GPSInfoIFDPointer => Context::Gps,
            Tag::InteropIFDPointer => Context::Interop,
            _ => {
                entries.push(Field {
                    tag,
                    ifd_num: In(ifd_num),
                    value: val,
                });
                continue;
            }
        };
        //parse_child_ifd::<E>(data, val, child_ctx, ifd_num)?
    }

    Ok(entries
        .into_iter()
        .map(|e| (e.tag.1, e))
        .collect::<HashMap<_, _>>())
}

fn parse_ifd_entry<E>(data: &[u8], offset: usize) -> Result<(u16, Value), Error>
where
    E: Endian,
{
    // The size of entry has been checked in parse_ifd().
    let tag = E::loadu16(data, offset);
    let typ = E::loadu16(data, offset + 2);
    let cnt = E::loadu32(data, offset + 4);
    let valofs_at = offset + 8;
    let (unitlen, parser) = get_type_info::<E>(typ);
    let vallen = unitlen
        .checked_mul(cnt as usize)
        .ok_or(Error::InvalidFormat("Invalid entry count"))?;
    let val = if vallen <= 4 {
        parser(data, valofs_at, cnt as usize)
    } else {
        let ofs = E::loadu32(data, valofs_at) as usize;
        if data.len() < ofs || data.len() - ofs < vallen {
            return Err(Error::InvalidFormat("Truncated field value"));
        }
        Value::Unknown(typ, cnt, u32::try_from(ofs).unwrap_or(u32::MAX))
    };
    Ok((tag, val))
}
