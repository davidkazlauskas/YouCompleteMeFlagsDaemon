#![allow(non_snake_case)]

//use std::sync::mspc::channel;
use std::thread;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};

struct Command {
    dir: String,
    command: String,
    file: String,
}

fn handleStream(mut stream: TcpStream) {
    println!("DOIN IT");
    let mut theStr = String::with_capacity(1024 * 2);
    stream.read_to_string(&mut theStr);
    println!("{}",theStr);
}

fn parseCommands(string: &String) -> Vec<Command> {
    vec!()
}

fn hitTheFile(filepath: String,projectName: String) {
    use std::io::prelude::*;
    use std::fs::File;

    let mut f = File::create(filepath).unwrap();
    let mut contents = String::with_capacity(1024 * 64);
    f.read_to_string(&mut contents);
    parseCommands(&contents);
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
    let cmd = "/home/deividas/Desktop/ramdisk/bld/compile_commands.json".to_string();
    hitTheFile(cmd,"moo".to_string());
}
