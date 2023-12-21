use std::fmt::Write as _;
use std::io::Write as _;
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::str;


fn handle_connection(mut stream :TcpStream) {
    let mut buffer = [0; 64];
    stream.read(&mut buffer).unwrap();

    let buffer_str = str::from_utf8(&buffer).unwrap();

    let mut lines = buffer_str.lines();
    let first_line = lines.next().unwrap();
    let words: Vec<&str> = first_line.split(' ').collect();
    let response = match words[1] {
        "/" => String::from("HTTP/1.1 200 OK\r\n\r\n"),
        _ => {
            if words[1].starts_with("/echo/") {
                let parts: Vec<&str> = words[1].split("/echo/").collect();
                let random_string:&str = parts[parts.len() - 1];
            
                let mut lines = String::new();
                write!(&mut lines, "HTTP/1.1 200 OK\r\n").unwrap();
                write!(&mut lines, "Content-Type: text/plain\r\n").unwrap();
                write!(&mut lines, "Content-Length: {}\r\n", random_string.as_bytes().len()).unwrap();
                write!(&mut lines, "\r\n").unwrap();
                write!(&mut lines, "{random_string}\r\n").unwrap();

                lines
            } else {
                String::from("HTTP/1.1 404 Not Found\r\n\r\n")
            }
        }
    };

    stream.write(response.as_bytes()).unwrap();
}

fn main() {
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
