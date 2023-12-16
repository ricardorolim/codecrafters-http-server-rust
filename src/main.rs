use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::str;


fn handle_connection(mut stream :TcpStream) {
    let mut buffer = [0; 64];
    stream.read(&mut buffer).unwrap();

    let buffer_str = str::from_utf8(&buffer).unwrap();

    let mut lines = buffer_str.lines();
    let first_line = lines.next().unwrap();
    let words: Vec<&str> = first_line.split(' ').collect();
    let response = match words[1] {
        "/" => "HTTP/1.1 200 OK\r\n\r\n",
        _ => "HTTP/1.1 404 Not Found\r\n\r\n"
    };

    stream.write(response.as_bytes()).unwrap();
}

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");
                handle_connection(stream);
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
