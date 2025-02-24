use crate::deserialize::{BufferFromByteArray, FromBytes};
use crate::deserialize_in_order;
use crate::error::XPTError;
use crate::part::{
    ColumnMeta, DocumentBase, DocumentHeader, DocumentMeta, StringDecoder, V5MemberTitleHeader,
    V5NameSt, V5NameStrTitleHeader, V8LabelStrTitleHeader, V8MemberTitleHeader, V8NameSt,
    V8NameStrTitleHeader, V8ObsHeaderRecord,
};
#[cfg(feature = "multi_encoding")]
use encoding::all::GBK;
#[cfg(feature = "multi_encoding")]
use encoding::{DecoderTrap, Encoding};
use std::collections::HashMap;
use std::fmt::Display;
#[cfg(not(feature = "async"))]
use {std::cell::RefCell, std::io::Read, std::rc::Rc};
#[cfg(feature = "async")]
use {std::sync::Arc, tokio::io::AsyncReadExt, tokio::sync::Mutex};

pub(crate) struct ReaderWrap<'a> {
    #[cfg(not(feature = "async"))]
    reader: &'a mut dyn Read,
    #[cfg(feature = "async")]
    reader: &'a mut (dyn tokio::io::AsyncRead + Unpin + Send),
}

impl<'a> ReaderWrap<'a> {
    #[cfg(not(feature = "async"))]
    pub fn new(reader: &'a mut dyn Read) -> Self {
        ReaderWrap { reader }
    }
    #[cfg(feature = "async")]
    pub fn new(reader: &'a mut (dyn tokio::io::AsyncRead + Unpin + Send)) -> Self {
        ReaderWrap { reader }
    }

    #[cfg(not(feature = "async"))]
    pub fn read<T: FromBytes>(&mut self, length: usize) -> T {
        let mut vec = vec![0; length];
        self.read2(vec.as_mut_slice())
    }

    #[cfg(not(feature = "async"))]
    pub fn read2<T: FromBytes>(&mut self, tmp: &mut [u8]) -> T {
        let _ = &self.reader.read_exact(tmp).unwrap();
        T::from_bytes(tmp)
    }

    #[cfg(not(feature = "async"))]
    pub fn read_exact(&mut self, tmp: &mut [u8]) -> std::io::Result<()> {
        self.reader.read_exact(tmp)
    }

    #[cfg(not(feature = "async"))]
    pub fn read2bytes(&mut self, tmp: &mut [u8]) -> std::io::Result<usize> {
        self.reader.read(tmp)
    }

    #[cfg(not(feature = "async"))]
    pub fn skip(&mut self, length: usize) {
        self.reader
            .read_exact(vec![0; length].as_mut_slice())
            .unwrap();
    }

    #[cfg(feature = "async")]
    pub async fn read<T: FromBytes>(&mut self, length: usize) -> T {
        let mut vec = vec![0; length];
        self.read2(vec.as_mut_slice()).await
    }

    #[cfg(feature = "async")]
    pub async fn read2<T: FromBytes>(&mut self, tmp: &mut [u8]) -> T {
        let _ = &self.reader.read_exact(tmp).await.unwrap();
        T::from_bytes(tmp)
    }

    #[cfg(feature = "async")]
    pub async fn read_exact(&mut self, tmp: &mut [u8]) -> std::io::Result<usize> {
        self.reader.read_exact(tmp).await
    }

    #[cfg(feature = "async")]
    pub async fn read2bytes(&mut self, tmp: &mut [u8]) -> std::io::Result<usize> {
        self.reader.read(tmp).await
    }

    #[cfg(feature = "async")]
    pub async fn skip(&mut self, length: usize) {
        self.reader
            .read_exact(vec![0; length].as_mut_slice())
            .await
            .unwrap();
    }
}

pub struct RawReader<'a> {
    #[cfg(not(feature = "async"))]
    reader: Rc<RefCell<ReaderWrap<'a>>>,
    #[cfg(feature = "async")]
    reader: Arc<Mutex<ReaderWrap<'a>>>,
    line_length: u32,
    line_str_array: Vec<u8>,
    line_number: usize,
    v5_name_sts: Vec<(u32, u16, u16)>,
    string_decoder: StringDecoder,
    observations: usize,
}

pub enum Val {
    Number(f64),
    Char(String),
    Nil,
}

