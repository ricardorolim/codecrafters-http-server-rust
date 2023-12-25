use std::env;
use std::fs;
use std::io;
use std::str;
use std::fmt::Write as _;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::ErrorKind;
use std::io::Read as _;
use std::io::Write as _;
use std::net::{TcpListener, TcpStream};
use std::path::Path;


const USER_AGENT_HEADER: &str = "User-Agent";
const CONTENT_TYPE_TEXT: &str = "text/plain";
const CONTENT_TYPE_FILE: &str = "application/octet-stream";


fn handle_connection(mut stream :TcpStream, directory: Option<String>) {
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

    let empty_path_response = "HTTP/1.1 200 OK\r\n\r\n";
    let not_found_response = "HTTP/1.1 404 Not Found\r\n\r\n";
    let mut text = String::new();
    let mut buffer = Vec::new();

    let response = match &path[..] {
        "/" => empty_path_response.as_bytes(),
        "/user-agent" => {
            write_text(&mut text, &user_agent);
            text.as_bytes()
        },
        path if path.starts_with("/echo/") => {
            let parts: Vec<&str> = path.split("/echo/").collect();
            let random_string:&str = parts[1];
            write_text(&mut text, random_string);
            text.as_bytes()
        },
        path if path.starts_with("/files/") => {
            let parts: Vec<&str> = path.split("/files/").collect();
            let filename = parts[1];

            let dir_str = directory.unwrap_or_else(|| ".".to_string());

            let path = Path::new(&dir_str).join(filename);
            let path = path.to_str().expect("filename is not a valid UTF-8 sequence");

            let mut filedata = Vec::new();

            match read_file(&mut filedata, path) {
                Ok(_) => {
                    write_file(&mut buffer, &mut filedata);
                    &buffer
                },
                Err(error) => match error.kind() {
                    ErrorKind::NotFound => not_found_response.as_bytes(),
                    other_error => {panic!("Problem opening file {:?}", other_error)}
                }
            }
        },
        _ => not_found_response.as_bytes()
    };

    stream.write(response).expect("Failure to write response");
}

fn read_file(buffer: &mut Vec<u8>, filename: &str) -> io::Result<()> {
    let metadata = fs::metadata(filename)?;

    if metadata.is_dir() {
        return Err(ErrorKind::NotFound.into());
    }

    let mut f = File::open(filename)?;
    f.read_to_end(buffer)?;
    Ok(())
}

fn write_text(lines: &mut String, content: &str) {
    write_header(lines, CONTENT_TYPE_TEXT, content.as_bytes().len()).expect("Failure to write response header");
    write!(lines, "{content}\r\n").expect("Failure to write response content");
}

fn write_file(buffer: &mut Vec<u8>, content: &[u8]) {
    let mut lines = String::new();
    write_header(&mut lines, CONTENT_TYPE_FILE, content.len()).expect("Failure to write response header") ;
    buffer.append(&mut lines.into_bytes());
    buffer.append(&mut content.to_vec());
}

fn write_header(lines: &mut String, content_type: &str, content_length: usize) -> std::fmt::Result{
    write!(lines, "HTTP/1.1 200 OK\r\n")?;
    write!(lines, "Content-Type: {content_type}\r\n")?;
    write!(lines, "Content-Length: {}\r\n", content_length)?;
    write!(lines, "\r\n")?;
    Ok(())
}

fn parse_directory() -> Option<String> {
    let args: Vec<String> = env::args().collect();

    match args.len() {
        3 => {
            let arg1 = &args[1];
            let arg2 = &args[2];
            match &arg1[..] {
                "--directory" => Some(arg2.to_string()),
                _ => None
            }
        },
        _ => None
    }
}

fn main() {
    let directory = parse_directory();
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");
                let dir = directory.clone();
                std::thread::spawn(move || handle_connection(stream, dir));
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
