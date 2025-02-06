use crate::deserialize::{FromBytes, U8Array};
use crate::deserialize_in_order;
use crate::error::XPTError;

#[derive(Debug)]
pub struct DocumentBase {
    pub sas: U8Array<8>,
    pub dataset_name: U8Array<8>,
    pub header_type: U8Array<8>,
    pub version: U8Array<8>,
    pub operation_system: U8Array<8>,
    pub time: U8Array<16>,
}

pub trait XptHeader: Sized {
    fn new(vec: &[u8; 80]) -> Result<Self, XPTError> {
        let string = String::from_utf8(vec.to_vec())?;
        let title = &string[20..48];
        Self::from_raw(
            match title.find("!") {
                Some(idx) => &title[..idx],
                None => title,
            },
            &string[48..],
        )
    }

    fn from_raw(header: &str, body: &str) -> Result<Self, XPTError>;
}

#[derive(Debug)]
pub enum DocumentHeader {
    V5,
    V8,
}

impl XptHeader for DocumentHeader {
    fn from_raw(header: &str, body: &str) -> Result<Self, XPTError> {
        if body != "000000000000000000000000000000  " {
            return Err(XPTError::ParseError(format!("unknown body {}", body)));
        }
        match header {
            "LIBRARY HEADER RECORD" => Ok(DocumentHeader::V5),
            "LIBV8   HEADER RECORD" => Ok(DocumentHeader::V8),
            _ => Err(XPTError::ParseError(format!(
                "unknown type detected {}",
                header
            ))),
        }
    }
}

#[derive(Debug)]
pub struct V5MemberTitleHeader(pub u16);

impl XptHeader for V5MemberTitleHeader {
    fn from_raw(header: &str, body: &str) -> Result<Self, XPTError> {
        if header != "MEMBER  HEADER RECORD" {
            return Err(XPTError::ParseError(format!(
                "unknown member header {}",
                header
            )));
        }
        if !body.starts_with("000000000000000001600000000") {
            return Err(XPTError::ParseError(format!(
                "unknown member body {}",
                body
            )));
        }
        Ok(V5MemberTitleHeader(
            u16::from_str_radix(&body[26..].trim(), 10).unwrap(),
        ))
    }
}
#[derive(Debug)]
pub struct V8MemberTitleHeader(pub u16);

impl XptHeader for V8MemberTitleHeader {
    fn from_raw(header: &str, body: &str) -> Result<Self, XPTError> {
        if header != "MEMBV8  HEADER RECORD" {
            return Err(XPTError::ParseError(format!(
                "unknown member header {}",
                header
            )));
        }
        if !body.starts_with("000000000000000001600000000") {
            return Err(XPTError::ParseError(format!(
                "unknown member body {}",
                body
            )));
        }
        Ok(V8MemberTitleHeader(
            u16::from_str_radix(&body[26..].trim(), 10).unwrap(),
        ))
    }
}

#[derive(Debug)]
pub struct V5NameStrTitleHeader(pub u16);

impl XptHeader for V5NameStrTitleHeader {
    fn from_raw(header: &str, body: &str) -> Result<Self, XPTError> {
        if header != "NAMESTR HEADER RECORD" {
            return Err(XPTError::ParseError(format!(
                "unknown NAMESTR header {}",
                header
            )));
        }
        Ok(V5NameStrTitleHeader(
            u16::from_str_radix(&body[6..10].trim(), 10).unwrap(),
        ))
    }
}

#[derive(Debug)]
pub struct V8NameStrTitleHeader(pub u16);

impl XptHeader for V8NameStrTitleHeader {
    fn from_raw(header: &str, body: &str) -> Result<Self, XPTError> {
        if header != "NAMSTV8 HEADER RECORD" {
            return Err(XPTError::ParseError(format!(
                "unknown NAMESTR header {}",
                header
            )));
        }
        Ok(V8NameStrTitleHeader(
            u16::from_str_radix(&body[6..10].trim(), 10).unwrap(),
        ))
    }
}
#[derive(Debug)]
pub struct V8ObsHeaderRecord(pub u64);

impl XptHeader for V8ObsHeaderRecord {
    fn from_raw(header: &str, body: &str) -> Result<Self, XPTError> {
        if header != "OBSV8   HEADER RECORD" {
            return Err(XPTError::ParseError(format!(
                "unknown NAMESTR header {}",
                header
            )));
        }
        Ok(V8ObsHeaderRecord(
            u64::from_str_radix(&body[..].trim(), 10).unwrap(),
        ))
    }
}