fn byte2number(bytearray: &[u8]) -> Option<f64> {
    let mut array: Vec<u8> = vec![0; 8];
    for i in 0..bytearray.len() {
        array[i] = bytearray[i]
    }
    let val = i64::from_be_bytes(array[0..8].try_into().unwrap());
    let mut mantissa = val & 0x00ffffffffffffff;
    if mantissa == 0 {
        return if array[0] == 0x00 {
            Some(0f64)
        } else if array[0] == 0x8 {
            Some(-0f64)
        } else if array[0] == 46 {
            None
        } else {
            match array[0] {
                b'A' | b'B' | b'C' | b'D' | b'E' | b'F' | b'G' | b'H' | b'I' | b'J' | b'K'
                | b'L' | b'M' | b'N' | b'O' | b'P' | b'Q' | b'R' | b'S' | b'T' | b'U' | b'V'
                | b'W' | b'X' | b'Y' | b'Z' | b'_' => None,
                _ => {
                    panic!("Zero Mantissa Value was not readable");
                }
            }
        };
    }

    let sign = val & (0x8000000000000000u64 as i64);
    let mut exponent = (val & 0x7f00000000000000) >> 56;
    let shift;

    if (val & 0x0080000000000000) > 0 {
        shift = 3;
    } else if (val & 0x0040000000000000) > 0 {
        shift = 2;
    } else if (val & 0x0020000000000000) > 0 {
        shift = 1;
    } else {
        shift = 0;
    }

    mantissa = mantissa >> shift;
    mantissa = mantissa & (0xffefffffffffffffu64 as i64);
    exponent -= 65;
    exponent <<= 2;
    exponent += shift + 1023;
    let ieee = sign | (exponent << 52) | mantissa;
    Some(f64::from_be_bytes(ieee.to_be_bytes()))
}

fn number2Byte(option: Option<f64>) -> [u8; 8] {
    // Python uses IEEE: sign * 1.mantissa * 2 ** (exponent - 1023)
    // IBM mainframe: sign * 0.mantissa * 16 ** (exponent - 64)

    if option.is_none() {
        return [0; 8];
    }
    let ieee = option.unwrap();

    if ieee.is_nan() || ieee == 0.0 {
        return [0; 8];
    }

    if ieee.is_infinite() {
        panic!("Cannot convert infinity");
    }

    let buffer = ieee.to_be_bytes();

    let ulong = i64::from_be_bytes(buffer);
    let mut sign = (ulong & (1 << 63)) >> 63; // 1-bit sign
    let mut exponent = ((ulong & (0x7ff << 52)) >> 52) - 1023; // 11-bits exponent
    let mut mantissa = ulong & 0x000fffffffffffff; // 52-bits mantissa

    if exponent > 248 {
        panic!("Cannot store magnitude more than ~ 16 ** 63 as IBM-format");
    }
    if exponent < -260 {
        panic!("Cannot store magnitude less than ~ 16 ** -65 as IBM-format");
    }

    mantissa = 0x0010000000000000 | mantissa;

    let quotient = exponent >> 2;
    let remainder = exponent - (quotient << 2);
    exponent = quotient;

    mantissa <<= remainder;
    exponent += 1;
    exponent = exponent + 64;

    sign = sign << 63;
    exponent = exponent << 56;

    // We lose some precision, but who said floats were perfect?
    let result = sign | exponent | mantissa;
    let mut result_bytes: [u8; 8] = [0; 8];

    for i in 0..8 {
        result_bytes[i] = ((result as u64) >> (8 * (7 - i))) as u8;
    }

    result_bytes
}

impl Display for Val {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Val::Number(number) => number.to_string(),
                Val::Char(vec) => vec.clone(),
                Val::Nil => String::new(),
            }
        )
    }
}
impl<'a> RawReader<'a> {
    #[cfg(not(feature = "async"))]
    pub fn read_line(&mut self) -> Result<Option<Vec<Val>>, XPTError> {
        let mut reader = self.reader.borrow_mut();
        let left = reader.read2bytes(&mut self.line_str_array)?;
        if self.observations != 0 && self.line_number > self.observations {
            return Ok(None);
        }
        if left < self.line_length.try_into().unwrap() {
            return Ok(None);
        }
        if self.v5_name_sts.is_empty() {
            return Ok(None);
        }
        let mut vec: Vec<Val> = Vec::with_capacity(self.v5_name_sts.len());
        for v5_name_st in &self.v5_name_sts {
            let start: usize = v5_name_st.0 as usize;
            let end: usize = start + (v5_name_st.1 as usize);
            vec.push(if v5_name_st.2 == 1 {
                if let Some(numer) = byte2number(&self.line_str_array[start..end]) {
                    Val::Number(numer)
                } else {
                    Val::Nil
                }
            } else {
                let decoder = self.string_decoder;
                Val::Char(decoder(&self.line_str_array[start..end])?)
            });
        }
        self.line_number += 1;
        Ok(Some(vec))
    }

