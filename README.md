# python_marshal

`python_marshal` is a Rust implementation of Python's `marshal` module. It provides functionality to read and write Python objects in a binary format. Additionally, it includes extensions for handling `.pyc` files directly.
NOTE: This library only supports Python 3.0 and later.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
python_marshal = "0.1.0"
```

## Usage

Check out the [documentation](https://docs.rs/python_marshal) for more information.
There are examples available in the `examples` directory.

Important to note: since Python allows for recursive references we have to store them separately.
This means that we represent references to objects using `Object::LoadRef(index)` and `Object::StoreRef(index)`. This is necessary to avoid infinite recursion when serializing and deserializing objects.

```rust
use python_marshal::load_bytes;

let data = b"r\x00\x00\x00\x00"; // Example byte data with a reference
let python_version = (3, 10);
let (object, references) = load_bytes(data, python_version).unwrap();
```
`references` is a hashmap that maps the index of the reference to the object it references.


## Contributing

Contributions are welcome! Please open an issue or submit a pull request on GitHub.

## License

This project is licensed under the MIT License.
