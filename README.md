# XPT File Reader (Async Rust)

A high-performance asynchronous SAS XPT file parser implemented in Rust, supporting both basic and multi-byte encodings.

## Features

- 🚀 Async-first implementation using Tokio
- 📦 Supports both standard UTF-8 and GBK encodings (via feature flags)
- 📈 Efficient streaming parser
- 📂 Metadata extraction (library info, column names, labels)
- 📝 Row-by-row data reading

## Installation

Add to your `Cargo.toml`:
```toml
[dependencies]
your_crate_name = { version = "0.1", features = ["async"] }
```

## Features Flags
- `async`: Enables async support (requires Tokio runtime)
- `multi_encoding`: Adds GBK encoding support

## Usage

### Basic Example
```rust
use your_crate_name::Reader;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = tokio::fs::File::open("sample.xpt").await?;
    
    // Create reader based on encoding feature
    #[cfg(not(feature = "multi_encoding"))]
    let mut reader = Reader::new(&mut file, |x| {
        String::from_utf8(x.to_vec()).unwrap().trim().to_string()
    });
    
    #[cfg(feature = "multi_encoding")]
    let mut reader = Reader::new_gbk(&mut file);

    // Read metadata
    let (mut data_handle, metadata) = reader.start().await?;
    println!("Library: {:?}", metadata.library);
    println!("Columns: {}", metadata.columns.iter().map(|c| &c.name).collect::<Vec<_>>().join("\t"));
    
    // Read data rows
    while let Some(row) = data_handle.read_line().await? {
        println!("{:?}", row);
    }
    
    Ok(())
}
```

### Configuration Options
The reader supports different initialization methods based on encoding needs:
```rust
// For standard UTF-8 processing
Reader::new(&mut file, |bytes| {
    String::from_utf8(bytes.to_vec()).unwrap().trim().to_string()
});

// For GBK encoding (requires multi_encoding feature)
Reader::new_gbk(&mut file);
```

## API Overview

### Metadata Structure
```rust
pub struct Metadata {
    pub library: String,
    pub columns: Vec<Column>,
}

pub struct Column {
    pub name: String,
    pub label: String,
    // ... other fields
}
```

### Data Reading
The `read_line()` method returns:
- `Some(Vec<Value>)` when data is available
- `None` when end of file is reached

## Performance Notes
- Uses zero-copy parsing where possible
- Current-thread Tokio runtime recommended for simple applications
- Enable `multi_encoding` feature only when GBK support is required

## Testing
Run tests with different feature combinations:
```bash
cargo test --features "async"
cargo test --features "async multi_encoding"
```

## License
MIT License
---

This README template:
1. Matches the async implementation shown in your code
2. Explains the dual encoding support through feature flags
3. Highlights the streaming nature of the parser
4. Shows both metadata and data reading aspects
5. Includes configuration options visible in the test code
6. Maintains Rust-idiomatic presentation

Would you like me to adjust any particular section or add more details about specific components?