    #[cfg(feature = "async")]
    pub async fn read_line(&mut self) -> Result<Option<Vec<Val>>, XPTError> {
        let mut reader = self.reader.lock().await;
        let left = reader.read2bytes(&mut self.line_str_array).await?;
        if self.observations != 0 && self.line_number > self.observations {
            return Ok(None);
        }
        if left < self.line_length.try_into().unwrap() {
            return Ok(None);
        }
        if self.v5_name_sts.is_empty() {
            return Ok(None);
        }
        let mut vec: Vec<Val> = Vec::with_capacity(self.v5_name_sts.len());
        for v5_name_st in &self.v5_name_sts {
            let start: usize = v5_name_st.0 as usize;
            let end: usize = start + (v5_name_st.1 as usize);
            vec.push(if v5_name_st.2 == 1 {
                if let Some(numer) = byte2number(&self.line_str_array[start..end]) {
                    Val::Number(numer)
                } else {
                    Val::Nil
                }
            } else {
                let decoder = self.string_decoder;
                Val::Char(decoder(&self.line_str_array[start..end])?)
            });
        }
        self.line_number += 1;
        Ok(Some(vec))
    }
}

pub struct Reader<'a> {
    #[cfg(not(feature = "async"))]
    reader: Rc<RefCell<ReaderWrap<'a>>>,
    #[cfg(feature = "async")]
    reader: Arc<Mutex<ReaderWrap<'a>>>,
    string_decoder: StringDecoder,
}

#[cfg(feature = "multi_encoding")]
pub const GBK_STRING_DECODER: StringDecoder = |x| {
    GBK.decode(x, DecoderTrap::Ignore)
        .map_err(|x| XPTError::DecodeError(x.to_string()))
        .map(|x| x.trim().to_string())
};

impl<'a> Reader<'a> {
    #[cfg(not(feature = "async"))]
    pub fn new(reader: &'a mut dyn Read, string_decoder: StringDecoder) -> Self {
        Reader {
            reader: Rc::new(RefCell::new(ReaderWrap::new(reader))),
            string_decoder,
        }
    }

    #[cfg(feature = "async")]
    pub fn new(
        reader: &'a mut (dyn tokio::io::AsyncRead + Unpin + Send),
        string_decoder: StringDecoder,
    ) -> Self {
        Reader {
            reader: Arc::new(Mutex::new(ReaderWrap::new(reader))),
            string_decoder,
        }
    }

    #[cfg(feature = "multi_encoding")]
    #[cfg(not(feature = "async"))]
    pub fn new_gbk(reader: &'a mut dyn Read) -> Self {
        Self::new(reader, GBK_STRING_DECODER)
    }

    #[cfg(feature = "multi_encoding")]
    #[cfg(feature = "async")]
    pub fn new_gbk(reader: &'a mut (dyn tokio::io::AsyncRead + Unpin + Send)) -> Self {
        Self::new(reader, GBK_STRING_DECODER)
    }

