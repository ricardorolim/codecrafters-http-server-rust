use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::str;
use std::fmt::Write as _;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::ErrorKind;
// use std::io::Read as _;
use std::io::Write as _;
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::io::Read;


const HDR_CONTENT_LENGTH: &str = "Content-Length";
const HDR_USER_AGENT: &str = "User-Agent";
const CONTENT_TYPE_TEXT: &str = "text/plain";
const CONTENT_TYPE_FILE: &str = "application/octet-stream";


struct HttpRequest {
    method: String,
    uri: String,
    headers: HashMap<String, String>,
    reader: Option<Box<dyn Read>>,
}

impl HttpRequest {
    fn new(method: &str, uri: &str, reader: Option<Box<dyn Read>>) -> HttpRequest {
        HttpRequest {
            method: method.to_string(),
            uri: uri.to_string(), 
            headers: HashMap::new(),
            reader,
        }
    }
}

fn parse_request<T: Read + 'static>(reader: T) -> HttpRequest {
    let mut buffer = BufReader::new(reader);
    let mut line = String::new();
    buffer.read_line(&mut line).unwrap();
    let parts: Vec<&str> = line.split(" ").collect();
    let mut request = HttpRequest::new(parts[0], parts[1], None);
    line.clear();

    while buffer.read_line(&mut line).is_ok() {
        if line == "\r\n" {
            if let Some(length) = request.headers.get(HDR_CONTENT_LENGTH) {
                let length: u64 = length.parse().expect("Unable to parse length header");
                request.reader = Some(Box::new(buffer.take(length)));
            }
            break;
        }

        if line.contains(": ") {
            let (header, value) = match line.trim().split_once(": ") {
                Some((header, value)) => (header, value),
                None => continue
            };

            request.headers.insert(header.to_string(), value.to_string());
        }

        line.clear();
    } 

    request
}


fn handle_connection(mut stream :TcpStream, directory: Option<String>) {
    let reader = stream.try_clone().expect("cloning of tcp stream failed");
    let request = parse_request(reader);

    let response = match request.method.as_str() {
        "GET" => handle_get(&request, &directory),
        "POST" => handle_post(request, &directory),
        method => {
            println!("method not implemented: {}", method);
            write_status(503, "Server Internal Error")
        }
    };

    stream.write(&response).expect("Failure to write response");
}

fn handle_get(request: &HttpRequest, directory: &Option<String>) -> Vec<u8> {
    let user_agent = &request.headers.get(HDR_USER_AGENT);
    let mut text = String::new();
    let mut buffer = Vec::new();

    match request.uri.as_str() {
        "/" => write_status(200, "OK"),
        "/user-agent" => {
            let empty_str = String::new();
            write_text(&mut text, user_agent.unwrap_or_else(|| &empty_str));
            text.as_bytes().to_vec()
        },
        path if request.uri.starts_with("/echo/") => {
            let parts: Vec<&str> = path.split("/echo/").collect();
            let random_string:&str = parts[1];
            write_text(&mut text, random_string);
            text.as_bytes().to_vec()
        },
        path if request.uri.starts_with("/files/") => {
            let fspath = parse_files_path(path, directory);
            let mut filedata = Vec::new();

            match read_file(&mut filedata, &fspath) {
                Ok(_) => {
                    write_file(&mut buffer, &mut filedata);
                    buffer
                },
                Err(error) => match error.kind() {
                    ErrorKind::NotFound => write_status(404, "Not Found"),
                    other_error => {
                        write_status(500, "Internal Server Error");
                        panic!("Problem opening file {:?}", other_error)
                    }
                }
            }
        },
        _ => write_status(404, "Not Found")
    }
}

fn handle_post(request: HttpRequest, directory: &Option<String>) -> Vec<u8> {
    match &request.uri {
        uri if request.uri.starts_with("/files/") => {
            let path = parse_files_path(uri, directory);

            match save_file(&mut request.reader.expect("no reader for request"), &path) {
                Ok(_) => {
                    write_status(201, "Created")
                },
                Err(error) => match error.kind() {
                    ErrorKind::NotFound => write_status(404, "Not Found"),
                    other_error => {
                        write_status(500, "Internal Server Error");
                        panic!("Error writing file to disk: {:?}", other_error)
                    }
                }
            }
        },
        _ => write_status(500, "Internal Server Error")
    }
}

fn parse_files_path(uri: &str, directory: &Option<String>) -> String {
    let parts: Vec<&str> = uri.split("/files/").collect();
    let filename = parts[1];
    let dir_str = directory.clone().unwrap_or_else(|| ".".to_string());
    let path = Path::new(&dir_str).join(filename);
    let path = path.to_str().expect("filename is not a valid UTF-8 sequence");
    path.to_string()
}

fn save_file<R>(reader: &mut R, path: &str) -> io::Result<()> 
where R: Read {
    let mut f = File::create(path).unwrap();
    io::copy(reader, &mut f)?;
    Ok(())
}

fn write_status(code: u32, message: &str) -> Vec<u8> {
    let mut buffer = Vec::new();
    write!(buffer, "HTTP/1.1 {code} {message}\r\n\r\n").unwrap();
    buffer
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_parse_get() {
        let req_str = "\
            GET /index.html HTTP/1.1\r\n\
            User-Agent: curl/7.86.0\r\n\r\n";
        let request = parse_request(req_str.as_bytes());

        assert_eq!(request.method, "GET");
        assert_eq!(request.uri, "/index.html");
        assert_eq!(request.headers.get(HDR_USER_AGENT), Some(&"curl/7.86.0".to_string()));
    }

    #[test]
    fn can_parse_post() {
        let req_str = "\
            POST /files/index.html HTTP/1.1\r\n\
            User-Agent: curl/7.86.0\r\n\
            Content-Length: 5\r\n\
            \r\n\
            hello";
        let request = parse_request(req_str.as_bytes());

        assert_eq!(request.method, "POST");
        assert_eq!(request.uri, "/files/index.html");
        assert_eq!(request.headers.get(HDR_USER_AGENT), Some(&"curl/7.86.0".to_string()));

        let mut content = String::new();
        let _ = request.reader.expect("reader not found for request").read_to_string(&mut content);
        assert_eq!(content, "hello");
    }
}