#[derive(Debug)]
pub struct V8LabelStrTitleHeader(pub u16);

impl XptHeader for V8LabelStrTitleHeader {
    fn from_raw(header: &str, body: &str) -> Result<Self, XPTError> {
        if header != "LABELV8 HEADER RECORD" {
            return Err(XPTError::ParseError(format!(
                "unknown NAMESTR header {}",
                header
            )));
        }
        Ok(V8LabelStrTitleHeader(
            u16::from_str_radix(&body[..].trim(), 10).unwrap(),
        ))
    }
}
#[derive(Debug)]
pub struct V5NameSt {
    pub ntype: u16,
    pub nhfun: u16,
    pub nlng: u16,
    pub nvar0: u16,
    pub nname: U8Array<8>,
    pub nlabel: U8Array<40>,
    pub nform: U8Array<8>,
    pub nfl: u16,
    pub nfd: u16,
    pub nfj: u16,
    pub nfill: [u8; 2],
    pub niform: U8Array<8>,
    pub nifl: u16,
    pub nifd: u16,
    pub npos: u32,
    pub rest: String,
}
#[derive(Debug)]

pub struct V8NameSt {
    pub ntype: u16,
    pub nhfun: u16,
    pub nlng: u16,
    pub nvar0: u16,
    pub nname: U8Array<8>,
    pub nlabel: U8Array<40>,
    pub nform: U8Array<8>,
    pub nfl: u16,
    pub nfd: u16,
    pub nfj: u16,
    pub nfill: [u8; 2],
    pub niform: U8Array<8>,
    pub nifl: u16,
    pub nifd: u16,
    pub npos: u32,
    pub rest: String,
    pub nlname: U8Array<32>,
    pub lablen: u16,
}

impl FromBytes for DocumentBase {
    fn from_bytes(input: &[u8]) -> Self {
        deserialize_in_order!(input, {
         sas:U8Array<8> with 8,
            dataset_name:U8Array<8> with 8,
            header_type:U8Array<8> with 8,
            version:U8Array<8> with 8,
            operation_system:U8Array<8> with 8,
            _ignore:U8Array<24> with 24,
            time:U8Array<16> with 16
        });
        DocumentBase {
            sas,
            dataset_name,
            header_type,
            version,
            operation_system,
            time,
        }
    }
}

#[derive(Debug)]
pub enum ColumnType {
    NUMERIC = 1,
    CHAR = 2,
}

#[derive(Debug)]
pub struct ColumnMeta {
    pub column_type: ColumnType,
    pub length: u16,
    pub var_count: u16,
    pub name: String,
    pub label: String,
    pub format: (String, u16, u16),
    pub in_format: (String, u16, u16),
}

pub type StringDecoder = fn(&[u8]) -> String;

impl ColumnMeta {
    pub fn from_v5(name_st: &V5NameSt, decode: StringDecoder) -> Self {
        Self {
            column_type: match name_st.ntype {
                1 => ColumnType::NUMERIC,
                2 => ColumnType::CHAR,
                _ => ColumnType::NUMERIC,
            },
            length: name_st.nlng,
            var_count: name_st.nvar0,
            name: decode(&name_st.nname.inner),
            label: decode(&name_st.nlabel.inner),
            format: (decode(&name_st.nform.inner), name_st.nfl, name_st.nfd),
            in_format: (decode(&name_st.niform.inner), name_st.nifl, name_st.nifd),
        }
    }

    pub fn from_v8(name_st: &V8NameSt, decode: StringDecoder) -> Self {
        Self {
            column_type: match name_st.ntype {
                1 => ColumnType::NUMERIC,
                2 => ColumnType::CHAR,
                _ => ColumnType::NUMERIC,
            },
            length: name_st.nlng,
            var_count: name_st.nvar0,
            name: decode(&name_st.nlname.inner),
            label: decode(&name_st.nlabel.inner),
            format: (decode(&name_st.nform.inner), name_st.nfl, name_st.nfd),
            in_format: (decode(&name_st.niform.inner), name_st.nifl, name_st.nifd),
        }
    }
}

#[derive(Debug)]
pub struct DocumentMeta {
    pub version: DocumentHeader,
    pub doc_version: String,
    pub operation_system: String,
    pub doc_update_time: String,
    pub dataset_name: String,
    pub lib_update_time: String,
    pub member_meta_length: u16,
    pub library: String,
    pub columns: Vec<ColumnMeta>,
}