    #[cfg(not(feature = "async"))]
    pub fn start(&mut self) -> Result<(RawReader<'a>, DocumentMeta), XPTError> {
        let mut u80 = [0; 80];
        #[cfg(not(feature = "async"))]
        let mut reader = self.reader.borrow_mut();
        #[cfg(feature = "async")]
        let mut reader = self.reader.lock().unwrap();
        let document_header = reader
            .read2::<BufferFromByteArray<DocumentHeader>>(&mut u80)
            .0;
        let document_base: DocumentBase = reader.read2(&mut u80);
        let update_date: String = reader.read2(&mut u80);
        let member_title_header = match document_header {
            DocumentHeader::V5 => {
                reader
                    .read2::<BufferFromByteArray<V5MemberTitleHeader>>(&mut u80)
                    .0
                     .0
            }
            DocumentHeader::V8 => {
                reader
                    .read2::<BufferFromByteArray<V8MemberTitleHeader>>(&mut u80)
                    .0
                     .0
            }
        };
        reader.read2::<String>(&mut u80);
        let library_base = reader.read2::<DocumentBase>(&mut u80);
        let library_update_date: String = reader.read2(&mut u80);
        let str_title_header = match document_header {
            DocumentHeader::V5 => {
                reader
                    .read2::<BufferFromByteArray<V5NameStrTitleHeader>>(&mut u80)
                    .0
                     .0
            }
            DocumentHeader::V8 => {
                reader
                    .read2::<BufferFromByteArray<V8NameStrTitleHeader>>(&mut u80)
                    .0
                     .0
            }
        };
        let mut left_blank = member_title_header * str_title_header % 80;
        let mut name_str_array = vec![0; member_title_header.into()];
        let mut line_length = 0;
        let mut column_meta_array: Vec<ColumnMeta> = Vec::with_capacity(str_title_header.into());
        let mut v5_name_sts: Vec<(u32, u16, u16)> = Vec::with_capacity(str_title_header.into());
        let decoder = self.string_decoder;
        let mut observations: usize = 0;
        match document_header {
            DocumentHeader::V5 => {
                for i in 0..str_title_header {
                    let name_st: V5NameSt = reader.read2(&mut name_str_array);
                    line_length = name_st.npos + ((&name_st).nlng as u32);
                    column_meta_array.insert(
                        i.into(),
                        ColumnMeta::from_v5(&name_st, self.string_decoder)?,
                    );
                    v5_name_sts.insert(i.into(), (name_st.npos, name_st.nlng, name_st.ntype));
                }
                if left_blank > 0 {
                    reader.skip((80 - left_blank) as usize);
                }
                let _obs_header = reader.read2::<String>(&mut u80);
            }
            DocumentHeader::V8 => {
                let mut with_long_label = false;
                for i in 0..str_title_header {
                    let name_st: V8NameSt = reader.read2(&mut name_str_array);
                    line_length = name_st.npos + ((&name_st).nlng as u32);
                    column_meta_array.insert(
                        i.into(),
                        ColumnMeta::from_v8(&name_st, self.string_decoder)?,
                    );
                    if name_st.lablen > 40 {
                        with_long_label = true;
                    }
                    v5_name_sts.insert(i.into(), (name_st.npos, name_st.nlng, name_st.ntype));
                }
                if left_blank > 0 {
                    reader.skip((80 - left_blank) as usize);
                }
                //should be handle in v8
                if with_long_label {
                    let title_header_count = reader
                        .read2::<BufferFromByteArray<V8LabelStrTitleHeader>>(&mut u80)
                        .0
                         .0;
                    left_blank = 0;
                    let mut len_def = [0; 6];
                    let mut var_map: HashMap<u16, ColumnMeta> = column_meta_array
                        .into_iter()
                        .map(|f| (f.var_count, f))
                        .collect();
                    for _i in 0..title_header_count {
                        reader.read_exact(&mut len_def)?;
                        deserialize_in_order!(
                            len_def,{
                                var_number :u16 with 2,
                                name_len:u16 with 2,
                                label_len:u16 with 2
                            }
                        );
                        let option = var_map.get_mut(&var_number).unwrap();
                        let mut vec2 = vec![0; name_len as usize];
                        reader.read_exact(vec2.as_mut_slice())?;
                        option.name = decoder(vec2.as_slice())?;
                        vec2 = vec![0; label_len as usize];
                        reader.read_exact(vec2.as_mut_slice())?;
                        option.label = decoder(vec2.as_slice())?;
                        left_blank = (left_blank + 6 + name_len + label_len) % 80;
                    }
                    if left_blank > 0 {
                        reader.skip((80 - left_blank) as usize);
                    }
                    column_meta_array = var_map.into_values().collect();
                    column_meta_array.sort_by(|a, b| a.var_count.cmp(&b.var_count));
                }
                observations = reader
                    .read2::<BufferFromByteArray<V8ObsHeaderRecord>>(&mut u80)
                    .0
                     .0 as usize;
            }
        }
        Ok((
            RawReader {
                reader: self.reader.clone(),
                line_length,
                line_str_array: vec![0; line_length as usize],
                line_number: 0,
                v5_name_sts,
                string_decoder: self.string_decoder,
                observations,
            },
            DocumentMeta {
                version: document_header,
                doc_version: decoder(&document_base.version.inner)?,
                operation_system: decoder(&document_base.operation_system.inner)?,
                doc_update_time: update_date,
                dataset_name: decoder(&library_base.version.inner)?,
                lib_update_time: library_update_date,
                member_meta_length: member_title_header,
                library: decoder(&library_base.dataset_name.inner)?,
                columns: column_meta_array,
            },
        ))
    }

