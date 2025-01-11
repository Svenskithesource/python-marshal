use python_marshal::load_bytes;

fn main() {
    let data = b"z\x15A string from Python!"; // A simple string marshalled by Python 3.10
    let (obj, _) = load_bytes(data, (3, 10).into()).unwrap(); // Specify the Python version that was used to marshal the data
    println!("{:?}", obj);
}
