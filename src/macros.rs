#[macro_export]
macro_rules! deserialize_in_order {
    ($input:expr,{$($var:ident:$type:tt$(<$sub:tt>)? with $length:expr),+}) => {
        let mut _offset = 0;
        $(
                let $var:$type$(<$sub>)? = FromBytes::from_bytes(&$input[_offset.._offset+$length]);
                _offset+=$length;
        )*
    };
    ($input:expr, $([$var:ident;$type:ty;$length:expr]),*) => {
        let mut _offset = 0;
        $(
                // 通过类型推导获取变量类型
                let $var:$type = FromBytes::from_bytes(&$input[_offset.._offset+$length]);
                _offset+=$length;
        ),*
    };
}