    #[cfg(feature = "async")]
    pub async fn start(&mut self) -> Result<(RawReader<'a>, DocumentMeta), XPTError> {
        let mut u80 = [0; 80];
        #[cfg(not(feature = "async"))]
        let mut reader = self.reader.borrow_mut();
        #[cfg(feature = "async")]
        let mut reader = self.reader.lock().await;
        let document_header = reader
            .read2::<BufferFromByteArray<DocumentHeader>>(&mut u80)
            .await
            .0;
        let document_base: DocumentBase = reader.read2(&mut u80).await;
        let update_date: String = reader.read2(&mut u80).await;
        let member_title_header = match document_header {
            DocumentHeader::V5 => {
                reader
                    .read2::<BufferFromByteArray<V5MemberTitleHeader>>(&mut u80)
                    .await
                    .0
                     .0
            }
            DocumentHeader::V8 => {
                reader
                    .read2::<BufferFromByteArray<V8MemberTitleHeader>>(&mut u80)
                    .await
                    .0
                     .0
            }
        };
        reader.read2::<String>(&mut u80).await;
        let library_base = reader.read2::<DocumentBase>(&mut u80).await;
        let library_update_date: String = reader.read2(&mut u80).await;
        let str_title_header = match document_header {
            DocumentHeader::V5 => {
                reader
                    .read2::<BufferFromByteArray<V5NameStrTitleHeader>>(&mut u80)
                    .await
                    .0
                     .0
            }
            DocumentHeader::V8 => {
                reader
                    .read2::<BufferFromByteArray<V8NameStrTitleHeader>>(&mut u80)
                    .await
                    .0
                     .0
            }
        };
        let mut left_blank = member_title_header * str_title_header % 80;
        let mut name_str_array = vec![0; member_title_header.into()];
        let mut line_length = 0;
        let mut column_meta_array: Vec<ColumnMeta> = Vec::with_capacity(str_title_header.into());
        let mut v5_name_sts: Vec<(u32, u16, u16)> = Vec::with_capacity(str_title_header.into());
        let decoder = self.string_decoder;
        let mut observations: usize = 0;
        match document_header {
            DocumentHeader::V5 => {
                for i in 0..str_title_header {
                    let name_st: V5NameSt = reader.read2(&mut name_str_array).await;
                    line_length = name_st.npos + ((&name_st).nlng as u32);
                    column_meta_array.insert(
                        i.into(),
                        ColumnMeta::from_v5(&name_st, self.string_decoder)?,
                    );
                    v5_name_sts.insert(i.into(), (name_st.npos, name_st.nlng, name_st.ntype));
                }
                if left_blank > 0 {
                    reader.skip((80 - left_blank) as usize).await;
                }
                let _obs_header = reader.read2::<String>(&mut u80).await;
            }
            DocumentHeader::V8 => {
                let mut with_long_label = false;
                for i in 0..str_title_header {
                    let name_st: V8NameSt = reader.read2(&mut name_str_array).await;
                    line_length = name_st.npos + ((&name_st).nlng as u32);
                    column_meta_array.insert(
                        i.into(),
                        ColumnMeta::from_v8(&name_st, self.string_decoder)?,
                    );
                    if name_st.lablen > 40 {
                        with_long_label = true;
                    }
                    v5_name_sts.insert(i.into(), (name_st.npos, name_st.nlng, name_st.ntype));
                }
                if left_blank > 0 {
                    reader.skip((80 - left_blank) as usize).await;
                }
                //should be handle in v8
                if with_long_label {
                    let title_header_count = reader
                        .read2::<BufferFromByteArray<V8LabelStrTitleHeader>>(&mut u80)
                        .await
                        .0
                         .0;
                    left_blank = 0;
                    let mut len_def = [0; 6];
                    let mut var_map: HashMap<u16, ColumnMeta> = column_meta_array
                        .into_iter()
                        .map(|f| (f.var_count, f))
                        .collect();
                    for _i in 0..title_header_count {
                        reader.read_exact(&mut len_def).await?;
                        deserialize_in_order!(
                            len_def,{
                                var_number :u16 with 2,
                                name_len:u16 with 2,
                                label_len:u16 with 2
                            }
                        );
                        let option = var_map.get_mut(&var_number).unwrap();
                        let mut vec2 = vec![0; name_len as usize];
                        reader.read_exact(vec2.as_mut_slice()).await?;
                        option.name = decoder(vec2.as_slice())?;
                        vec2 = vec![0; label_len as usize];
                        reader.read_exact(vec2.as_mut_slice()).await?;
                        option.label = decoder(vec2.as_slice())?;
                        left_blank = (left_blank + 6 + name_len + label_len) % 80;
                    }
                    if left_blank > 0 {
                        reader.skip((80 - left_blank) as usize).await;
                    }
                    column_meta_array = var_map.into_values().collect();
                    column_meta_array.sort_by(|a, b| a.var_count.cmp(&b.var_count));
                }
                observations = reader
                    .read2::<BufferFromByteArray<V8ObsHeaderRecord>>(&mut u80)
                    .await
                    .0
                     .0 as usize;
            }
        }
        Ok((
            RawReader {
                reader: self.reader.clone(),
                line_length,
                line_str_array: vec![0; line_length as usize],
                line_number: 0,
                v5_name_sts,
                string_decoder: self.string_decoder,
                observations,
            },
            DocumentMeta {
                version: document_header,
                doc_version: decoder(&document_base.version.inner)?,
                operation_system: decoder(&document_base.operation_system.inner)?,
                doc_update_time: update_date,
                dataset_name: decoder(&library_base.version.inner)?,
                lib_update_time: library_update_date,
                member_meta_length: member_title_header,
                library: decoder(&library_base.dataset_name.inner)?,
                columns: column_meta_array,
            },
        ))
    }
}

