use std::fmt::Write as _;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write as _;
use std::net::{TcpListener, TcpStream};
use std::str;


const USER_AGENT_HEADER: &str = "User-Agent";


fn handle_connection(mut stream :TcpStream) {
    let buffer = BufReader::new(&mut stream);
    let mut path = String::new();
    let mut user_agent = String::new();
    let mut start = true;

    for line in buffer.lines() {
        let line = line.unwrap();
        if start {
            let words: Vec<&str> = line.split(' ').collect();
            path = String::from(words[1]);
            start = false;
        }

        if line == "" {
            break;
        }

        if !line.contains(": ") {
            continue;
        }

        let (header, value) = match line.split_once(": ") {
            Some((header, value)) => (header, value),
            None => continue
        };

        if header.starts_with(USER_AGENT_HEADER) {
            user_agent = String::from(value);
        }

    } 

    let response = match &path[..] {
        "/" => String::from("HTTP/1.1 200 OK\r\n\r\n"),
        "/user-agent" => write_response(&user_agent),
        path if path.starts_with("/echo") => {
                let parts: Vec<&str> = path.split("/echo/").collect();
                let random_string:&str = parts[parts.len() - 1];
                write_response(random_string)
        },
        _ => String::from("HTTP/1.1 404 Not Found\r\n\r\n")
    };

    stream.write(response.as_bytes()).unwrap();
}

fn write_response(content: &str) -> String {
    let mut lines = String::new();
    write!(&mut lines, "HTTP/1.1 200 OK\r\n").unwrap();
    write!(&mut lines, "Content-Type: text/plain\r\n").unwrap();
    write!(&mut lines, "Content-Length: {}\r\n", content.as_bytes().len()).unwrap();
    write!(&mut lines, "\r\n").unwrap();
    write!(&mut lines, "{content}\r\n").unwrap();
    lines
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");
                std::thread::spawn(move || handle_connection(stream));
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
