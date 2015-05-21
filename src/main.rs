#![allow(non_snake_case)]

//use std::sync::mspc::channel;
use std::thread;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};

fn handleStream(mut stream: TcpStream) {
    println!("DOIN IT");
    let mut theStr = String::with_capacity(1024 * 2);
    stream.read_to_string(&mut theStr);
    println!("{}",theStr);
}

fn hitTheFile(filepath: String,projectName: String) {

}

fn listen() {
    let listener = TcpListener::bind("127.0.0.1:7777").unwrap();
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handleStream(stream);
            }
            Err(e) => {
                println!("{}",e);
            }
        }
    }
}

fn main() {
    hitTheFile("moo".to_string(),"moo".to_string());
}