#[cfg(test)]
mod test {
    use crate::error::XPTError;
    use crate::reader::{byte2number, number2Byte, Reader};
    #[cfg(not(feature = "async"))]
    use std::fs::File;

    #[test]
    #[cfg(feature = "async")]
    fn test_v5_reader() {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let mut file = tokio::fs::File::open("sample/LB2.xpt").await.unwrap();
                #[cfg(not(feature = "multi_encoding"))]
                let mut reader = Reader::new(&mut file, |x| {
                    String::from_utf8(x.to_vec())
                        .map_err(|x| XPTError::DecodeError(x.to_string()))
                        .map(|x| x.trim().to_string())
                });
                #[cfg(feature = "multi_encoding")]
                let mut reader = Reader::new_gbk(&mut file);
                let result = reader.start().await.unwrap();
                println!("{:?}", result.1.library);
                println!(
                    "{}",
                    result
                        .1
                        .columns
                        .iter()
                        .map(|x| x.name.clone())
                        .collect::<Vec<String>>()
                        .join("\t")
                );
                println!(
                    "{}",
                    result
                        .1
                        .columns
                        .iter()
                        .map(|x| x.label.clone())
                        .collect::<Vec<String>>()
                        .join("\t")
                );
                let mut c = result.0;
                while let Some(line) = c.read_line().await.unwrap() {
                    println!(
                        "{}",
                        line.iter()
                            .map(|v| v.to_string())
                            .collect::<Vec<String>>()
                            .join("\t")
                    );
                }
            });
    }

    #[test]
    #[cfg(not(feature = "async"))]
    fn test_v5_reader() {
        let mut file = File::open("sample/LB2.xpt").unwrap();
        #[cfg(not(feature = "multi_encoding"))]
        let mut reader = Reader::new(&mut file, |x| {
            String::from_utf8(x.to_vec())
                .map_err(|x| XPTError::DecodeError(x.to_string()))
                .map(|x| x.trim().to_string())
        });
        #[cfg(feature = "multi_encoding")]
        let mut reader = Reader::new_gbk(&mut file);
        let result = reader.start().unwrap();
        println!("{:?}", result.1.library);
        println!(
            "{}",
            result
                .1
                .columns
                .iter()
                .map(|x| x.name.clone())
                .collect::<Vec<String>>()
                .join("\t")
        );
        println!(
            "{}",
            result
                .1
                .columns
                .iter()
                .map(|x| x.label.clone())
                .collect::<Vec<String>>()
                .join("\t")
        );
        let mut c = result.0;
        while let Some(line) = c.read_line().unwrap() {
            println!(
                "{}",
                line.iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<String>>()
                    .join("\t")
            );
        }
    }

    #[test]
    fn test_byte2number() {
        assert_eq!(
            byte2number(&[63, 245, 194, 143, 92, 0]).unwrap(),
            0.059999999997671694
        );
        assert_eq!(
            number2Byte(Some(0.059999999997671694)),
            [63, 245, 194, 143, 92, 0, 0, 0]
        );
    }
}
