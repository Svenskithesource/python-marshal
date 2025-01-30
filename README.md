# python_marshal

`python_marshal` is a Rust implementation of Python's `marshal` module. It provides functionality to read and write Python objects in a binary format. Additionally, it includes extensions for handling `.pyc` files directly.
NOTE: This library only supports Python 3.10, 3.11, 3.12 and 3.13.

## Installation
Use `cargo add python_marshal` to add this library to your project.
You can also manually add it to your `Cargo.toml` file:

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
let (object, references) = load_bytes(data, python_version.into()).unwrap();
```
`references` is a hashmap that maps the index of the reference to the object it references.

## Testing
This library is very thoroughly tested. To ensure it can output the exact same bytes as the input data, we rewrite the whole standard library and compare the output with the input. It produces a 1:1 copy of the input data.
You can run the tests with `cargo test` (integration tests only work on Windows).

## Contributing

Contributions are welcome! Please open an issue or submit a pull request on GitHub.

## License

This project is licensed under the GNU GPL v3.0 license. See `LICENSE` for more information.
